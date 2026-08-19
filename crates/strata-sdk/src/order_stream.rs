use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

use super::*;

pub const ORDER_STREAM_AUTH_DOMAIN: &str = "strata:order-command-stream:v2";
const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const EVENT_BUFFER: usize = 1_024;
const MAX_CLIENT_COMMANDS_PER_FRAME: usize = 64;
const MAX_SERVER_EVENTS_PER_FRAME: usize = 64;

#[derive(Clone, Debug)]
pub struct OrderChallengeResult {
    pub self_trade_prevention: PlatformSelfTradePrevention,
    pub prevented_order_ids: Vec<String>,
    pub effective_request: PlatformOrderChallengeRequest,
    pub response: PlatformOrderChallengeResponse,
}

struct ActorRequest {
    command: PlatformOrderCommand,
    response: oneshot::Sender<Result<PlatformOrderCommandEvent, SdkError>>,
}

/// Cloneable handle to one authenticated persistent order-command socket.
/// Commands from clones are sequenced by a single writer task, so concurrent
/// strategies cannot produce sequence gaps on the wire.
#[derive(Clone)]
pub struct OrderCommandStream {
    market_id: String,
    owner_wallet: String,
    session_public_key: String,
    commands: mpsc::Sender<ActorRequest>,
    events: broadcast::Sender<PlatformOrderCommandEvent>,
}

impl std::fmt::Debug for OrderCommandStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OrderCommandStream")
            .field("market_id", &self.market_id)
            .field("owner_wallet", &self.owner_wallet)
            .field("session_public_key", &self.session_public_key)
            .finish_non_exhaustive()
    }
}

impl OrderCommandStream {
    pub(crate) async fn connect<S: SessionSigner + ?Sized>(
        client: &StrataClient,
        market_id: &str,
        owner_wallet: &str,
        signer: &S,
    ) -> Result<Self, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        let owner_wallet = canonical_public_key(owner_wallet, "owner_wallet")?;
        let session_public_key = canonical_public_key(signer.public_key(), "session_public_key")?;
        if owner_wallet == session_public_key {
            return Err(SdkError::InvalidRequest(
                "session_public_key must be distinct from owner_wallet".to_owned(),
            ));
        }
        let mut url = client.base_url.clone();
        let scheme = match url.scheme() {
            "https" => "wss",
            "http" => "ws",
            _ => {
                return Err(SdkError::InvalidBaseUrl(
                    "order command URL must use http or https".to_owned(),
                ))
            }
        };
        url.set_scheme(scheme).map_err(|_| {
            SdkError::InvalidBaseUrl("could not select WebSocket scheme".to_owned())
        })?;
        let base_path = url.path().trim_end_matches('/');
        url.set_path(&format!("{base_path}/v2/markets/{market_id}/orders/stream"));
        url.set_query(None);
        url.set_fragment(None);

        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        let auth = tokio::time::timeout(COMMAND_TIMEOUT, socket.next())
            .await
            .map_err(|_| SdkError::Stream("authentication challenge timed out".to_owned()))?
            .ok_or_else(|| SdkError::Stream("socket closed before authentication".to_owned()))?
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        let Message::Text(auth) = auth else {
            return Err(SdkError::Stream(
                "expected a text authentication challenge".to_owned(),
            ));
        };
        let event: PlatformOrderCommandEvent = serde_json::from_str(&auth)
            .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
        let (challenge, server_time_ms, expires_at_ms) = match event {
            PlatformOrderCommandEvent::AuthChallenge {
                schema_version,
                contract_version,
                market_id: response_market,
                challenge,
                server_time_ms,
                expires_at_ms,
            } => {
                validate_platform_version(schema_version, &contract_version)?;
                if response_market != market_id
                    || challenge.len() != 64
                    || !challenge.bytes().all(|byte| byte.is_ascii_hexdigit())
                    || challenge.bytes().any(|byte| byte.is_ascii_uppercase())
                {
                    return Err(SdkError::InvalidResponse(
                        "order stream authentication bindings are invalid".to_owned(),
                    ));
                }
                (challenge, server_time_ms, expires_at_ms)
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "order stream did not begin with authentication".to_owned(),
                ))
            }
        };
        if expires_at_ms <= server_time_ms {
            return Err(SdkError::InvalidResponse(
                "order stream authentication challenge is expired".to_owned(),
            ));
        }
        let auth_message =
            order_stream_auth_message(&market_id, &owner_wallet, &session_public_key, &challenge);
        let signature = signer
            .sign_message(&auth_message)
            .await
            .map_err(SdkError::Signer)?;
        if signature.len() != 64 {
            return Err(SdkError::Signer(
                "stream authentication signature must contain 64 bytes".to_owned(),
            ));
        }
        socket
            .send(Message::Text(
                serde_json::to_string(&PlatformOrderCommandClientFrame::Authenticate {
                    owner_wallet: owner_wallet.clone(),
                    session_public_key: session_public_key.clone(),
                    signature: bs58::encode(signature).into_string(),
                    batch_format: Some(PlatformOrderCommandBatchFormat::CompactV1),
                })
                .map_err(|error| SdkError::InvalidRequest(error.to_string()))?
                .into(),
            ))
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;

        let ready = tokio::time::timeout(COMMAND_TIMEOUT, socket.next())
            .await
            .map_err(|_| SdkError::Stream("signed authentication timed out".to_owned()))?
            .ok_or_else(|| SdkError::Stream("socket closed during authentication".to_owned()))?
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        let Message::Text(ready) = ready else {
            return Err(SdkError::Stream(
                "expected a text authentication result".to_owned(),
            ));
        };
        let mut ready_events = parse_order_command_events(&ready)?;
        if ready_events.len() != 1 {
            return Err(SdkError::InvalidResponse(
                "order stream authentication returned an invalid event batch".to_owned(),
            ));
        }
        let ready = ready_events.pop().expect("one ready event");
        let (stream_id, sequence) = match &ready {
            PlatformOrderCommandEvent::Ready {
                schema_version,
                contract_version,
                market_id: response_market,
                stream_id,
                sequence,
                ..
            } => {
                validate_platform_version(*schema_version, contract_version)?;
                if response_market != &market_id
                    || !valid_handle(stream_id, "order_command_stream_")
                    || sequence != "1"
                {
                    return Err(SdkError::InvalidResponse(
                        "order stream ready bindings are invalid".to_owned(),
                    ));
                }
                (stream_id.clone(), 1u64)
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "order stream authentication was not accepted".to_owned(),
                ))
            }
        };

        let (commands, receiver) = mpsc::channel(512);
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let _ = events.send(ready);
        tokio::spawn(run_actor(
            socket,
            receiver,
            events.clone(),
            market_id.clone(),
            stream_id,
            sequence,
        ));
        Ok(Self {
            market_id,
            owner_wallet,
            session_public_key,
            commands,
            events,
        })
    }

    pub fn market_id(&self) -> &str {
        &self.market_id
    }

    pub fn owner_wallet(&self) -> &str {
        &self.owner_wallet
    }

    pub fn session_public_key(&self) -> &str {
        &self.session_public_key
    }

    /// Subscribe to heartbeats, correlated command results, and pushed chain
    /// confirmations. Lag is explicit through `broadcast::RecvError::Lagged`.
    pub fn subscribe(&self) -> broadcast::Receiver<PlatformOrderCommandEvent> {
        self.events.subscribe()
    }

    pub async fn command(
        &self,
        command: PlatformOrderCommand,
    ) -> Result<PlatformOrderCommandEvent, SdkError> {
        let (response, receiver) = oneshot::channel();
        self.commands
            .send(ActorRequest { command, response })
            .await
            .map_err(|_| SdkError::Stream("order command socket is closed".to_owned()))?;
        tokio::time::timeout(COMMAND_TIMEOUT, receiver)
            .await
            .map_err(|_| SdkError::Stream("order command timed out".to_owned()))?
            .map_err(|_| SdkError::Stream("order command actor stopped".to_owned()))?
    }

    /// Authenticated non-trading round trip for health and latency measurement.
    pub async fn probe(&self, nonce: &str) -> Result<(), SdkError> {
        if nonce.is_empty()
            || nonce.len() > 64
            || !nonce
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(SdkError::InvalidRequest(
                "order command probe nonce is invalid".to_owned(),
            ));
        }
        match self
            .command(PlatformOrderCommand::Probe {
                nonce: nonce.to_owned(),
            })
            .await?
        {
            PlatformOrderCommandEvent::ProbeResult {
                nonce: returned, ..
            } if returned == nonce => Ok(()),
            _ => Err(SdkError::InvalidResponse(
                "order command probe result is invalid".to_owned(),
            )),
        }
    }

    pub async fn challenge(
        &self,
        request: PlatformOrderChallengeRequest,
        self_trade_prevention: PlatformSelfTradePrevention,
    ) -> Result<OrderChallengeResult, SdkError> {
        let request = normalize_order_challenge_request(request)?;
        self.ensure_request_identity(&request)?;
        match self
            .command(PlatformOrderCommand::Challenge {
                request,
                self_trade_prevention,
            })
            .await?
        {
            PlatformOrderCommandEvent::ChallengeResult {
                self_trade_prevention,
                prevented_order_ids,
                effective_request,
                response,
                ..
            } => {
                self.ensure_request_identity(&effective_request)?;
                validate_challenge_result(&self.market_id, &effective_request, &response)?;
                Ok(OrderChallengeResult {
                    self_trade_prevention,
                    prevented_order_ids,
                    effective_request,
                    response,
                })
            }
            _ => Err(SdkError::InvalidResponse(
                "expected an order challenge result".to_owned(),
            )),
        }
    }

    /// Prepare an order-control transaction over the socket. `Authorized`
    /// hands back a challenge; because this socket already authenticated the
    /// session and the challenge is bound to it, `authorization_signature` may
    /// be `None` (one signature: the session signs only the transaction).
    /// `Direct` sends the operation itself.
    pub async fn prepare(
        &self,
        request: PlatformOrderPrepareRequest,
    ) -> Result<PlatformOrderPrepareResponse, SdkError> {
        let request = match request {
            PlatformOrderPrepareRequest::Authorized(authorization) => {
                PlatformOrderPrepareRequest::Authorized(normalize_order_prepare_authorization(
                    authorization,
                )?)
            }
            PlatformOrderPrepareRequest::Direct(operation) => {
                let operation = normalize_order_challenge_request(operation)?;
                self.ensure_request_identity(&operation)?;
                PlatformOrderPrepareRequest::Direct(operation)
            }
        };
        match self
            .command(PlatformOrderCommand::Prepare {
                request: request.clone(),
            })
            .await?
        {
            PlatformOrderCommandEvent::PrepareResult { response, .. } => {
                validate_prepared(&self.market_id, &response)?;
                if let PlatformOrderPrepareRequest::Direct(operation) = &request {
                    if response.action != order_request_action(operation) {
                        return Err(SdkError::InvalidResponse(
                            "prepared order action does not match request".to_owned(),
                        ));
                    }
                }
                Ok(response)
            }
            _ => Err(SdkError::InvalidResponse(
                "expected an order prepare result".to_owned(),
            )),
        }
    }

    pub async fn submit(
        &self,
        request: PlatformOrderSubmitRequest,
    ) -> Result<PlatformOrderSubmitResponse, SdkError> {
        let request = normalize_submit_request(request)?;
        let expected_control = request.order_control_id.clone();
        match self
            .command(PlatformOrderCommand::Submit { request })
            .await?
        {
            PlatformOrderCommandEvent::SubmitResult { response, .. } => {
                validate_submit(&self.market_id, &expected_control, &response)?;
                Ok(response)
            }
            _ => Err(SdkError::InvalidResponse(
                "expected an order submit result".to_owned(),
            )),
        }
    }

    pub async fn status(
        &self,
        request: PlatformOrderStatusRequest,
    ) -> Result<PlatformOrderStatusResponse, SdkError> {
        let request = normalize_status_request(request)?;
        let expected_control = request.order_control_id.clone();
        match self
            .command(PlatformOrderCommand::Status { request })
            .await?
        {
            PlatformOrderCommandEvent::StatusResult { response, .. } => {
                validate_status(&self.market_id, &expected_control, &response)?;
                Ok(response)
            }
            _ => Err(SdkError::InvalidResponse(
                "expected an order status result".to_owned(),
            )),
        }
    }

    pub async fn execute_order<S, V>(
        &self,
        operation: &OrderExecuteOperation,
        signer: &S,
        verifier: &V,
        idempotency_key: Option<&str>,
        self_trade_prevention: PlatformSelfTradePrevention,
    ) -> Result<PlatformOrderSubmitResponse, SdkError>
    where
        S: SessionSigner + ?Sized,
        V: OrderVerifier + ?Sized,
    {
        self.ensure_signer(signer)?;
        let challenged = self
            .challenge(
                operation.challenge_request(self.session_public_key.clone()),
                self_trade_prevention,
            )
            .await?;
        let (prepared, signed_transaction_base64) = self
            .authorize_and_prepare(&challenged, signer, verifier)
            .await?;
        self.submit(PlatformOrderSubmitRequest {
            order_control_id: prepared.order_control_id.clone(),
            signed_transaction_base64,
            idempotency_key: normalize_idempotency_key(
                idempotency_key.unwrap_or(&prepared.order_control_id),
            )?,
        })
        .await
    }

    /// Prepare and arm a fail-closed cancel-all, then maintain its heartbeat
    /// for this transaction's lifetime. For indefinite unattended exposure,
    /// use [`Self::maintain_dead_man`] so the blockhash is refreshed too.
    pub async fn arm_dead_man<S, V>(
        &self,
        timeout: Duration,
        signer: &S,
        verifier: &V,
        idempotency_key: Option<&str>,
    ) -> Result<DeadManGuard, SdkError>
    where
        S: SessionSigner + ?Sized,
        V: OrderVerifier + ?Sized,
    {
        let timeout_ms = checked_dead_man_timeout(timeout)?;
        let (state, _) = self
            .arm_dead_man_once(timeout_ms, signer, verifier, idempotency_key)
            .await?;
        let stream = self.clone();
        let task = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_millis((timeout_ms / 3).max(250)));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;
            loop {
                interval.tick().await;
                match stream.heartbeat_dead_man().await {
                    Ok(state) if state.status == PlatformDeadManStatus::Armed => {}
                    _ => break,
                }
            }
        });
        Ok(DeadManGuard {
            initial_state: state,
            stream: self.clone(),
            task,
            active: true,
        })
    }

    /// Maintain a dead-man indefinitely by heartbeating the current ticket and
    /// externally signing a fresh exact cancel-all before its blockhash expires.
    /// The caller supplies owner-controlled signer/verifier adapters in `Arc`s
    /// solely because the maintenance task must outlive this method call.
    pub async fn maintain_dead_man<S, V>(
        &self,
        timeout: Duration,
        signer: Arc<S>,
        verifier: Arc<V>,
    ) -> Result<DeadManGuard, SdkError>
    where
        S: SessionSigner + 'static,
        V: OrderVerifier + 'static,
    {
        let timeout_ms = checked_dead_man_timeout(timeout)?;
        let (state, mut transaction_expires_at_ms) = self
            .arm_dead_man_once(timeout_ms, signer.as_ref(), verifier.as_ref(), None)
            .await?;
        let stream = self.clone();
        let maintained_stream = stream.clone();
        let task = tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_millis((timeout_ms / 3).max(250)));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await;
            let refresh_lead_ms = timeout_ms.saturating_mul(2).max(5_000);
            loop {
                interval.tick().await;
                let now = unix_ms().unwrap_or(u64::MAX);
                if now.saturating_add(refresh_lead_ms) >= transaction_expires_at_ms {
                    match maintained_stream
                        .arm_dead_man_once(timeout_ms, signer.as_ref(), verifier.as_ref(), None)
                        .await
                    {
                        Ok((next, expires)) if next.status == PlatformDeadManStatus::Armed => {
                            transaction_expires_at_ms = expires;
                        }
                        _ => break,
                    }
                } else {
                    match maintained_stream.heartbeat_dead_man().await {
                        Ok(next) if next.status == PlatformDeadManStatus::Armed => {}
                        _ => break,
                    }
                }
            }
        });
        Ok(DeadManGuard {
            initial_state: state,
            stream,
            task,
            active: true,
        })
    }

    pub async fn heartbeat_dead_man(&self) -> Result<PlatformDeadManState, SdkError> {
        self.dead_man_command(PlatformOrderCommand::DeadManHeartbeat)
            .await
    }

    pub async fn dead_man_status(&self) -> Result<PlatformDeadManState, SdkError> {
        self.dead_man_command(PlatformOrderCommand::DeadManStatus)
            .await
    }

    pub async fn disarm_dead_man(&self) -> Result<PlatformDeadManState, SdkError> {
        self.dead_man_command(PlatformOrderCommand::DeadManDisarm)
            .await
    }

    async fn dead_man_command(
        &self,
        command: PlatformOrderCommand,
    ) -> Result<PlatformDeadManState, SdkError> {
        match self.command(command).await? {
            PlatformOrderCommandEvent::DeadManResult { state, .. } => Ok(state),
            _ => Err(SdkError::InvalidResponse(
                "expected a dead-man result".to_owned(),
            )),
        }
    }

    async fn arm_dead_man_once<S, V>(
        &self,
        timeout_ms: u64,
        signer: &S,
        verifier: &V,
        idempotency_key: Option<&str>,
    ) -> Result<(PlatformDeadManState, u64), SdkError>
    where
        S: SessionSigner + ?Sized,
        V: OrderVerifier + ?Sized,
    {
        self.ensure_signer(signer)?;
        let challenged = self
            .challenge(
                PlatformOrderChallengeRequest::CancelAll {
                    owner_wallet: self.owner_wallet.clone(),
                    session_public_key: self.session_public_key.clone(),
                },
                PlatformSelfTradePrevention::CancelTaker,
            )
            .await?;
        let (prepared, signed_transaction_base64) = self
            .authorize_and_prepare(&challenged, signer, verifier)
            .await?;
        let transaction_expires_at_ms = prepared.expires_at_ms;
        let request = PlatformOrderSubmitRequest {
            order_control_id: prepared.order_control_id.clone(),
            signed_transaction_base64,
            idempotency_key: normalize_idempotency_key(
                idempotency_key.unwrap_or(&prepared.order_control_id),
            )?,
        };
        let state = match self
            .command(PlatformOrderCommand::DeadManArm {
                timeout_ms,
                request,
            })
            .await?
        {
            PlatformOrderCommandEvent::DeadManResult { state, .. } => state,
            _ => {
                return Err(SdkError::InvalidResponse(
                    "expected a dead-man result".to_owned(),
                ))
            }
        };
        if state.status != PlatformDeadManStatus::Armed {
            return Err(SdkError::InvalidResponse(
                "dead-man ticket was not armed".to_owned(),
            ));
        }
        Ok((state, transaction_expires_at_ms))
    }

    /// One signature: this socket already authenticated the session and the
    /// challenge is bound to it, so no message signature is needed — the
    /// challenge's authorization payload is still parsed to bind the prepared
    /// blockhash and order set, then the session signs only the transaction,
    /// after it has been verified.
    async fn authorize_and_prepare<S, V>(
        &self,
        challenged: &OrderChallengeResult,
        signer: &S,
        verifier: &V,
    ) -> Result<(PlatformOrderPrepareResponse, String), SdkError>
    where
        S: SessionSigner + ?Sized,
        V: OrderVerifier + ?Sized,
    {
        let authorization =
            validate_order_authorization(&challenged.response, &challenged.effective_request)?;
        let prepared = self
            .prepare(PlatformOrderPrepareRequest::Authorized(
                PlatformOrderPrepareAuthorization {
                    challenge_id: challenged.response.challenge_id.clone(),
                    authorization_signature: None,
                },
            ))
            .await?;
        validate_order_prepare_binding(&prepared, &challenged.response, &authorization)?;
        verifier
            .verify(&OrderVerificationContext {
                challenge: Some(&challenged.response),
                operation: &challenged.effective_request,
                market_id: &self.market_id,
                prepared: &prepared,
                owner_wallet: &self.owner_wallet,
                session_public_key: &self.session_public_key,
            })
            .await
            .map_err(SdkError::Verification)?;
        let transaction = signer
            .sign_transaction(&prepared.transaction_base64)
            .await
            .map_err(SdkError::Signer)?;
        Ok((
            prepared,
            canonical_base64(&transaction, "signed_transaction_base64")?,
        ))
    }

    fn ensure_request_identity(
        &self,
        request: &PlatformOrderChallengeRequest,
    ) -> Result<(), SdkError> {
        if order_request_owner(request) != self.owner_wallet
            || order_request_session(request) != self.session_public_key
        {
            return Err(SdkError::InvalidRequest(
                "order command identity does not match the authenticated socket".to_owned(),
            ));
        }
        Ok(())
    }

    fn ensure_signer<S: SessionSigner + ?Sized>(&self, signer: &S) -> Result<(), SdkError> {
        if canonical_public_key(signer.public_key(), "session_public_key")?
            != self.session_public_key
        {
            return Err(SdkError::InvalidRequest(
                "signer does not match the authenticated order command session".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Keeps the durable dead-man deadline alive. Drop is deliberately fail-closed:
/// it stops heartbeats and leaves the pre-signed cancel-all armed.
pub struct DeadManGuard {
    pub initial_state: PlatformDeadManState,
    stream: OrderCommandStream,
    task: JoinHandle<()>,
    active: bool,
}

impl DeadManGuard {
    pub async fn disarm(&mut self) -> Result<PlatformDeadManState, SdkError> {
        self.task.abort();
        let state = self.stream.disarm_dead_man().await?;
        self.active = false;
        Ok(state)
    }
}

impl Drop for DeadManGuard {
    fn drop(&mut self) {
        self.task.abort();
        if self.active {
            // Intentionally no disarm. A crashed/dropped agent must fail closed.
        }
    }
}

pub fn order_stream_auth_message(
    market_id: &str,
    owner_wallet: &str,
    session_public_key: &str,
    challenge: &str,
) -> Vec<u8> {
    format!(
        "{ORDER_STREAM_AUTH_DOMAIN}\n{market_id}\n{owner_wallet}\n{session_public_key}\n{challenge}"
    )
    .into_bytes()
}

async fn run_actor<S>(
    mut socket: tokio_tungstenite::WebSocketStream<S>,
    mut commands: mpsc::Receiver<ActorRequest>,
    events: broadcast::Sender<PlatformOrderCommandEvent>,
    market_id: String,
    stream_id: String,
    mut server_sequence: u64,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let mut client_sequence = 0u64;
    let mut request_counter = 0u64;
    let mut pending =
        HashMap::<String, oneshot::Sender<Result<PlatformOrderCommandEvent, SdkError>>>::new();
    let failure = 'actor: loop {
        tokio::select! {
            request = commands.recv() => {
                let Some(request) = request else {
                    let _ = socket.close(None).await;
                    break "order command handle closed".to_owned();
                };
                let mut requests = Vec::with_capacity(MAX_CLIENT_COMMANDS_PER_FRAME);
                requests.push(request);
                // Give concurrently queued callers one scheduler turn to join
                // this transport batch. Each command keeps its own sequence,
                // request ID and response channel.
                tokio::task::yield_now().await;
                while requests.len() < MAX_CLIENT_COMMANDS_PER_FRAME {
                    match commands.try_recv() {
                        Ok(request) => requests.push(request),
                        Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected) => break,
                    }
                }
                let mut frames = Vec::with_capacity(requests.len());
                let mut responses = Vec::with_capacity(requests.len());
                for request in requests {
                    client_sequence = client_sequence.saturating_add(1);
                    request_counter = request_counter.saturating_add(1);
                    let request_id = format!("rust-{request_counter:x}");
                    frames.push(PlatformOrderCommandClientFrame::Command {
                        request_id: request_id.clone(),
                        sequence: client_sequence.to_string(),
                        command: request.command,
                    });
                    responses.push((request_id, request.response));
                }
                let message = match encode_order_command_frames(&frames) {
                    Ok(message) => message,
                    Err(error) => {
                        let text = error.to_string();
                        for (_, response) in responses {
                            let _ = response.send(Err(SdkError::InvalidRequest(text.clone())));
                        }
                        break text;
                    }
                };
                if let Err(error) = socket.send(Message::Text(message.into())).await {
                    let text = error.to_string();
                    for (_, response) in responses {
                        let _ = response.send(Err(SdkError::Stream(text.clone())));
                    }
                    break text;
                }
                for (request_id, response) in responses {
                    pending.insert(request_id, response);
                }
            }
            incoming = socket.next() => {
                let Some(incoming) = incoming else {
                    break "order command socket closed".to_owned();
                };
                let message = match incoming {
                    Ok(Message::Text(message)) => message,
                    Ok(Message::Ping(payload)) => {
                        if let Err(error) = socket.send(Message::Pong(payload)).await {
                            break error.to_string();
                        }
                        continue;
                    }
                    Ok(Message::Pong(_)) => continue,
                    Ok(Message::Close(_)) => break "order command socket closed".to_owned(),
                    Ok(_) => continue,
                    Err(error) => break error.to_string(),
                };
                let frame_events = match parse_order_command_events(&message) {
                    Ok(events) => events,
                    Err(error) => break error.to_string(),
                };
                for event in frame_events {
                    if let Err(error) = validate_event_sequence(
                        &event,
                        &market_id,
                        &stream_id,
                        &mut server_sequence,
                    ) {
                        break 'actor error.to_string();
                    }
                    let request_id = event_request_id(&event).map(str::to_owned);
                    let _ = events.send(event.clone());
                    if let Some(request_id) = request_id {
                        if let Some(response) = pending.remove(&request_id) {
                            let result = match &event {
                                PlatformOrderCommandEvent::CommandError { error, .. } => {
                                    Err(SdkError::Command {
                                        code: serde_json::to_value(error.code)
                                            .ok()
                                            .and_then(|value| value.as_str().map(str::to_owned))
                                            .unwrap_or_else(|| "command_rejected".to_owned()),
                                        message: error.message.clone(),
                                        retryable: error.retryable,
                                    })
                                }
                                _ => Ok(event),
                            };
                            let _ = response.send(result);
                        }
                    }
                }
            }
        }
    };
    for response in pending.into_values() {
        let _ = response.send(Err(SdkError::Stream(failure.clone())));
    }
}

fn encode_order_command_frames(
    frames: &[PlatformOrderCommandClientFrame],
) -> Result<String, serde_json::Error> {
    if frames.len() == 1 {
        serde_json::to_string(&frames[0])
    } else {
        serde_json::to_string(frames)
    }
}

fn parse_order_command_events(message: &str) -> Result<Vec<PlatformOrderCommandEvent>, SdkError> {
    let value: serde_json::Value = serde_json::from_str(message).map_err(|error| {
        SdkError::InvalidResponse(format!("invalid order command frame: {error}"))
    })?;
    let events = if value.is_array() {
        serde_json::from_value::<Vec<PlatformOrderCommandEvent>>(value)
    } else if value.get("type").and_then(serde_json::Value::as_str) == Some("event_batch") {
        return serde_json::from_value::<PlatformOrderCommandServerFrame>(value)
            .map_err(|error| {
                SdkError::InvalidResponse(format!("invalid order command event batch: {error}"))
            })
            .and_then(expand_order_command_event_batch);
    } else {
        serde_json::from_value::<PlatformOrderCommandEvent>(value).map(|event| vec![event])
    }
    .map_err(|error| SdkError::InvalidResponse(format!("invalid order command event: {error}")))?;
    if events.is_empty() || events.len() > MAX_SERVER_EVENTS_PER_FRAME {
        return Err(SdkError::InvalidResponse(
            "order command event batch is invalid".to_owned(),
        ));
    }
    Ok(events)
}

fn expand_order_command_event_batch(
    frame: PlatformOrderCommandServerFrame,
) -> Result<Vec<PlatformOrderCommandEvent>, SdkError> {
    let PlatformOrderCommandServerFrame::EventBatch {
        schema_version,
        contract_version,
        market_id,
        stream_id,
        first_sequence,
        previous_sequence,
        server_time_ms,
        events,
    } = frame;
    if events.is_empty() || events.len() > MAX_SERVER_EVENTS_PER_FRAME {
        return Err(SdkError::InvalidResponse(
            "order command event batch is invalid".to_owned(),
        ));
    }
    let first = parse_wire_sequence(&first_sequence)?;
    let previous = parse_wire_sequence(&previous_sequence)?;
    if first
        != previous.checked_add(1).ok_or_else(|| {
            SdkError::InvalidResponse("order command event batch sequence overflowed".to_owned())
        })?
    {
        return Err(SdkError::InvalidResponse(
            "order command event batch sequence is invalid".to_owned(),
        ));
    }
    events
        .into_iter()
        .enumerate()
        .map(|(index, event)| {
            let sequence = first
                .checked_add(u64::try_from(index).map_err(|_| {
                    SdkError::InvalidResponse("order command event batch is too large".to_owned())
                })?)
                .ok_or_else(|| {
                    SdkError::InvalidResponse(
                        "order command event batch sequence overflowed".to_owned(),
                    )
                })?;
            let previous_sequence = sequence.saturating_sub(1).to_string();
            let sequence = sequence.to_string();
            let common = || {
                (
                    schema_version,
                    contract_version.clone(),
                    market_id.clone(),
                    stream_id.clone(),
                    sequence.clone(),
                    previous_sequence.clone(),
                    server_time_ms,
                )
            };
            Ok(match event {
                PlatformOrderCommandBatchEvent::ProbeResult { request_id, nonce } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::ProbeResult {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        nonce,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::ChallengeResult {
                    request_id,
                    self_trade_prevention,
                    prevented_order_ids,
                    effective_request,
                    response,
                } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::ChallengeResult {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        self_trade_prevention,
                        prevented_order_ids,
                        effective_request,
                        response,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::PrepareResult {
                    request_id,
                    response,
                } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::PrepareResult {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        response,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::SubmitResult {
                    request_id,
                    response,
                } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::SubmitResult {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        response,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::StatusResult {
                    request_id,
                    response,
                } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::StatusResult {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        response,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::DeadManResult { request_id, state } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::DeadManResult {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        state,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::CommandError { request_id, error } => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::CommandError {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        request_id,
                        error,
                        server_time_ms,
                    }
                }
                PlatformOrderCommandBatchEvent::Heartbeat => {
                    let (
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    ) = common();
                    PlatformOrderCommandEvent::Heartbeat {
                        schema_version,
                        contract_version,
                        market_id,
                        stream_id,
                        sequence,
                        previous_sequence,
                        server_time_ms,
                    }
                }
            })
        })
        .collect()
}

fn validate_event_sequence(
    event: &PlatformOrderCommandEvent,
    market_id: &str,
    stream_id: &str,
    server_sequence: &mut u64,
) -> Result<(), SdkError> {
    let Some((schema, contract, market, stream, sequence, previous)) = event_sequence(event) else {
        return Err(SdkError::InvalidResponse(
            "order stream restarted without signed authentication".to_owned(),
        ));
    };
    validate_platform_version(schema, contract)?;
    let sequence = parse_wire_sequence(sequence)?;
    let previous = parse_wire_sequence(previous)?;
    if market != market_id
        || stream != stream_id
        || previous != *server_sequence
        || sequence != server_sequence.saturating_add(1)
    {
        return Err(SdkError::InvalidResponse(
            "order command event sequence is not contiguous".to_owned(),
        ));
    }
    *server_sequence = sequence;
    Ok(())
}

fn event_sequence(
    event: &PlatformOrderCommandEvent,
) -> Option<(u16, &str, &str, &str, &str, &str)> {
    match event {
        PlatformOrderCommandEvent::ProbeResult {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::ChallengeResult {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::PrepareResult {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::SubmitResult {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::StatusResult {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::DeadManResult {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::CommandError {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        }
        | PlatformOrderCommandEvent::Heartbeat {
            schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
            ..
        } => Some((
            *schema_version,
            contract_version,
            market_id,
            stream_id,
            sequence,
            previous_sequence,
        )),
        PlatformOrderCommandEvent::AuthChallenge { .. }
        | PlatformOrderCommandEvent::Ready { .. } => None,
    }
}

fn event_request_id(event: &PlatformOrderCommandEvent) -> Option<&str> {
    match event {
        PlatformOrderCommandEvent::ProbeResult { request_id, .. }
        | PlatformOrderCommandEvent::ChallengeResult { request_id, .. }
        | PlatformOrderCommandEvent::PrepareResult { request_id, .. }
        | PlatformOrderCommandEvent::SubmitResult { request_id, .. }
        | PlatformOrderCommandEvent::StatusResult { request_id, .. }
        | PlatformOrderCommandEvent::DeadManResult { request_id, .. }
        | PlatformOrderCommandEvent::CommandError { request_id, .. } => Some(request_id),
        _ => None,
    }
}

fn parse_wire_sequence(value: &str) -> Result<u64, SdkError> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(SdkError::InvalidResponse(
            "order command sequence is not canonical".to_owned(),
        ));
    }
    value
        .parse()
        .map_err(|_| SdkError::InvalidResponse("order command sequence exceeds u64".to_owned()))
}

fn checked_dead_man_timeout(timeout: Duration) -> Result<u64, SdkError> {
    let timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX);
    if !(1_000..=30_000).contains(&timeout_ms) {
        return Err(SdkError::InvalidRequest(
            "dead-man timeout must be between one and thirty seconds".to_owned(),
        ));
    }
    Ok(timeout_ms)
}

fn validate_challenge_result(
    market_id: &str,
    request: &PlatformOrderChallengeRequest,
    response: &PlatformOrderChallengeResponse,
) -> Result<(), SdkError> {
    validate_platform_version(response.schema_version, &response.contract_version)?;
    if response.market_id != market_id
        || response.action != order_request_action(request)
        || !valid_handle(&response.challenge_id, "oc_")
        || response.order_ids.is_empty()
        || response.order_ids.len() > 12
        || response.expires_at_ms <= response.server_time_ms
        || response
            .order_ids
            .iter()
            .any(|id| !valid_handle(id, "order_"))
    {
        return Err(SdkError::InvalidResponse(
            "order challenge bindings are invalid".to_owned(),
        ));
    }
    canonical_base64(
        &response.authorization_payload_base64,
        "authorization_payload_base64",
    )?;
    Ok(())
}

fn validate_prepared(
    market_id: &str,
    response: &PlatformOrderPrepareResponse,
) -> Result<(), SdkError> {
    validate_platform_version(response.schema_version, &response.contract_version)?;
    if response.market_id != market_id
        || !valid_handle(&response.order_control_id, "or_")
        || response.order_ids.is_empty()
        || response.order_ids.len() > 12
        || response.expires_at_ms == 0
    {
        return Err(SdkError::InvalidResponse(
            "prepared order control is invalid".to_owned(),
        ));
    }
    canonical_base64(&response.transaction_base64, "transaction_base64")?;
    canonical_base58_32(&response.recent_blockhash, "recent_blockhash")?;
    Ok(())
}

fn normalize_submit_request(
    request: PlatformOrderSubmitRequest,
) -> Result<PlatformOrderSubmitRequest, SdkError> {
    if !valid_handle(&request.order_control_id, "or_") {
        return Err(SdkError::InvalidRequest(
            "order_control_id is invalid".to_owned(),
        ));
    }
    Ok(PlatformOrderSubmitRequest {
        order_control_id: request.order_control_id,
        signed_transaction_base64: canonical_base64(
            &request.signed_transaction_base64,
            "signed_transaction_base64",
        )?,
        idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
    })
}

fn normalize_status_request(
    request: PlatformOrderStatusRequest,
) -> Result<PlatformOrderStatusRequest, SdkError> {
    if !valid_handle(&request.order_control_id, "or_") {
        return Err(SdkError::InvalidRequest(
            "order_control_id is invalid".to_owned(),
        ));
    }
    Ok(PlatformOrderStatusRequest {
        order_control_id: request.order_control_id,
        idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
    })
}

fn validate_submit(
    market_id: &str,
    control_id: &str,
    response: &PlatformOrderSubmitResponse,
) -> Result<(), SdkError> {
    validate_platform_version(response.schema_version, &response.contract_version)?;
    if response.market_id != market_id
        || response.order_control_id != control_id
        || response.status != PlatformOrderSubmissionStatus::Submitted
    {
        return Err(SdkError::InvalidResponse(
            "order submit bindings are invalid".to_owned(),
        ));
    }
    canonical_signature(&response.signature, "signature")?;
    Ok(())
}

fn validate_status(
    market_id: &str,
    control_id: &str,
    response: &PlatformOrderStatusResponse,
) -> Result<(), SdkError> {
    validate_platform_version(response.schema_version, &response.contract_version)?;
    if response.market_id != market_id
        || response.order_control_id != control_id
        || response.order_ids.is_empty()
        || response.order_ids.len() > 12
        || response
            .order_ids
            .iter()
            .any(|id| !valid_handle(id, "order_"))
        || (response.status == PlatformOrderControlStatus::Failed)
            != response
                .failure_code
                .as_deref()
                .is_some_and(|code| !code.is_empty())
    {
        return Err(SdkError::InvalidResponse(
            "order status bindings are invalid".to_owned(),
        ));
    }
    canonical_signature(&response.signature, "signature")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_domain_binds_every_identity() {
        let message = String::from_utf8(order_stream_auth_message(
            "market_11111111111111111111111111111111",
            "owner",
            "session",
            &"ab".repeat(32),
        ))
        .unwrap();
        assert_eq!(
            message,
            format!(
                "{ORDER_STREAM_AUTH_DOMAIN}\nmarket_11111111111111111111111111111111\nowner\nsession\n{}",
                "ab".repeat(32)
            )
        );
    }

    #[test]
    fn canonical_sequence_rejects_gaps_and_leading_zeroes() {
        assert_eq!(parse_wire_sequence("8").unwrap(), 8);
        assert!(parse_wire_sequence("08").is_err());
        assert!(parse_wire_sequence("-1").is_err());
    }

    #[test]
    fn parses_ordered_event_batches_and_single_frame_fallback() {
        let event = serde_json::json!({
            "type": "heartbeat",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": "market_11111111111111111111111111111111",
            "stream_id": "order_command_stream_11111111111111111111111111111111",
            "sequence": "2",
            "previous_sequence": "1",
            "server_time_ms": 1_786_810_000_000u64
        });
        let single = parse_order_command_events(&event.to_string()).unwrap();
        assert_eq!(single.len(), 1);
        let batch = parse_order_command_events(
            &serde_json::Value::Array(vec![event.clone(), event]).to_string(),
        )
        .unwrap();
        assert_eq!(batch.len(), 2);
        let compact = serde_json::json!({
            "type": "event_batch",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": "market_11111111111111111111111111111111",
            "stream_id": "order_command_stream_11111111111111111111111111111111",
            "first_sequence": "2",
            "previous_sequence": "1",
            "server_time_ms": 1_786_810_000_000u64,
            "events": [
                {
                    "type": "probe_result",
                    "request_id": "probe-1",
                    "nonce": "health-1"
                },
                {"type": "heartbeat"}
            ]
        });
        let compact = parse_order_command_events(&compact.to_string()).unwrap();
        assert_eq!(compact.len(), 2);
        assert!(matches!(
            &compact[0],
            PlatformOrderCommandEvent::ProbeResult {
                sequence,
                previous_sequence,
                request_id,
                nonce,
                ..
            } if sequence == "2"
                && previous_sequence == "1"
                && request_id == "probe-1"
                && nonce == "health-1"
        ));
        assert!(matches!(
            &compact[1],
            PlatformOrderCommandEvent::Heartbeat { sequence, previous_sequence, .. }
                if sequence == "3" && previous_sequence == "2"
        ));
        assert!(parse_order_command_events("[]").is_err());
    }

    #[test]
    fn encodes_single_commands_compatibly_and_concurrent_commands_as_one_batch() {
        let frame = |request_id: &str, sequence: &str| PlatformOrderCommandClientFrame::Command {
            request_id: request_id.to_owned(),
            sequence: sequence.to_owned(),
            command: PlatformOrderCommand::DeadManStatus,
        };
        let singleton = encode_order_command_frames(&[frame("one", "1")]).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&singleton)
            .unwrap()
            .is_object());
        let batch = encode_order_command_frames(&[frame("two", "2"), frame("three", "3")]).unwrap();
        let decoded = serde_json::from_str::<Vec<PlatformOrderCommandClientFrame>>(&batch).unwrap();
        assert_eq!(decoded.len(), 2);
    }
}
