//! Official Rust client for Strata markets and Sonar quotes.
//!
//! It provides typed requests and responses and validates compatibility, quote
//! binding, and economic fields before returning data to the application.

use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::time::Duration;
use strata_public_contract::{ErrorResponse, CONTRACT_MAJOR, CONTRACT_VERSION};
use thiserror::Error;

pub use strata_public_contract::{
    CapabilityCatalog, CapabilityDescriptor, CapabilityRisk, CapabilityStability, Market,
    MarketsResponse, McpExposure, QuoteRequest, QuoteResponse, QuoteSide, DEFAULT_SLIPPAGE_BPS,
};

pub const DEFAULT_API_BASE: &str = "https://api.stratabook.app";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

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
