use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::market_stream::platform_websocket_url;
use super::*;

type PlatformSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// One sequenced wallet-scoped TWAP progress stream for one market. The
/// first event is the snapshot; later events are `twap_update`, heartbeat, or
/// recovery snapshots on the same stream identity. Any identity or sequence
/// mismatch fails closed so the caller reconnects and recovers from a fresh
/// snapshot.
pub struct TwapStream {
    market_id: String,
    wallet_address: String,
    stream_id: Option<String>,
    sequence: u64,
    socket: PlatformSocket,
}

impl std::fmt::Debug for TwapStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TwapStream")
            .field("market_id", &self.market_id)
            .field("wallet_address", &self.wallet_address)
            .field("stream_id", &self.stream_id)
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}

impl TwapStream {
    pub(crate) async fn connect(
        client: &StrataClient,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<Self, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let url = platform_websocket_url(
            &client.base_url,
            &format!("v2/markets/{market_id}/account/{wallet_address}/twaps/stream"),
        )?;
        let (socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        Ok(Self {
            market_id,
            wallet_address,
            stream_id: None,
            sequence: 0,
            socket,
        })
    }

    pub fn market_id(&self) -> &str {
        &self.market_id
    }

    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }

    /// Receive and validate the next event. `Ok(None)` means the peer closed
    /// cleanly. Callers should reconnect after any error or close.
    pub async fn next_event(&mut self) -> Result<Option<PlatformTwapEvent>, SdkError> {
        loop {
            let Some(frame) = self.socket.next().await else {
                return Ok(None);
            };
            let frame = frame.map_err(|error| SdkError::Stream(error.to_string()))?;
            match frame {
                Message::Text(text) => {
                    let event: PlatformTwapEvent = serde_json::from_str(&text)
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
                        "TWAP stream sent a non-text data frame".to_owned(),
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

    fn validate_event(&mut self, event: &PlatformTwapEvent) -> Result<(), SdkError> {
        match event {
            PlatformTwapEvent::TwapsSnapshot {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                twaps,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                if !valid_handle(stream_id, "twap_stream_") {
                    return Err(SdkError::InvalidResponse(
                        "TWAP stream identity is invalid".to_owned(),
                    ));
                }
                let next = validate_response_atoms(sequence, "sequence", false)?;
                if let Some(current) = &self.stream_id {
                    if current != stream_id || next <= self.sequence {
                        return Err(SdkError::InvalidResponse(
                            "TWAP recovery snapshot did not advance its sequence".to_owned(),
                        ));
                    }
                }
                validate_twap_rows(twaps)?;
                self.stream_id = Some(stream_id.clone());
                self.sequence = next;
            }
            PlatformTwapEvent::TwapUpdate {
                schema_version,
                contract_version,
                market_id,
                wallet_address,
                stream_id,
                sequence,
                previous_sequence,
                twap,
                ..
            } => {
                self.validate_identity(
                    *schema_version,
                    contract_version,
                    market_id,
                    wallet_address,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
                validate_twap_rows(std::slice::from_ref(twap))?;
            }
            PlatformTwapEvent::Heartbeat {
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
        validate_platform_market_response(
            schema_version,
            contract_version,
            market_id,
            &self.market_id,
        )?;
        if wallet_address != self.wallet_address {
            return Err(SdkError::InvalidResponse(
                "TWAP stream wallet does not match the request".to_owned(),
            ));
        }
        Ok(())
    }

    fn validate_sequence(
        &mut self,
        stream_id: &str,
        sequence: &str,
        previous_sequence: &str,
    ) -> Result<(), SdkError> {
        let Some(current) = &self.stream_id else {
            return Err(SdkError::InvalidResponse(
                "TWAP event arrived without its snapshot".to_owned(),
            ));
        };
        let next = validate_response_atoms(sequence, "sequence", false)?;
        let previous = validate_response_atoms(previous_sequence, "previous_sequence", false)?;
        if stream_id != current || previous != self.sequence || next != previous.saturating_add(1) {
            return Err(SdkError::InvalidResponse(
                "TWAP stream sequence gap detected".to_owned(),
            ));
        }
        self.sequence = next;
        Ok(())
    }
}

/// Every TWAP row must carry opaque identities and consistent progress; the
/// same rule the HTTP read applies, plus a per-frame uniqueness check.
pub(crate) fn validate_twap_rows(twaps: &[PlatformTwap]) -> Result<(), SdkError> {
    if twaps.len() > 2_000 {
        return Err(SdkError::InvalidResponse(
            "TWAP stream rows exceed the bounded size".to_owned(),
        ));
    }
    let mut ids = HashSet::new();
    for twap in twaps {
        if !valid_handle(&twap.twap_id, "twap_")
            || !ids.insert(twap.twap_id.as_str())
            || twap.slices_executed > twap.slices_total
            || twap.fills.len() > usize::from(twap.slices_total)
        {
            return Err(SdkError::InvalidResponse(
                "TWAP stream contains an invalid schedule".to_owned(),
            ));
        }
        let total = validate_response_atoms(&twap.total_size_atoms, "total_size_atoms", false)?;
        let executed =
            validate_response_atoms(&twap.executed_size_atoms, "executed_size_atoms", true)?;
        validate_response_atoms(&twap.limit_price_atoms, "limit_price_atoms", false)?;
        validate_response_atoms(
            &twap.gross_quote_executed_atoms,
            "gross_quote_executed_atoms",
            true,
        )?;
        if executed > total {
            return Err(SdkError::InvalidResponse(
                "TWAP executed size exceeds its schedule".to_owned(),
            ));
        }
        for fill in &twap.fills {
            if !valid_handle(&fill.fill_id, "twap_fill_") {
                return Err(SdkError::InvalidResponse(
                    "TWAP fill identity is invalid".to_owned(),
                ));
            }
            validate_response_atoms(&fill.size_atoms, "size_atoms", false)?;
            validate_response_atoms(&fill.price_atoms, "price_atoms", false)?;
        }
    }
    Ok(())
}
