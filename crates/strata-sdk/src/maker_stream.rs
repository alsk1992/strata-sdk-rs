use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::account_stream::validate_account_state;
use super::market_stream::platform_websocket_url;
use super::*;

pub const MAKER_STREAM_AUTH_DOMAIN: &str = "strata:mm-fills-stream:v2";
const AUTH_TIMEOUT: Duration = Duration::from_secs(10);

type PlatformSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// One externally authenticated owner-only maker stream. The SDK retains no
/// signer and fails closed on any identity or sequence mismatch.
pub struct MakerStream {
    market_id: String,
    wallet_address: String,
    stream_id: String,
    sequence: u64,
    initial_snapshot: Option<PlatformMakerEvent>,
    socket: PlatformSocket,
}

impl std::fmt::Debug for MakerStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MakerStream")
            .field("market_id", &self.market_id)
            .field("wallet_address", &self.wallet_address)
            .field("stream_id", &self.stream_id)
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}

impl MakerStream {
    /// `signer` answers the server's compatibility challenge with a signature;
    /// `None` opens the public stream by wallet address (`{"type":"open"}`).
    pub(crate) async fn connect<S: AccountSigner + ?Sized>(
        client: &StrataClient,
        market_id: &str,
        wallet_address: &str,
        signer: Option<&S>,
    ) -> Result<Self, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address = canonical_public_key(wallet_address, "maker wallet address")?;
        let url = platform_websocket_url(
            &client.base_url,
            &format!("v2/markets/{market_id}/makers/{wallet_address}/stream"),
        )?;
        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;

        let challenge_frame = next_text(&mut socket, "maker authentication challenge").await?;
        let challenge_event: PlatformMakerEvent = serde_json::from_str(&challenge_frame)
            .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
        let challenge = match challenge_event {
            PlatformMakerEvent::AuthChallenge {
                schema_version,
                contract_version,
                market_id: response_market,
                wallet_address: response_wallet,
                challenge,
                server_time_ms,
                expires_at_ms,
            } => {
                validate_platform_version(schema_version, &contract_version)?;
                if response_market != market_id
                    || response_wallet != wallet_address
                    || expires_at_ms <= server_time_ms
                    || challenge.len() != 64
                    || !challenge
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                {
                    return Err(SdkError::InvalidResponse(
                        "maker stream authentication bindings are invalid".to_owned(),
                    ));
                }
                challenge
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "maker stream did not begin with authentication".to_owned(),
                ))
            }
        };

        let answer = match signer {
            Some(signer) => {
                let message = maker_stream_auth_message(&market_id, &wallet_address, &challenge)?;
                let signature = signer
                    .sign_message(&message)
                    .await
                    .map_err(SdkError::Signer)?;
                if signature.len() != 64 {
                    return Err(SdkError::Signer(
                        "maker signer must return a 64-byte Ed25519 signature".to_owned(),
                    ));
                }
                serde_json::json!({
                    "type": "authenticate",
                    "signature": hex::encode(signature),
                })
            }
            // Public read: no signature needed.
            None => serde_json::json!({ "type": "open" }),
        };
        socket
            .send(Message::Text(answer.to_string().into()))
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;

        let snapshot_frame = next_text(&mut socket, "signed maker snapshot").await?;
        let snapshot: PlatformMakerEvent = serde_json::from_str(&snapshot_frame)
            .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
        let (stream_id, sequence) = match &snapshot {
            PlatformMakerEvent::MakerSnapshot {
                schema_version,
                contract_version,
                market_id: response_market,
                wallet_address: response_wallet,
                stream_id,
                sequence,
                status,
                fills,
                ..
            } => {
                validate_maker_identity(
                    *schema_version,
                    contract_version,
                    response_market,
                    response_wallet,
                    &market_id,
                    &wallet_address,
                )?;
                if !valid_handle(stream_id, "maker_stream_") {
                    return Err(SdkError::InvalidResponse(
                        "maker stream identity is invalid".to_owned(),
                    ));
                }
                validate_maker_stream_state(status, fills, &market_id, &wallet_address)?;
                (
                    stream_id.clone(),
                    validate_response_atoms(sequence, "sequence", false)?,
                )
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "maker authentication did not return a signed snapshot".to_owned(),
                ))
            }
        };

        Ok(Self {
            market_id,
            wallet_address,
            stream_id,
            sequence,
            initial_snapshot: Some(snapshot),
            socket,
        })
    }

    pub fn market_id(&self) -> &str {
        &self.market_id
    }

    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }

    /// The first call returns the authenticated snapshot received during
    /// connection. Later calls return sequenced fill, status, heartbeat, or
    /// recovery-snapshot events.
    pub async fn next_event(&mut self) -> Result<Option<PlatformMakerEvent>, SdkError> {
        if let Some(snapshot) = self.initial_snapshot.take() {
            return Ok(Some(snapshot));
        }
        loop {
            let Some(frame) = self.socket.next().await else {
                return Ok(None);
            };
            let frame = frame.map_err(|error| SdkError::Stream(error.to_string()))?;
            match frame {
                Message::Text(text) => {
                    let event: PlatformMakerEvent = serde_json::from_str(&text)
                        .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
                    if let Err(error) = self.validate_event(&event) {
                        let _ = self.socket.close(None).await;
                        return Err(error);
                    }
                    return Ok(Some(event));
                }
                Message::Ping(payload) => {
                    self.socket
                        .send(Message::Pong(payload))
                        .await
                        .map_err(|error| SdkError::Stream(error.to_string()))?;
                }
                Message::Pong(_) => {}
                Message::Close(_) => return Ok(None),
                _ => {
                    let _ = self.socket.close(None).await;
                    return Err(SdkError::InvalidResponse(
                        "maker stream sent a non-text data frame".to_owned(),
                    ));
                }
            }
        }
    }

    pub async fn close(&mut self) -> Result<(), SdkError> {
        self.socket
            .close(None)
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))
    }

    fn validate_event(&mut self, event: &PlatformMakerEvent) -> Result<(), SdkError> {
        match event {
            PlatformMakerEvent::AuthChallenge { .. } => {
                return Err(SdkError::InvalidResponse(
                    "maker stream challenged after state delivery".to_owned(),
                ))
            }
            PlatformMakerEvent::MakerSnapshot {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                status,
                fills,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                let next = validate_response_atoms(sequence, "sequence", false)?;
                if stream_id != &self.stream_id || next <= self.sequence {
                    return Err(SdkError::InvalidResponse(
                        "maker recovery snapshot did not advance its sequence".to_owned(),
                    ));
                }
                validate_maker_stream_state(status, fills, &self.market_id, &self.wallet_address)?;
                self.sequence = next;
            }
            PlatformMakerEvent::MakerFill {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                previous_sequence,
                fill,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
                validate_maker_fills(std::slice::from_ref(fill))?;
            }
            PlatformMakerEvent::MakerStatus {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                previous_sequence,
                status,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
                validate_maker_stream_state(status, &[], &self.market_id, &self.wallet_address)?;
            }
            PlatformMakerEvent::Heartbeat {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                previous_sequence,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
            }
        }
        Ok(())
    }

    fn validate_identity(
        &self,
        schema_version: u16,
        contract_version: &str,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<(), SdkError> {
        validate_maker_identity(
            schema_version,
            contract_version,
            market_id,
            wallet_address,
            &self.market_id,
            &self.wallet_address,
        )
    }

    fn validate_sequence(
        &mut self,
        stream_id: &str,
        sequence: &str,
        previous_sequence: &str,
    ) -> Result<(), SdkError> {
        let next = validate_response_atoms(sequence, "sequence", false)?;
        let previous = validate_response_atoms(previous_sequence, "previous_sequence", false)?;
        if stream_id != self.stream_id
            || previous != self.sequence
            || next != previous.saturating_add(1)
        {
            return Err(SdkError::InvalidResponse(
                "maker stream sequence gap detected".to_owned(),
            ));
        }
        self.sequence = next;
        Ok(())
    }
}

async fn next_text(socket: &mut PlatformSocket, expected: &str) -> Result<String, SdkError> {
    let frame = tokio::time::timeout(AUTH_TIMEOUT, socket.next())
        .await
        .map_err(|_| SdkError::Stream(format!("{expected} timed out")))?
        .ok_or_else(|| SdkError::Stream(format!("socket closed before {expected}")))?
        .map_err(|error| SdkError::Stream(error.to_string()))?;
    let Message::Text(text) = frame else {
        return Err(SdkError::InvalidResponse(format!(
            "expected a text {expected}"
        )));
    };
    Ok(text.to_string())
}

fn validate_maker_identity(
    schema_version: u16,
    contract_version: &str,
    actual_market: &str,
    actual_wallet: &str,
    expected_market: &str,
    expected_wallet: &str,
) -> Result<(), SdkError> {
    validate_platform_market_response(
        schema_version,
        contract_version,
        actual_market,
        expected_market,
    )?;
    if actual_wallet != expected_wallet {
        return Err(SdkError::InvalidResponse(
            "maker stream wallet does not match signed request".to_owned(),
        ));
    }
    Ok(())
}

/// The status inside a stream event must be the same owner-scoped projection
/// the HTTP read returns, bound to the same market and wallet.
fn validate_maker_stream_state(
    status: &PlatformMakerStatusResponse,
    fills: &[PlatformMakerFill],
    market_id: &str,
    wallet_address: &str,
) -> Result<(), SdkError> {
    validate_platform_market_response(
        status.schema_version,
        &status.contract_version,
        &status.market_id,
        market_id,
    )?;
    if status.wallet_address != wallet_address {
        return Err(SdkError::InvalidResponse(
            "maker stream status wallet does not match the stream".to_owned(),
        ));
    }
    validate_maker_status(status)?;
    validate_maker_fills(fills)
}

pub(crate) fn validate_maker_fills(fills: &[PlatformMakerFill]) -> Result<(), SdkError> {
    if fills.len() > 2_000 {
        return Err(SdkError::InvalidResponse(
            "maker stream fills exceed the bounded size".to_owned(),
        ));
    }
    let projected: Vec<PlatformAccountFill> = fills
        .iter()
        .map(|fill| PlatformAccountFill {
            fill_id: fill.fill_id.clone(),
            side: fill.side,
            price_atoms: fill.price_atoms.clone(),
            size_atoms: fill.size_atoms.clone(),
            fee_quote_atoms: fill.fee_quote_atoms.clone(),
            fee_is_final: fill.fee_is_final,
            settlement: fill.settlement,
            executed_at_ms: fill.executed_at_ms,
            confirmed_at_ms: fill.confirmed_at_ms,
            transaction_id: fill.transaction_id.clone(),
            realized_pnl_quote_atoms: fill.realized_pnl_quote_atoms.clone(),
        })
        .collect();
    validate_account_state(&[], &projected)
        .map_err(|_| SdkError::InvalidResponse("maker stream contains an invalid fill".to_owned()))
}

pub fn maker_stream_auth_message(
    market_id: &str,
    wallet_address: &str,
    challenge: &str,
) -> Result<Vec<u8>, SdkError> {
    let market_id = validate_platform_market_id(market_id)?;
    let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
    if challenge.len() != 64
        || !challenge
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SdkError::InvalidRequest(
            "maker stream challenge must be 32-byte lowercase hexadecimal".to_owned(),
        ));
    }
    Ok(
        format!("{MAKER_STREAM_AUTH_DOMAIN}\n{market_id}\n{wallet_address}\n{challenge}")
            .into_bytes(),
    )
}
