use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::*;

type PlatformSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Clone, Debug)]
struct BookCursor {
    stream_id: String,
    sequence: u64,
}

/// One sequenced connection to Strata's product-level market stream.
///
/// The stream returns only public book, best-price, trade, and market-status
/// events. A malformed frame or sequence gap returns an error and closes the
/// connection; reconnecting obtains a fresh snapshot.
pub struct MarketDataStream {
    market_id: String,
    socket: PlatformSocket,
    book: Option<BookCursor>,
}

impl std::fmt::Debug for MarketDataStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MarketDataStream")
            .field("market_id", &self.market_id)
            .field("has_book_snapshot", &self.book.is_some())
            .finish_non_exhaustive()
    }
}

impl MarketDataStream {
    pub(crate) async fn connect(client: &StrataClient, market_id: &str) -> Result<Self, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        let url =
            platform_websocket_url(&client.base_url, &format!("v2/markets/{market_id}/stream"))?;
        let (socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        Ok(Self {
            market_id,
            socket,
            book: None,
        })
    }

    pub fn market_id(&self) -> &str {
        &self.market_id
    }

    /// Receive and validate the next public event. `Ok(None)` means the peer
    /// closed cleanly. Callers should reconnect after any error or close.
    pub async fn next_event(&mut self) -> Result<Option<PlatformMarketDataEvent>, SdkError> {
        loop {
            let Some(frame) = self.socket.next().await else {
                return Ok(None);
            };
            let frame = frame.map_err(|error| SdkError::Stream(error.to_string()))?;
            match frame {
                Message::Text(text) => {
                    let event: PlatformMarketDataEvent = serde_json::from_str(&text)
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
                        "market stream sent a non-text data frame".to_owned(),
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

    fn validate_event(&mut self, event: &PlatformMarketDataEvent) -> Result<(), SdkError> {
        match event {
            PlatformMarketDataEvent::BookSnapshot {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                snapshot_id,
                bids,
                asks,
                ..
            } => {
                self.validate_identity(*schema_version, contract_version, market_id)?;
                let sequence = validate_response_atoms(sequence, "sequence", false)?;
                if stream_id.trim().is_empty() || snapshot_id.trim().is_empty() {
                    return Err(SdkError::InvalidResponse(
                        "market stream snapshot identity is invalid".to_owned(),
                    ));
                }
                validate_book_levels(bids, asks)?;
                self.book = Some(BookCursor {
                    stream_id: stream_id.clone(),
                    sequence,
                });
            }
            PlatformMarketDataEvent::BookDelta {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                previous_sequence,
                changes,
                ..
            } => {
                self.validate_identity(*schema_version, contract_version, market_id)?;
                let next = validate_response_atoms(sequence, "sequence", false)?;
                let previous =
                    validate_response_atoms(previous_sequence, "previous_sequence", false)?;
                let cursor = self.book.as_mut().ok_or_else(|| {
                    SdkError::InvalidResponse(
                        "book delta arrived before a recoverable snapshot".to_owned(),
                    )
                })?;
                if &cursor.stream_id != stream_id
                    || previous != cursor.sequence
                    || next != previous.saturating_add(1)
                {
                    return Err(SdkError::InvalidResponse(
                        "market stream sequence gap detected".to_owned(),
                    ));
                }
                for change in changes {
                    validate_response_atoms(&change.price_atoms, "price_atoms", false)?;
                    validate_response_atoms(&change.size_atoms, "size_atoms", true)?;
                }
                cursor.sequence = next;
            }
            PlatformMarketDataEvent::BestBidAsk {
                schema_version,
                contract_version,
                market_id,
                sequence,
                best_bid,
                best_ask,
                ..
            } => {
                self.validate_identity(*schema_version, contract_version, market_id)?;
                validate_response_atoms(sequence, "sequence", false)?;
                if let Some(level) = best_bid {
                    validate_book_level(level)?;
                }
                if let Some(level) = best_ask {
                    validate_book_level(level)?;
                }
            }
            PlatformMarketDataEvent::Trade {
                schema_version,
                contract_version,
                market_id,
                trade,
                ..
            } => {
                self.validate_identity(*schema_version, contract_version, market_id)?;
                if trade.trade_id.trim().is_empty() {
                    return Err(SdkError::InvalidResponse(
                        "market stream trade identity is invalid".to_owned(),
                    ));
                }
                validate_response_atoms(&trade.price_atoms, "price_atoms", false)?;
                validate_response_atoms(&trade.size_atoms, "size_atoms", false)?;
            }
            PlatformMarketDataEvent::MarketStatus {
                schema_version,
                contract_version,
                market_id,
                ..
            }
            | PlatformMarketDataEvent::Heartbeat {
                schema_version,
                contract_version,
                market_id,
                ..
            } => self.validate_identity(*schema_version, contract_version, market_id)?,
        }
        Ok(())
    }

    fn validate_identity(
        &self,
        schema_version: u16,
        contract_version: &str,
        market_id: &str,
    ) -> Result<(), SdkError> {
        validate_platform_market_response(
            schema_version,
            contract_version,
            market_id,
            &self.market_id,
        )
    }
}

pub(crate) fn platform_websocket_url(base_url: &Url, path: &str) -> Result<Url, SdkError> {
    let mut url = base_url.clone();
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        _ => {
            return Err(SdkError::InvalidBaseUrl(
                "stream URL must use http or https".to_owned(),
            ))
        }
    };
    url.set_scheme(scheme)
        .map_err(|_| SdkError::InvalidBaseUrl("could not select WebSocket scheme".to_owned()))?;
    let base_path = url.path().trim_end_matches('/');
    url.set_path(&format!("{base_path}/{path}"));
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}
