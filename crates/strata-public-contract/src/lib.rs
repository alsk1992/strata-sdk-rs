//! Strata's public product contract.
//!
//! This crate is intentionally isolated from Sonar implementation types. A
//! server must explicitly convert its internal result into these DTOs, making
//! accidental disclosure a compile-time-visible change instead of a serde
//! side-effect.

use serde::{Deserialize, Serialize};

pub const CONTRACT_MAJOR: u16 = 1;
pub const CONTRACT_VERSION: &str = "1.0";

/// Canonical v1 examples used to prove cross-language contract parity.
///
/// This module is excluded from ordinary production builds and exists only for
/// crate verification and downstream SDK tests.
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub mod contract_fixtures {
    pub const CAPABILITIES: &str = include_str!("../fixtures/v1/capabilities.json");
    pub const MARKETS: &str = include_str!("../fixtures/v1/markets.json");
    pub const QUOTE: &str = include_str!("../fixtures/v1/quote.json");
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteRequest {
    pub market_id: String,
    pub side: QuoteSide,
    /// Atomic input amount encoded as a base-10 string. Public money values
    /// never cross JSON as floating-point numbers.
    pub amount_in_atoms: String,
    pub slippage_bps: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuoteResponse {
    pub schema_version: u16,
    pub contract_version: String,
    /// Opaque, short-lived handle. It identifies no execution source and
    /// carries no readable Sonar plan material.
    pub quote_id: String,
    pub server_time_ms: u64,
    pub expires_at_ms: u64,
    pub market_id: String,
    pub side: QuoteSide,
    pub amount_in_atoms: String,
    /// Requested input actually consumed by the quoted execution.
    pub amount_in_consumed_atoms: String,
    pub amount_out_atoms: String,
    pub minimum_output_atoms: String,
    /// Fees charged in the request's input asset. Sonar can charge fees on
    /// either side, so a single unlabelled fee is unsafe.
    pub input_fee_atoms: String,
    /// Fees charged in the response's output asset.
    pub output_fee_atoms: String,
    /// Display-only decimal strings. SDKs may parse these for presentation but
    /// must not use them for settlement or signing bounds.
    pub reference_price: String,
    pub price_impact_pct: String,
    pub provider: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Market {
    pub base: String,
    pub quote: String,
    pub market_pda: Option<String>,
    pub label: String,
    /// Whether the public Sonar quote operation is enabled for this market.
    /// Liquidity remains live state and a quote can still be temporarily
    /// unavailable.
    pub ready: bool,
    pub base_decimals: u8,
    pub quote_decimals: u8,
    /// Stable product-level operation for a Sonar quote. Its implementation
    /// remains opaque.
    pub quote_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MarketsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub markets: Vec<Market>,
}

impl MarketsResponse {
    pub fn new(markets: Vec<Market>) -> Self {
        Self {
            schema_version: CONTRACT_MAJOR,
            contract_version: CONTRACT_VERSION.to_owned(),
            markets,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub error: ErrorDetail,
}

impl ErrorResponse {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            schema_version: CONTRACT_MAJOR,
            contract_version: CONTRACT_VERSION.to_owned(),
            error: ErrorDetail {
                code: code.into(),
                message: message.into(),
                retryable,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStability {
    Internal,
    Beta,
    Stable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityRisk {
    Read,
    Prepare,
    Submit,
    Destructive,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum McpExposure {
    None,
    Read,
    Prepare,
    Submit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub id: String,
    pub introduced_in: String,
    pub stability: CapabilityStability,
    pub required_scope: String,
    pub risk: CapabilityRisk,
    pub default_enabled: bool,
    pub public_sdk: bool,
    pub mcp_exposure: McpExposure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCatalog {
    pub schema_version: u16,
    pub contract_version: String,
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl CapabilityCatalog {
    pub fn foundation() -> Self {
        use CapabilityRisk::{Prepare, Read, Submit};
        use CapabilityStability::{Beta, Stable};
        use McpExposure::{None as McpNone, Read as McpRead};

        let capability =
            |id: &str, stability, scope: &str, risk, default_enabled, public_sdk, mcp_exposure| {
                CapabilityDescriptor {
                    id: id.to_owned(),
                    introduced_in: CONTRACT_VERSION.to_owned(),
                    stability,
                    required_scope: scope.to_owned(),
                    risk,
                    default_enabled,
                    public_sdk,
                    mcp_exposure,
                }
            };

        Self {
            schema_version: CONTRACT_MAJOR,
            contract_version: CONTRACT_VERSION.to_owned(),
            capabilities: vec![
                capability(
                    "markets.read",
                    Stable,
                    "market:read",
                    Read,
                    true,
                    true,
                    McpRead,
                ),
                capability(
                    "books.read",
                    Beta,
                    "market:read",
                    Read,
                    false,
                    false,
                    McpNone,
                ),
                capability(
                    "quotes.read",
                    Beta,
                    "market:read",
                    Read,
                    true,
                    true,
                    McpRead,
                ),
                capability(
                    "account.read",
                    Beta,
                    "account:read",
                    Read,
                    false,
                    false,
                    McpNone,
                ),
                capability(
                    "trade.prepare",
                    Beta,
                    "trade:prepare",
                    Prepare,
                    false,
                    false,
                    McpNone,
                ),
                capability(
                    "trade.submit",
                    Beta,
                    "trade:submit",
                    Submit,
                    false,
                    false,
                    McpNone,
                ),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_quote_field_set_is_sealed() {
        let quote: QuoteResponse = serde_json::from_str(contract_fixtures::QUOTE).unwrap();
        let value = serde_json::to_value(quote).unwrap();
        let object = value.as_object().unwrap();
        let mut actual = object.keys().map(String::as_str).collect::<Vec<_>>();
        actual.sort_unstable();
        let mut expected = vec![
            "amount_in_atoms",
            "amount_in_consumed_atoms",
            "amount_out_atoms",
            "contract_version",
            "expires_at_ms",
            "input_fee_atoms",
            "market_id",
            "minimum_output_atoms",
            "output_fee_atoms",
            "price_impact_pct",
            "provider",
            "quote_id",
            "reference_price",
            "schema_version",
            "server_time_ms",
            "side",
        ];
        expected.sort_unstable();
        assert_eq!(actual, expected, "public quote fields must remain sealed");
        assert_eq!(object["amount_out_atoms"], "1990000");
        assert_eq!(object["minimum_output_atoms"], "1980050");
        assert_eq!(object["provider"], "Sonar");
    }

    #[test]
    fn new_capabilities_are_safe_by_default() {
        let catalog = CapabilityCatalog::foundation();
        for capability in catalog.capabilities {
            if capability.risk != CapabilityRisk::Read {
                assert!(!capability.default_enabled);
            }
        }
    }

    #[test]
    fn shared_v1_fixtures_decode_strictly() {
        let quote: QuoteResponse = serde_json::from_str(contract_fixtures::QUOTE).unwrap();
        let markets: MarketsResponse = serde_json::from_str(contract_fixtures::MARKETS).unwrap();
        let capabilities: CapabilityCatalog =
            serde_json::from_str(contract_fixtures::CAPABILITIES).unwrap();

        assert_eq!(quote.contract_version, CONTRACT_VERSION);
        assert_eq!(markets.contract_version, CONTRACT_VERSION);
        assert_eq!(capabilities, CapabilityCatalog::foundation());
    }

    #[test]
    fn strict_contract_rejects_unreviewed_quote_fields() {
        let mut value: serde_json::Value = serde_json::from_str(contract_fixtures::QUOTE).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected_field".to_owned(), serde_json::json!("hidden"));

        assert!(serde_json::from_value::<QuoteResponse>(value).is_err());
    }
}
