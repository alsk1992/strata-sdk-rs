//! Official Rust client for Strata markets and Sonar quotes.
//!
//! It provides typed requests and responses and validates compatibility, quote
//! binding, and economic fields before returning data to the application.

use async_trait::async_trait;
use base64::Engine as _;
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strata_public_contract::{ErrorResponse, CONTRACT_MAJOR, CONTRACT_VERSION};
use thiserror::Error;

pub use strata_public_contract::{
    CapabilityCatalog, CapabilityDescriptor, CapabilityRisk, CapabilityStability,
    ExecutionChallengeRequest, ExecutionChallengeResponse, ExecutionPrepareRequest,
    ExecutionPrepareResponse, ExecutionStatus, ExecutionSubmitRequest, ExecutionSubmitResponse,
    Market, MarketsResponse, McpExposure, QuoteRequest, QuoteResponse, QuoteSide,
    DEFAULT_SLIPPAGE_BPS,
};

pub const DEFAULT_API_BASE: &str = "https://api.stratabook.app";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const PUBLIC_EXECUTION_AUTH_DOMAIN: &[u8] = b"strata-sonar-execution:v1\0";

#[async_trait]
pub trait SessionSigner: Send + Sync {
    /// Canonical base58 Ed25519 public key registered as the Vault delegate.
    fn public_key(&self) -> &str;

    /// Sign the exact SDK-validated public execution authorization.
    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, String>;

    /// Add only the session signature to an already-verified transaction.
    async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, String>;
}

#[derive(Debug)]
pub struct ExecutionVerificationContext<'a> {
    pub quote: &'a QuoteResponse,
    pub challenge: &'a ExecutionChallengeResponse,
    pub prepared: &'a ExecutionPrepareResponse,
    pub owner_wallet: &'a str,
    pub session_public_key: &'a str,
}

#[async_trait]
pub trait ExecutionVerifier: Send + Sync {
    /// Reject unless the prepared transaction is acceptable for this exact
    /// Vault session and public economic intent.
    async fn verify(&self, context: &ExecutionVerificationContext<'_>) -> Result<(), String>;
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("invalid API base URL: {0}")]
    InvalidBaseUrl(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("market is not available: {0}")]
    MarketNotFound(String),
    #[error("operation is not available for market: {0}")]
    OperationUnavailable(String),
    #[error("Strata API error {status} ({code}): {message}")]
    Api {
        status: StatusCode,
        code: String,
        message: String,
        retryable: bool,
    },
    #[error("invalid public contract response: {0}")]
    InvalidResponse(String),
    #[error("session signer rejected the operation: {0}")]
    Signer(String),
    #[error("prepared transaction was rejected: {0}")]
    Verification(String),
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
}

#[derive(Clone, Debug)]
pub struct StrataClient {
    base_url: Url,
    http: reqwest::Client,
}

impl StrataClient {
    pub fn production() -> Result<Self, SdkError> {
        Self::new(DEFAULT_API_BASE)
    }

    pub fn new(base_url: impl AsRef<str>) -> Result<Self, SdkError> {
        Self::with_timeout(base_url, DEFAULT_TIMEOUT)
    }

    pub fn with_timeout(base_url: impl AsRef<str>, timeout: Duration) -> Result<Self, SdkError> {
        if timeout.is_zero() {
            return Err(SdkError::InvalidRequest(
                "timeout must be greater than zero".to_owned(),
            ));
        }
        let base_url = normalize_base_url(base_url.as_ref())?;
        let http = reqwest::Client::builder().timeout(timeout).build()?;
        Ok(Self { base_url, http })
    }

    pub async fn capabilities(&self) -> Result<CapabilityCatalog, SdkError> {
        let catalog: CapabilityCatalog = self.get("sonar/capabilities", &[]).await?;
        validate_version(catalog.schema_version, &catalog.contract_version)?;

        let mut ids = HashSet::new();
        if catalog
            .capabilities
            .iter()
            .any(|capability| !ids.insert(capability.id.as_str()))
        {
            return Err(SdkError::InvalidResponse(
                "capability IDs must be unique".to_owned(),
            ));
        }
        Ok(catalog)
    }

    pub async fn markets(&self) -> Result<MarketsResponse, SdkError> {
        let markets: MarketsResponse = self.get("sonar/markets", &[]).await?;
        validate_version(markets.schema_version, &markets.contract_version)?;
        Ok(markets)
    }

    /// Request a short-lived Sonar quote by human market label or market ID.
    pub async fn quote(&self, request: QuoteRequest) -> Result<QuoteResponse, SdkError> {
        let amount_in = parse_atoms("amount_in_atoms", &request.amount_in_atoms)?;
        if amount_in == 0 {
            return Err(SdkError::InvalidRequest(
                "amount_in_atoms must be greater than zero".to_owned(),
            ));
        }
        if request.slippage_bps > 1_000 {
            return Err(SdkError::InvalidRequest(
                "slippage_bps must be between 0 and 1,000".to_owned(),
            ));
        }

        let markets = self.markets().await?;
        let market = markets
            .markets
            .iter()
            .find(|market| {
                market.label.eq_ignore_ascii_case(&request.market_id)
                    || market.market_pda.as_deref() == Some(request.market_id.as_str())
            })
            .ok_or_else(|| SdkError::MarketNotFound(request.market_id.clone()))?;
        if !market.ready {
            return Err(SdkError::OperationUnavailable(market.label.clone()));
        }
        let market_pda = market
            .market_pda
            .as_deref()
            .ok_or_else(|| SdkError::MarketNotFound(request.market_id.clone()))?;
        let quote_path = market
            .quote_path
            .as_deref()
            .filter(|path| valid_public_operation_path(path))
            .ok_or_else(|| SdkError::OperationUnavailable(market.label.clone()))?;
        let wire = QuoteRequest {
            market_id: market_pda.to_owned(),
            side: request.side,
            amount_in_atoms: request.amount_in_atoms.clone(),
            slippage_bps: request.slippage_bps,
        };
        let quote: QuoteResponse = self.post(quote_path, &wire).await?;
        validate_quote(&quote, market_pda, &request, amount_in)?;
        Ok(quote)
    }

    /// Execute one short-lived Sonar quote without giving the SDK custody of a
    /// session private key. The transaction verifier always runs before the
    /// session adapter is allowed to sign.
    pub async fn execute_quote<S, V>(
        &self,
        quote: &QuoteResponse,
        owner_wallet: &str,
        account_sequence: u64,
        signer: &S,
        verifier: &V,
        idempotency_key: Option<&str>,
    ) -> Result<ExecutionSubmitResponse, SdkError>
    where
        S: SessionSigner + ?Sized,
        V: ExecutionVerifier + ?Sized,
    {
        validate_version(quote.schema_version, &quote.contract_version)?;
        let now_ms = unix_ms()?;
        if quote.expires_at_ms <= now_ms {
            return Err(SdkError::InvalidRequest("quote has expired".to_owned()));
        }
        let owner_wallet = canonical_public_key(owner_wallet, "owner_wallet")?;
        let session_public_key = canonical_public_key(signer.public_key(), "session_public_key")?;
        let markets = self.markets().await?;
        let market = markets
            .markets
            .iter()
            .find(|market| market.market_pda.as_deref() == Some(quote.market_id.as_str()))
            .ok_or_else(|| SdkError::MarketNotFound(quote.market_id.clone()))?;
        let quote_path = market
            .quote_path
            .as_deref()
            .filter(|path| valid_public_operation_path(path))
            .ok_or_else(|| SdkError::OperationUnavailable(market.label.clone()))?;
        let execution_path = format!(
            "{}/execution",
            quote_path
                .strip_suffix("/quote")
                .ok_or_else(|| SdkError::OperationUnavailable(market.label.clone()))?
        );
        let challenge: ExecutionChallengeResponse = self
            .post(
                &format!("{execution_path}/challenge"),
                &ExecutionChallengeRequest {
                    quote_id: quote.quote_id.clone(),
                    owner_wallet: owner_wallet.clone(),
                    session_public_key: session_public_key.clone(),
                    account_sequence: account_sequence.to_string(),
                },
            )
            .await?;
        validate_execution_challenge(&challenge, quote)?;
        let authorization = validate_execution_authorization(
            &challenge,
            quote,
            &owner_wallet,
            &session_public_key,
            account_sequence,
        )?;
        let signature = signer
            .sign_message(&authorization.bytes)
            .await
            .map_err(SdkError::Signer)?;
        if signature.len() != 64 {
            return Err(SdkError::InvalidResponse(
                "session authorization signature must contain 64 bytes".to_owned(),
            ));
        }
        let prepared: ExecutionPrepareResponse = self
            .post(
                &format!("{execution_path}/prepare"),
                &ExecutionPrepareRequest {
                    challenge_id: challenge.challenge_id.clone(),
                    authorization_signature: bs58::encode(signature).into_string(),
                },
            )
            .await?;
        validate_execution_prepare(&prepared, quote, &challenge, &authorization)?;
        verifier
            .verify(&ExecutionVerificationContext {
                quote,
                challenge: &challenge,
                prepared: &prepared,
                owner_wallet: &owner_wallet,
                session_public_key: &session_public_key,
            })
            .await
            .map_err(SdkError::Verification)?;
        let signed_transaction = signer
            .sign_transaction(&prepared.transaction_base64)
            .await
            .map_err(SdkError::Signer)?;
        base64::engine::general_purpose::STANDARD
            .decode(signed_transaction.trim())
            .map_err(|_| {
                SdkError::InvalidResponse(
                    "session signer returned an invalid base64 transaction".to_owned(),
                )
            })?;
        let idempotency_key =
            normalize_idempotency_key(idempotency_key.unwrap_or(&prepared.execution_id))?;
        let submitted: ExecutionSubmitResponse = self
            .post(
                &format!("{execution_path}/submit"),
                &ExecutionSubmitRequest {
                    execution_id: prepared.execution_id.clone(),
                    signed_transaction_base64: signed_transaction,
                    idempotency_key,
                },
            )
            .await?;
        validate_version(submitted.schema_version, &submitted.contract_version)?;
        if submitted.execution_id != prepared.execution_id
            || submitted.status != ExecutionStatus::Submitted
            || submitted.signature.trim().is_empty()
        {
            return Err(SdkError::InvalidResponse(
                "execution receipt does not match the prepared transaction".to_owned(),
            ));
        }
        Ok(submitted)
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, SdkError> {
        let mut url = self.base_url.join(path).map_err(|error| {
            SdkError::InvalidBaseUrl(format!("could not join public operation: {error}"))
        })?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());

        let response = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            return match serde_json::from_slice::<ErrorResponse>(&bytes) {
                Ok(error) => Err(SdkError::Api {
                    status,
                    code: error.error.code,
                    message: error.error.message,
                    retryable: error.error.retryable,
                }),
                Err(_) => Err(SdkError::Api {
                    status,
                    code: "request_failed".to_owned(),
                    message: "Strata could not complete the request.".to_owned(),
                    retryable: status.is_server_error(),
                }),
            };
        }
        serde_json::from_slice(&bytes).map_err(|error| SdkError::InvalidResponse(error.to_string()))
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, SdkError> {
        let url = self.base_url.join(path).map_err(|error| {
            SdkError::InvalidBaseUrl(format!("could not join public operation: {error}"))
        })?;
        let response = self
            .http
            .post(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(body)
            .send()
            .await?;
        let status = response.status();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            return match serde_json::from_slice::<ErrorResponse>(&bytes) {
                Ok(error) => Err(SdkError::Api {
                    status,
                    code: error.error.code,
                    message: error.error.message,
                    retryable: error.error.retryable,
                }),
                Err(_) => Err(SdkError::Api {
                    status,
                    code: "request_failed".to_owned(),
                    message: "Strata could not complete the request.".to_owned(),
                    retryable: status.is_server_error(),
                }),
            };
        }
        serde_json::from_slice(&bytes).map_err(|error| SdkError::InvalidResponse(error.to_string()))
    }
}

fn normalize_base_url(value: &str) -> Result<Url, SdkError> {
    let mut normalized = value.trim().to_owned();
    if !normalized.ends_with('/') {
        normalized.push('/');
    }
    let url =
        Url::parse(&normalized).map_err(|error| SdkError::InvalidBaseUrl(error.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.cannot_be_a_base() {
        return Err(SdkError::InvalidBaseUrl(
            "URL must use http or https and include a host".to_owned(),
        ));
    }
    Ok(url)
}

fn validate_version(schema_version: u16, contract_version: &str) -> Result<(), SdkError> {
    if schema_version != CONTRACT_MAJOR || contract_version != CONTRACT_VERSION {
        return Err(SdkError::InvalidResponse(format!(
            "unsupported contract {contract_version} (schema {schema_version})"
        )));
    }
    Ok(())
}

fn parse_atoms(field: &str, value: &str) -> Result<u64, SdkError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(SdkError::InvalidResponse(format!(
            "{field} must be an unsigned atomic decimal string"
        )));
    }
    value
        .parse::<u64>()
        .map_err(|_| SdkError::InvalidResponse(format!("{field} exceeds the supported range")))
}

fn valid_public_operation_path(path: &str) -> bool {
    let Some(market_id) = path
        .strip_prefix("/sonar/markets/")
        .and_then(|value| value.strip_suffix("/quote"))
    else {
        return false;
    };
    !market_id.is_empty()
        && !market_id.starts_with('-')
        && !market_id.ends_with('-')
        && market_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_quote(
    quote: &QuoteResponse,
    market_id: &str,
    request: &QuoteRequest,
    requested_amount: u64,
) -> Result<(), SdkError> {
    validate_version(quote.schema_version, &quote.contract_version)?;
    if quote.provider != "Sonar"
        || quote.market_id != market_id
        || quote.side != request.side
        || quote.amount_in_atoms != request.amount_in_atoms
        || quote.quote_id.len() != 35
        || !quote.quote_id.starts_with("sq_")
        || !quote.quote_id[3..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || quote.expires_at_ms <= quote.server_time_ms
    {
        return Err(SdkError::InvalidResponse(
            "quote binding or lifetime is invalid".to_owned(),
        ));
    }

    let consumed = parse_atoms("amount_in_consumed_atoms", &quote.amount_in_consumed_atoms)?;
    let output = parse_atoms("amount_out_atoms", &quote.amount_out_atoms)?;
    let minimum = parse_atoms("minimum_output_atoms", &quote.minimum_output_atoms)?;
    parse_atoms("input_fee_atoms", &quote.input_fee_atoms)?;
    parse_atoms("output_fee_atoms", &quote.output_fee_atoms)?;
    if consumed > requested_amount || minimum > output {
        return Err(SdkError::InvalidResponse(
            "quote economics are internally inconsistent".to_owned(),
        ));
    }
    quote
        .reference_price
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| SdkError::InvalidResponse("reference_price is invalid".to_owned()))?;
    quote
        .price_impact_pct
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| SdkError::InvalidResponse("price_impact_pct is invalid".to_owned()))?;
    Ok(())
}

struct ExecutionAuthorization {
    bytes: Vec<u8>,
    recent_blockhash: String,
    last_valid_block_height: u64,
}

fn validate_execution_challenge(
    challenge: &ExecutionChallengeResponse,
    quote: &QuoteResponse,
) -> Result<(), SdkError> {
    validate_version(challenge.schema_version, &challenge.contract_version)?;
    validate_execution_binding(
        &challenge.quote_id,
        &challenge.market_id,
        challenge.side,
        &challenge.amount_in_atoms,
        &challenge.minimum_output_atoms,
        quote,
    )?;
    if !valid_handle(&challenge.challenge_id, "sc_")
        || challenge.expires_at_ms <= challenge.server_time_ms
        || challenge.expires_at_ms > quote.expires_at_ms
    {
        return Err(SdkError::InvalidResponse(
            "execution challenge binding or lifetime is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_execution_prepare(
    prepared: &ExecutionPrepareResponse,
    quote: &QuoteResponse,
    challenge: &ExecutionChallengeResponse,
    authorization: &ExecutionAuthorization,
) -> Result<(), SdkError> {
    validate_version(prepared.schema_version, &prepared.contract_version)?;
    validate_execution_binding(
        &prepared.quote_id,
        &prepared.market_id,
        prepared.side,
        &prepared.amount_in_atoms,
        &prepared.minimum_output_atoms,
        quote,
    )?;
    if !valid_handle(&prepared.execution_id, "se_")
        || prepared.recent_blockhash != authorization.recent_blockhash
        || prepared.last_valid_block_height != authorization.last_valid_block_height
        || prepared.expires_at_ms > challenge.expires_at_ms
        || prepared.transaction_base64.trim().is_empty()
        || base64::engine::general_purpose::STANDARD
            .decode(prepared.transaction_base64.trim())
            .is_err()
    {
        return Err(SdkError::InvalidResponse(
            "prepared execution changed the signed authorization".to_owned(),
        ));
    }
    Ok(())
}

fn validate_execution_binding(
    quote_id: &str,
    market_id: &str,
    side: QuoteSide,
    amount_in_atoms: &str,
    minimum_output_atoms: &str,
    quote: &QuoteResponse,
) -> Result<(), SdkError> {
    if quote_id != quote.quote_id
        || market_id != quote.market_id
        || side != quote.side
        || amount_in_atoms != quote.amount_in_atoms
        || minimum_output_atoms != quote.minimum_output_atoms
    {
        return Err(SdkError::InvalidResponse(
            "execution does not match the Sonar quote".to_owned(),
        ));
    }
    Ok(())
}

fn validate_execution_authorization(
    challenge: &ExecutionChallengeResponse,
    quote: &QuoteResponse,
    owner_wallet: &str,
    session_public_key: &str,
    account_sequence: u64,
) -> Result<ExecutionAuthorization, SdkError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(challenge.authorization_payload_base64.trim())
        .map_err(|_| SdkError::InvalidResponse("authorization payload is not base64".to_owned()))?;
    let market = decode_public_key(&quote.market_id, "market_id")?;
    let owner = decode_public_key(owner_wallet, "owner_wallet")?;
    let session = decode_public_key(session_public_key, "session_public_key")?;
    let mut cursor = 0usize;
    take_expected(
        &bytes,
        &mut cursor,
        PUBLIC_EXECUTION_AUTH_DOMAIN,
        "authorization domain",
    )?;
    take_expected(&bytes, &mut cursor, &market, "authorization market")?;
    take_expected(
        &bytes,
        &mut cursor,
        quote.quote_id.as_bytes(),
        "authorization quote",
    )?;
    take_expected(&bytes, &mut cursor, &owner, "authorization owner")?;
    take_expected(&bytes, &mut cursor, &session, "authorization session")?;
    let side = take_bytes(&bytes, &mut cursor, 1, "authorization side")?[0];
    if side != if quote.side == QuoteSide::Buy { 0 } else { 1 } {
        return Err(SdkError::InvalidResponse(
            "authorization side changed".to_owned(),
        ));
    }
    take_u64_eq(
        &bytes,
        &mut cursor,
        parse_atoms("amount_in_atoms", &quote.amount_in_atoms)?,
        "authorization input",
    )?;
    take_u64_eq(
        &bytes,
        &mut cursor,
        parse_atoms("minimum_output_atoms", &quote.minimum_output_atoms)?,
        "authorization minimum output",
    )?;
    take_u64_eq(
        &bytes,
        &mut cursor,
        account_sequence,
        "authorization account sequence",
    )?;
    let _output_balance = take_u64(&bytes, &mut cursor, "authorization output balance")?;
    let recent_blockhash = bs58::encode(take_bytes(
        &bytes,
        &mut cursor,
        32,
        "authorization blockhash",
    )?)
    .into_string();
    let last_valid_block_height =
        take_u64(&bytes, &mut cursor, "authorization last valid block height")?;
    take_u64_eq(
        &bytes,
        &mut cursor,
        challenge.expires_at_ms,
        "authorization expiry",
    )?;
    let nonce = take_bytes(&bytes, &mut cursor, 16, "authorization nonce")?;
    if hex::encode(nonce) != challenge.challenge_id[3..] {
        return Err(SdkError::InvalidResponse(
            "authorization challenge nonce changed".to_owned(),
        ));
    }
    let _epoch = take_bytes(&bytes, &mut cursor, 16, "authorization epoch")?;
    if cursor != bytes.len() {
        return Err(SdkError::InvalidResponse(
            "authorization contains unrecognized fields".to_owned(),
        ));
    }
    Ok(ExecutionAuthorization {
        bytes,
        recent_blockhash,
        last_valid_block_height,
    })
}

fn take_expected(
    source: &[u8],
    cursor: &mut usize,
    expected: &[u8],
    field: &str,
) -> Result<(), SdkError> {
    if take_bytes(source, cursor, expected.len(), field)? != expected {
        return Err(SdkError::InvalidResponse(format!("{field} changed")));
    }
    Ok(())
}

fn take_bytes<'a>(
    source: &'a [u8],
    cursor: &mut usize,
    length: usize,
    field: &str,
) -> Result<&'a [u8], SdkError> {
    let end = cursor
        .checked_add(length)
        .filter(|end| *end <= source.len())
        .ok_or_else(|| SdkError::InvalidResponse(format!("{field} is missing")))?;
    let value = &source[*cursor..end];
    *cursor = end;
    Ok(value)
}

fn take_u64(source: &[u8], cursor: &mut usize, field: &str) -> Result<u64, SdkError> {
    let bytes: [u8; 8] = take_bytes(source, cursor, 8, field)?
        .try_into()
        .map_err(|_| SdkError::InvalidResponse(format!("{field} is invalid")))?;
    Ok(u64::from_le_bytes(bytes))
}

fn take_u64_eq(
    source: &[u8],
    cursor: &mut usize,
    expected: u64,
    field: &str,
) -> Result<(), SdkError> {
    if take_u64(source, cursor, field)? != expected {
        return Err(SdkError::InvalidResponse(format!("{field} changed")));
    }
    Ok(())
}

fn decode_public_key(value: &str, field: &str) -> Result<Vec<u8>, SdkError> {
    let bytes = bs58::decode(value.trim())
        .into_vec()
        .map_err(|_| SdkError::InvalidRequest(format!("{field} must be base58")))?;
    if bytes.len() != 32 || bs58::encode(&bytes).into_string() != value.trim() {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be a canonical 32-byte public key"
        )));
    }
    Ok(bytes)
}

fn canonical_public_key(value: &str, field: &str) -> Result<String, SdkError> {
    decode_public_key(value, field)?;
    Ok(value.trim().to_owned())
}

fn valid_handle(value: &str, prefix: &str) -> bool {
    value.len() == prefix.len() + 32
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn normalize_idempotency_key(value: &str) -> Result<String, SdkError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.'
        })
    {
        return Err(SdkError::InvalidRequest(
            "idempotency key must contain 1-64 URL-safe characters".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn unix_ms() -> Result<u64, SdkError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SdkError::InvalidRequest("system clock is before Unix epoch".to_owned()))?;
    u64::try_from(elapsed.as_millis())
        .map_err(|_| SdkError::InvalidRequest("system clock exceeds supported range".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fixture(path: &str) -> serde_json::Value {
        let raw = match path {
            "markets" => strata_public_contract::contract_fixtures::MARKETS,
            "quote" => strata_public_contract::contract_fixtures::QUOTE,
            "capabilities" => strata_public_contract::contract_fixtures::CAPABILITIES,
            _ => unreachable!(),
        };
        serde_json::from_str(raw).unwrap()
    }

    #[tokio::test]
    async fn reads_capabilities_and_quotes_without_internal_metadata() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sonar/capabilities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("capabilities")))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/sonar/markets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("markets")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/sonar/markets/sol-usdc/quote"))
            .and(body_json(serde_json::json!({
                "market_id": "11111111111111111111111111111111",
                "side": "sell",
                "amount_in_atoms": "10000000",
                "slippage_bps": 50
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("quote")))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        let capabilities = client.capabilities().await.unwrap();
        assert!(capabilities
            .capabilities
            .iter()
            .any(|capability| capability.id == "quotes.read"));

        let quote = client
            .quote(QuoteRequest {
                market_id: "SOL/USDC".to_owned(),
                side: QuoteSide::Sell,
                amount_in_atoms: "10000000".to_owned(),
                slippage_bps: 50,
            })
            .await
            .unwrap();
        let public = serde_json::to_value(quote).unwrap();
        assert!(public.get("quote_id").is_some());
        assert!(public.get("unexpected_field").is_none());
    }

    #[test]
    fn rejects_non_http_base_urls() {
        assert!(matches!(
            StrataClient::new("file:///tmp/contract"),
            Err(SdkError::InvalidBaseUrl(_))
        ));
    }

    #[test]
    fn accepts_only_product_level_quote_operation_paths() {
        assert!(valid_public_operation_path("/sonar/markets/sol-usdc/quote"));
        for unsupported_or_ambiguous in [
            "/unsupported/build",
            "/unsupported/quote",
            "/sonar/markets/../quote",
            "/sonar/markets/SOL-USDC/quote",
        ] {
            assert!(!valid_public_operation_path(unsupported_or_ambiguous));
        }
    }
}
