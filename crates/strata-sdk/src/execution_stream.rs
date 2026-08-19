use std::collections::HashSet;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::market_stream::platform_websocket_url;
use super::*;

const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_WATCHED_EXECUTIONS: usize = 64;

type PlatformSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Sequenced state for the executions an agent prepared in one market. The
/// stream opens with a `watch` frame for the requested handles and begins with
/// a snapshot; later `watch` calls add handles on the same stream. Any market
/// or sequence mismatch fails closed so the caller reconnects.
pub struct ExecutionStream {
    market_id: String,
    stream_id: String,
    sequence: u64,
    watched: HashSet<String>,
    initial_snapshot: Option<PlatformExecutionEvent>,
    socket: PlatformSocket,
}

impl std::fmt::Debug for ExecutionStream {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExecutionStream")
            .field("market_id", &self.market_id)
            .field("stream_id", &self.stream_id)
            .field("sequence", &self.sequence)
            .field("watched", &self.watched.len())
            .finish_non_exhaustive()
    }
}

impl ExecutionStream {
    pub(crate) async fn connect(
        client: &StrataClient,
        market_id: &str,
        execution_ids: &[String],
    ) -> Result<Self, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        let ids = checked_execution_ids(execution_ids)?;
        let url = platform_websocket_url(
            &client.base_url,
            &format!("v2/markets/{market_id}/executions/stream"),
        )?;
        let (mut socket, _) = tokio_tungstenite::connect_async(url.as_str())
            .await
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        send_watch(&mut socket, &ids).await?;
        let frame = tokio::time::timeout(SNAPSHOT_TIMEOUT, socket.next())
            .await
            .map_err(|_| SdkError::Stream("execution snapshot timed out".to_owned()))?
            .ok_or_else(|| SdkError::Stream("socket closed before execution snapshot".to_owned()))?
            .map_err(|error| SdkError::Stream(error.to_string()))?;
        let Message::Text(text) = frame else {
            return Err(SdkError::InvalidResponse(
                "expected a text execution snapshot".to_owned(),
            ));
        };
        let snapshot: PlatformExecutionEvent = serde_json::from_str(&text)
            .map_err(|error| SdkError::InvalidResponse(error.to_string()))?;
        let (stream_id, sequence) = match &snapshot {
            PlatformExecutionEvent::ExecutionsSnapshot {
                schema_version,
                contract_version,
                market_id: response_market,
                stream_id,
                sequence,
                executions,
                unknown_execution_ids,
                ..
            } => {
                validate_platform_market_response(
                    *schema_version,
                    contract_version,
                    response_market,
                    &market_id,
                )?;
                if !valid_handle(stream_id, "execution_stream_") {
                    return Err(SdkError::InvalidResponse(
                        "execution stream identity is invalid".to_owned(),
                    ));
                }
                validate_execution_rows(executions, unknown_execution_ids, &market_id)?;
                (
                    stream_id.clone(),
                    validate_response_atoms(sequence, "sequence", false)?,
                )
            }
            _ => {
                return Err(SdkError::InvalidResponse(
                    "execution stream did not begin with a snapshot".to_owned(),
                ))
            }
        };
        Ok(Self {
            market_id,
            stream_id,
            sequence,
            watched: ids.into_iter().collect(),
            initial_snapshot: Some(snapshot),
            socket,
        })
    }

    pub fn market_id(&self) -> &str {
        &self.market_id
    }

    /// Watch additional handles; the server answers with an update or unknown
    /// event per new handle on the same sequence.
    pub async fn watch(&mut self, execution_ids: &[String]) -> Result<(), SdkError> {
        let fresh: Vec<String> = checked_execution_ids(execution_ids)?
            .into_iter()
            .filter(|id| !self.watched.contains(id))
            .collect();
        if fresh.is_empty() {
            return Ok(());
        }
        if self.watched.len().saturating_add(fresh.len()) > MAX_WATCHED_EXECUTIONS {
            return Err(SdkError::InvalidRequest(format!(
                "at most {MAX_WATCHED_EXECUTIONS} executions can be watched per stream"
            )));
        }
        send_watch(&mut self.socket, &fresh).await?;
        self.watched.extend(fresh);
        Ok(())
    }

    /// The first call returns the snapshot received during connection. Later
    /// calls return sequenced update, expired, unknown, heartbeat, or recovery
    /// snapshot events.
    pub async fn next_event(&mut self) -> Result<Option<PlatformExecutionEvent>, SdkError> {
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
                    let event: PlatformExecutionEvent = serde_json::from_str(&text)
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
                        "execution stream sent a non-text data frame".to_owned(),
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

    fn validate_event(&mut self, event: &PlatformExecutionEvent) -> Result<(), SdkError> {
        match event {
            PlatformExecutionEvent::ExecutionsSnapshot {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                executions,
                unknown_execution_ids,
                ..
            } => {
                validate_platform_market_response(
                    *schema_version,
                    contract_version,
                    market_id,
                    &self.market_id,
                )?;
                let next = validate_response_atoms(sequence, "sequence", false)?;
                if stream_id != &self.stream_id || next <= self.sequence {
                    return Err(SdkError::InvalidResponse(
                        "execution recovery snapshot did not advance its sequence".to_owned(),
                    ));
                }
                validate_execution_rows(executions, unknown_execution_ids, &self.market_id)?;
                self.sequence = next;
            }
            PlatformExecutionEvent::ExecutionUpdate {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                previous_sequence,
                execution,
                ..
            } => {
                validate_platform_market_response(
                    *schema_version,
                    contract_version,
                    market_id,
                    &self.market_id,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
                validate_execution_rows(std::slice::from_ref(execution), &[], &self.market_id)?;
            }
            PlatformExecutionEvent::ExecutionExpired {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                previous_sequence,
                execution_id,
                ..
            }
            | PlatformExecutionEvent::ExecutionUnknown {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                previous_sequence,
                execution_id,
                ..
            } => {
                validate_platform_market_response(
                    *schema_version,
                    contract_version,
                    market_id,
                    &self.market_id,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
                if !valid_handle(execution_id, "se_") {
                    return Err(SdkError::InvalidResponse(
                        "execution stream handle is invalid".to_owned(),
                    ));
                }
            }
            PlatformExecutionEvent::Heartbeat {
                schema_version,
                contract_version,
                market_id,
                stream_id,
                sequence,
                previous_sequence,
                ..
            } => {
                validate_platform_market_response(
                    *schema_version,
                    contract_version,
                    market_id,
                    &self.market_id,
                )?;
                self.validate_sequence(stream_id, sequence, previous_sequence)?;
            }
        }
        Ok(())
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
                "execution stream sequence gap detected".to_owned(),
            ));
        }
        self.sequence = next;
        Ok(())
    }
}

async fn send_watch(socket: &mut PlatformSocket, ids: &[String]) -> Result<(), SdkError> {
    socket
        .send(Message::Text(
            serde_json::to_string(&PlatformExecutionCommand::Watch {
                execution_ids: ids.to_vec(),
            })
            .map_err(|error| SdkError::InvalidRequest(error.to_string()))?
            .into(),
        ))
        .await
        .map_err(|error| SdkError::Stream(error.to_string()))
}

fn checked_execution_ids(execution_ids: &[String]) -> Result<Vec<String>, SdkError> {
    if execution_ids.is_empty() {
        return Err(SdkError::InvalidRequest(
            "at least one execution_id is required".to_owned(),
        ));
    }
    let mut seen = HashSet::new();
    let mut ids = Vec::with_capacity(execution_ids.len());
    for id in execution_ids {
        let id = id.trim().to_owned();
        if !valid_handle(&id, "se_") {
            return Err(SdkError::InvalidRequest(
                "execution_id must be an opaque Strata execution handle".to_owned(),
            ));
        }
        if seen.insert(id.clone()) {
            ids.push(id);
        }
    }
    if ids.len() > MAX_WATCHED_EXECUTIONS {
        return Err(SdkError::InvalidRequest(format!(
            "at most {MAX_WATCHED_EXECUTIONS} executions can be watched per stream"
        )));
    }
    Ok(ids)
}

fn validate_execution_rows(
    executions: &[PlatformExecutionRow],
    unknown_execution_ids: &[String],
    market_id: &str,
) -> Result<(), SdkError> {
    if executions.len() > MAX_WATCHED_EXECUTIONS
        || unknown_execution_ids.len() > MAX_WATCHED_EXECUTIONS
    {
        return Err(SdkError::InvalidResponse(
            "execution stream rows exceed the bounded size".to_owned(),
        ));
    }
    let mut ids = HashSet::new();
    for row in executions {
        let confirmed = row.status == PlatformExecutionState::Confirmed;
        if !valid_handle(&row.execution_id, "se_")
            || !ids.insert(row.execution_id.as_str())
            || row.market_id != market_id
            || confirmed != row.signature.is_some()
            || confirmed != (row.settlement == PlatformSettlementState::Confirmed)
        {
            return Err(SdkError::InvalidResponse(
                "execution stream contains an inconsistent execution".to_owned(),
            ));
        }
    }
    for id in unknown_execution_ids {
        if !valid_handle(id, "se_") || !ids.insert(id.as_str()) {
            return Err(SdkError::InvalidResponse(
                "execution stream unknown handles are invalid".to_owned(),
            ));
        }
    }
    Ok(())
}
