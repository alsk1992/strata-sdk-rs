use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::market_stream::platform_websocket_url;
use super::*;

pub const ACCOUNT_STREAM_AUTH_DOMAIN: &str = "strata:account-stream:v2";
const AUTH_TIMEOUT: Duration = Duration::from_secs(10);

type PlatformSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// One externally authenticated private account stream. The SDK retains no
/// signer and fails closed on any identity or sequence mismatch.
pub struct AccountStream {
    market_id: String,
    wallet_address: String,
    stream_id: String,
    sequence: u64,
    initial_snapshot: Option<PlatformAccountEvent>,
    socket: PlatformSocket,
}

impl std::fmt::Debug for AccountStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AccountStream")
            .field("market_id", &self.market_id)
            .field("wallet_address", &self.wallet_address)
            .field("stream_id", &self.stream_id)
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}

impl AccountStream {
    pub(crate) async fn connect<S: AccountSigner + ?Sized>(
        client: &StrataClient,
        market_id: &str,
        signer: &S,
    ) -> Result<Self, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address =
            canonical_public_key(signer.public_key(), "account signer public key")?;
        let url = platform_websocket_url(
            &client.base_url,
            &format!("v2/markets/{market_id}/account/{wallet_address}/stream"),
        )?;
        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;

        let challenge_frame = next_text(&mut socket, "account authentication challenge").await?;
        let challenge_event: PlatformAccountEvent = serde_json::from_str(&challenge_frame)
            .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
        let challenge = match challenge_event {
            PlatformAccountEvent::AuthChallenge {
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
                        "account stream authentication bindings are invalid".to_owned(),
                    ));
                }
                challenge
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "account stream did not begin with authentication".to_owned(),
                ))
            }
        };

        let message = account_stream_auth_message(&market_id, &wallet_address, &challenge)?;
        let signature = signer
            .sign_message(&message)
            .await
            .map_err(SdkError::Signer)?;
        if signature.len() != 64 {
            return Err(SdkError::Signer(
                "account signer must return a 64-byte Ed25519 signature".to_owned(),
            ));
        }
        socket
            .send(Message::Text(
                serde_json::json!({
                    "type": "authenticate",
                    "signature": hex::encode(signature),
                })
                .to_string()
                .into(),
            ))
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;

        let snapshot_frame = next_text(&mut socket, "signed account snapshot").await?;
        let snapshot: PlatformAccountEvent = serde_json::from_str(&snapshot_frame)
            .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
        let (stream_id, sequence) = match &snapshot {
            PlatformAccountEvent::AccountSnapshot {
                schema_version,
                contract_version,
                market_id: response_market,
                wallet_address: response_wallet,
                stream_id,
                sequence,
                orders,
                fills,
                ..
            } => {
                validate_account_identity(
                    schema_version,
                    contract_version,
                    response_market,
                    response_wallet,
                    &market_id,
                    &wallet_address,
                )?;
                if !valid_handle(stream_id, "account_stream_") {
                    return Err(SdkError::InvalidResponse(
                        "account stream identity is invalid".to_owned(),
                    ));
                }
                validate_account_state(orders, fills)?;
                (
                    stream_id.clone(),
                    validate_response_atoms(sequence, "sequence", false)?,
                )
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "account authentication did not return a signed snapshot".to_owned(),
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
    /// connection. Later calls return sequenced order, fill, heartbeat, or
    /// recovery-snapshot events.
    pub async fn next_event(&mut self) -> Result<Option<PlatformAccountEvent>, SdkError> {
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
                    let event: PlatformAccountEvent = serde_json::from_str(&text)
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
                        "account stream sent a non-text data frame".to_owned(),
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

    fn validate_event(&mut self, event: &PlatformAccountEvent) -> Result<(), SdkError> {
        match event {
            PlatformAccountEvent::AuthChallenge { .. } => {
                return Err(SdkError::InvalidResponse(
                    "account stream challenged after state delivery".to_owned(),
                ))
            }
            PlatformAccountEvent::AccountSnapshot {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                orders,
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
                        "account recovery snapshot did not advance its sequence".to_owned(),
                    ));
                }
                validate_account_state(orders, fills)?;
                self.sequence = next;
            }
            PlatformAccountEvent::OrdersSnapshot {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                previous_sequence,
                orders,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
                validate_account_state(orders, &[])?;
            }
            PlatformAccountEvent::Fill {
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
                validate_account_state(&[], std::slice::from_ref(fill))?;
            }
            PlatformAccountEvent::Heartbeat {
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
        validate_account_identity(
            &schema_version,
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
                "account stream sequence gap detected".to_owned(),
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

fn validate_account_identity(
    schema_version: &u16,
    contract_version: &str,
    actual_market: &str,
    actual_wallet: &str,
    expected_market: &str,
    expected_wallet: &str,
) -> Result<(), SdkError> {
    validate_platform_market_response(
        *schema_version,
        contract_version,
        actual_market,
        expected_market,
    )?;
    if actual_wallet != expected_wallet {
        return Err(SdkError::InvalidResponse(
            "account stream wallet does not match signed request".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_account_state(
    orders: &[PlatformAccountOrder],
    fills: &[PlatformAccountFill],
) -> Result<(), SdkError> {
    let mut order_ids = HashSet::new();
    if orders.iter().any(|order| {
        let original =
            validate_response_atoms(&order.original_size_atoms, "original_size_atoms", false);
        let remaining =
            validate_response_atoms(&order.remaining_size_atoms, "remaining_size_atoms", false);
        let state_matches = match (original, remaining) {
            (Ok(original), Ok(remaining)) if remaining <= original => {
                order.state
                    == if remaining == original {
                        PlatformOrderState::Open
                    } else {
                        PlatformOrderState::PartiallyFilled
                    }
            }
            _ => false,
        };
        !valid_handle(&order.order_id, "order_")
            || !order_ids.insert(order.order_id.as_str())
            || validate_response_atoms(&order.limit_price_atoms, "limit_price_atoms", false)
                .is_err()
            || !state_matches
    }) {
        return Err(SdkError::InvalidResponse(
            "account stream contains an invalid order".to_owned(),
        ));
    }
    let mut fill_ids = HashSet::new();
    if fills.iter().any(|fill| {
        let valid_confirmation = fill
            .confirmed_at_ms
            .is_none_or(|confirmed| confirmed >= fill.executed_at_ms)
            && (fill.settlement != PlatformSettlementState::Confirmed
                || fill.confirmed_at_ms.is_some());
        let valid_transaction = fill.transaction_id.as_deref().is_none_or(|transaction| {
            (32..=100).contains(&transaction.len())
                && transaction.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() && !matches!(byte, b'0' | b'O' | b'I' | b'l')
                })
        });
        !valid_handle(&fill.fill_id, "fill_")
            || !fill_ids.insert(fill.fill_id.as_str())
            || validate_response_atoms(&fill.price_atoms, "price_atoms", false).is_err()
            || validate_response_atoms(&fill.size_atoms, "size_atoms", false).is_err()
            || validate_response_atoms(&fill.fee_quote_atoms, "fee_quote_atoms", true).is_err()
            || validate_signed_response_atoms(
                &fill.realized_pnl_quote_atoms,
                "realized_pnl_quote_atoms",
            )
            .is_err()
            || !valid_confirmation
            || !valid_transaction
    }) {
        return Err(SdkError::InvalidResponse(
            "account stream contains an invalid fill".to_owned(),
        ));
    }
    Ok(())
}

fn validate_signed_response_atoms(value: &str, field: &str) -> Result<(), SdkError> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || (value.starts_with('-') && digits == "0")
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || digits.parse::<u64>().is_err()
    {
        return Err(SdkError::InvalidResponse(format!(
            "{field} must be a canonical signed atomic decimal string"
        )));
    }
    Ok(())
}

pub fn account_stream_auth_message(
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
            "account stream challenge must be 32-byte lowercase hexadecimal".to_owned(),
        ));
    }
    Ok(
        format!("{ACCOUNT_STREAM_AUTH_DOMAIN}\n{market_id}\n{wallet_address}\n{challenge}")
            .into_bytes(),
    )
}
