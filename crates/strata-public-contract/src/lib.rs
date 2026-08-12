//! Strata's public product contract.
//!
//! This crate is intentionally isolated from Sonar implementation types. A
//! server must explicitly convert its internal result into these DTOs, making
//! accidental disclosure a compile-time-visible change instead of a serde
//! side-effect.

use serde::{Deserialize, Serialize};

pub const CONTRACT_MAJOR: u16 = 1;
pub const CONTRACT_VERSION: &str = "1.1";
/// Exact-output default for the current read-only quote surface.
pub const DEFAULT_SLIPPAGE_BPS: u16 = 0;

/// Canonical v1 examples used to prove cross-language contract parity.
///
/// This module is excluded from ordinary production builds and exists only for
/// crate verification and downstream SDK tests.
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub mod contract_fixtures {
    pub const ACTION_GRAPH: &str = include_str!("../fixtures/v1/action-graph.json");
    pub const CAPABILITIES: &str = include_str!("../fixtures/v1/capabilities.json");
    pub const EXECUTION_CHALLENGE: &str = include_str!("../fixtures/v1/execution-challenge.json");
    pub const EXECUTION_PREPARE: &str = include_str!("../fixtures/v1/execution-prepare.json");
    pub const EXECUTION_SUBMIT: &str = include_str!("../fixtures/v1/execution-submit.json");
    pub const MARKETS: &str = include_str!("../fixtures/v1/markets.json");
    pub const QUOTE: &str = include_str!("../fixtures/v1/quote.json");
}

pub const ACTION_GRAPH_VERSION: &str = "1.0";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionNodeKind {
    Discovery,
    Read,
    Prepare,
    ExternalSignature,
    Submit,
    Receipt,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionAuthorityModel {
    /// Permission and signer policy are configured by the external agent owner.
    pub permission_source: String,
    /// Private signing material stays in the owner's agent or wallet runtime.
    pub signing_location: String,
    /// Strata accepts public keys and signatures, never private key material.
    pub accepts_private_keys: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOperation {
    pub method: String,
    pub path: String,
    pub mcp_tool: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionNode {
    pub id: String,
    pub kind: ActionNodeKind,
    pub summary: String,
    pub required_capabilities: Vec<String>,
    /// Computed from the live capability catalog for callable Strata nodes.
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<ActionOperation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionEdge {
    pub from: String,
    pub to: String,
    pub condition: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionGraph {
    pub schema_version: u16,
    pub graph_version: String,
    pub contract_version: String,
    pub entry_node: String,
    pub authority: ActionAuthorityModel,
    pub nodes: Vec<ActionNode>,
    pub edges: Vec<ActionEdge>,
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
    /// Maximum execution tolerance. Use [`DEFAULT_SLIPPAGE_BPS`] for an exact
    /// read-only quote.
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

/// Ask Strata for a one-time payload authorizing preparation of an existing
/// Sonar quote. The session key signs locally; no private signing material is
/// accepted by this contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionChallengeRequest {
    pub quote_id: String,
    pub owner_wallet: String,
    pub session_public_key: String,
    /// Vault-owned Market account sequence encoded as an unsigned decimal
    /// string. It prevents a prepared internal fill from targeting stale state.
    pub account_sequence: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionChallengeResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub challenge_id: String,
    pub quote_id: String,
    pub market_id: String,
    pub side: QuoteSide,
    pub amount_in_atoms: String,
    /// The sole customer-facing execution protection.
    pub minimum_output_atoms: String,
    /// Canonical bytes to sign locally with the declared session key.
    pub authorization_payload_base64: String,
    pub server_time_ms: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPrepareRequest {
    pub challenge_id: String,
    /// Base58 Ed25519 signature over `authorization_payload_base64`.
    pub authorization_signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub execution_id: String,
    pub quote_id: String,
    pub market_id: String,
    pub side: QuoteSide,
    pub amount_in_atoms: String,
    /// The same signed minimum returned by the challenge. Preparation may fail,
    /// but it may never weaken this value.
    pub minimum_output_atoms: String,
    /// Partially signed Solana v0 transaction. The session signature slot is
    /// deliberately empty and must be filled locally.
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSubmitRequest {
    pub execution_id: String,
    pub signed_transaction_base64: String,
    /// Caller-generated opaque key. Repeating it may return the original
    /// result, but can never create a second execution.
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionSubmitResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub execution_id: String,
    pub signature: String,
    pub status: ExecutionStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Submitted,
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
        use McpExposure::{
            None as McpNone, Prepare as McpPrepare, Read as McpRead, Submit as McpSubmit,
        };

        let capability = |id: &str,
                          introduced_in: &str,
                          stability,
                          scope: &str,
                          risk,
                          default_enabled,
                          public_sdk,
                          mcp_exposure| {
            CapabilityDescriptor {
                id: id.to_owned(),
                introduced_in: introduced_in.to_owned(),
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
                    "1.0",
                    Stable,
                    "market:read",
                    Read,
                    true,
                    true,
                    McpRead,
                ),
                capability(
                    "books.read",
                    "1.0",
                    Beta,
                    "market:read",
                    Read,
                    false,
                    false,
                    McpNone,
                ),
                capability(
                    "quotes.read",
                    "1.0",
                    Beta,
                    "market:read",
                    Read,
                    true,
                    true,
                    McpRead,
                ),
                capability(
                    "account.read",
                    "1.0",
                    Beta,
                    "account:read",
                    Read,
                    false,
                    false,
                    McpNone,
                ),
                capability(
                    "trade.prepare",
                    "1.1",
                    Beta,
                    "trade:prepare",
                    Prepare,
                    true,
                    true,
                    McpPrepare,
                ),
                capability(
                    "trade.submit",
                    "1.1",
                    Beta,
                    "trade:submit",
                    Submit,
                    true,
                    true,
                    McpSubmit,
                ),
            ],
        }
    }
}

impl ActionGraph {
    /// Build the stable action topology with availability projected from the
    /// live capability catalog. Static documentation never grants access: a
    /// callable node is available only when every required capability is live.
    pub fn for_catalog(catalog: &CapabilityCatalog) -> Self {
        let enabled = |required: &[&str]| {
            required.iter().all(|id| {
                catalog.capabilities.iter().any(|capability| {
                    capability.id == *id && capability.default_enabled && capability.public_sdk
                })
            })
        };
        let operation = |method: &str, path: &str, mcp_tool: &str| ActionOperation {
            method: method.to_owned(),
            path: path.to_owned(),
            mcp_tool: mcp_tool.to_owned(),
        };
        let node = |id: &str,
                    kind,
                    summary: &str,
                    required: &[&str],
                    operation: Option<ActionOperation>| ActionNode {
            id: id.to_owned(),
            kind,
            summary: summary.to_owned(),
            required_capabilities: required.iter().map(|value| (*value).to_owned()).collect(),
            available: operation.is_none() || enabled(required),
            operation,
        };
        let edge = |from: &str, to: &str, condition: &str| ActionEdge {
            from: from.to_owned(),
            to: to.to_owned(),
            condition: condition.to_owned(),
        };

        Self {
            schema_version: CONTRACT_MAJOR,
            graph_version: ACTION_GRAPH_VERSION.to_owned(),
            contract_version: CONTRACT_VERSION.to_owned(),
            entry_node: "discover_capabilities".to_owned(),
            authority: ActionAuthorityModel {
                permission_source: "external_agent_owner".to_owned(),
                signing_location: "external".to_owned(),
                accepts_private_keys: false,
            },
            nodes: vec![
                node(
                    "discover_capabilities",
                    ActionNodeKind::Discovery,
                    "Read the live capabilities that currently expose Strata operations.",
                    &[],
                    Some(operation("GET", "/sonar/capabilities", "strata_capabilities")),
                ),
                node(
                    "discover_markets",
                    ActionNodeKind::Discovery,
                    "Discover ready markets, token decimals, and public operation paths.",
                    &["markets.read"],
                    Some(operation("GET", "/sonar/markets", "strata_markets")),
                ),
                node(
                    "discover_action_graph",
                    ActionNodeKind::Discovery,
                    "Read the executable topology, live node availability, external signing steps, and transition conditions.",
                    &[],
                    Some(operation("GET", "/sonar/action-graph", "strata_action_graph")),
                ),
                node(
                    "request_quote",
                    ActionNodeKind::Read,
                    "Request economics bound to a market, side, exact input atoms, and tolerance.",
                    &["quotes.read"],
                    Some(operation(
                        "POST",
                        "/sonar/markets/{market}/quote",
                        "strata_quote",
                    )),
                ),
                node(
                    "request_execution_challenge",
                    ActionNodeKind::Prepare,
                    "Request canonical authorization bytes for an unexpired quote and external signer.",
                    &["trade.prepare"],
                    Some(operation(
                        "POST",
                        "/sonar/markets/{market}/execution/challenge",
                        "strata_execution_challenge",
                    )),
                ),
                node(
                    "sign_authorization",
                    ActionNodeKind::ExternalSignature,
                    "The agent owner's configured signer signs the returned authorization bytes externally.",
                    &[],
                    None,
                ),
                node(
                    "prepare_execution",
                    ActionNodeKind::Prepare,
                    "Exchange the authorization signature for a quote-bound partially signed transaction.",
                    &["trade.prepare"],
                    Some(operation(
                        "POST",
                        "/sonar/markets/{market}/execution/prepare",
                        "strata_execution_prepare",
                    )),
                ),
                node(
                    "sign_transaction",
                    ActionNodeKind::ExternalSignature,
                    "The external signer verifies and fills its signature slot without sending key material to Strata.",
                    &[],
                    None,
                ),
                node(
                    "submit_execution",
                    ActionNodeKind::Submit,
                    "Submit the signed transaction with an idempotency key.",
                    &["trade.submit"],
                    Some(operation(
                        "POST",
                        "/sonar/markets/{market}/execution/submit",
                        "strata_execution_submit",
                    )),
                ),
                node(
                    "receive_receipt",
                    ActionNodeKind::Receipt,
                    "Receive the execution ID, Solana signature, and submitted status.",
                    &[],
                    None,
                ),
            ],
            edges: vec![
                edge("discover_capabilities", "discover_action_graph", "the returned contract version is supported"),
                edge("discover_action_graph", "discover_markets", "markets.read is enabled"),
                edge("discover_markets", "request_quote", "quotes.read is enabled and the market is ready"),
                edge("request_quote", "request_execution_challenge", "trade.prepare is enabled and the quote is unexpired"),
                edge("request_execution_challenge", "sign_authorization", "the challenge bindings match the quote and signer"),
                edge("sign_authorization", "prepare_execution", "a valid external authorization signature is available"),
                edge("prepare_execution", "sign_transaction", "the prepared transaction preserves the signed bindings"),
                edge("sign_transaction", "submit_execution", "trade.submit is enabled and the signed transaction is unmodified"),
                edge("submit_execution", "receive_receipt", "the execution ID and idempotency key match"),
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
    fn reviewed_action_capabilities_are_public_and_typed() {
        let catalog = CapabilityCatalog::foundation();
        let prepare = catalog
            .capabilities
            .iter()
            .find(|item| item.id == "trade.prepare")
            .unwrap();
        let submit = catalog
            .capabilities
            .iter()
            .find(|item| item.id == "trade.submit")
            .unwrap();
        assert!(prepare.default_enabled && prepare.public_sdk);
        assert_eq!(prepare.risk, CapabilityRisk::Prepare);
        assert_eq!(prepare.mcp_exposure, McpExposure::Prepare);
        assert!(submit.default_enabled && submit.public_sdk);
        assert_eq!(submit.risk, CapabilityRisk::Submit);
        assert_eq!(submit.mcp_exposure, McpExposure::Submit);
    }

    #[test]
    fn shared_v1_fixtures_decode_strictly() {
        let quote: QuoteResponse = serde_json::from_str(contract_fixtures::QUOTE).unwrap();
        let markets: MarketsResponse = serde_json::from_str(contract_fixtures::MARKETS).unwrap();
        let capabilities: CapabilityCatalog =
            serde_json::from_str(contract_fixtures::CAPABILITIES).unwrap();
        let action_graph: ActionGraph =
            serde_json::from_str(contract_fixtures::ACTION_GRAPH).unwrap();

        assert_eq!(quote.contract_version, CONTRACT_VERSION);
        assert_eq!(markets.contract_version, CONTRACT_VERSION);
        assert_eq!(capabilities, CapabilityCatalog::foundation());
        assert_eq!(action_graph, ActionGraph::for_catalog(&capabilities));
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

    #[test]
    fn execution_contract_exposes_only_minimum_output_protection() {
        let challenge: ExecutionChallengeResponse =
            serde_json::from_str(contract_fixtures::EXECUTION_CHALLENGE).unwrap();
        let prepared: ExecutionPrepareResponse =
            serde_json::from_str(contract_fixtures::EXECUTION_PREPARE).unwrap();
        let submitted: ExecutionSubmitResponse =
            serde_json::from_str(contract_fixtures::EXECUTION_SUBMIT).unwrap();

        assert_eq!(
            challenge.minimum_output_atoms,
            prepared.minimum_output_atoms
        );
        assert_eq!(challenge.quote_id, prepared.quote_id);
        assert_eq!(challenge.market_id, prepared.market_id);
        assert_eq!(submitted.execution_id, prepared.execution_id);

        for fixture in [
            contract_fixtures::EXECUTION_CHALLENGE,
            contract_fixtures::EXECUTION_PREPARE,
            contract_fixtures::EXECUTION_SUBMIT,
        ] {
            let value: serde_json::Value = serde_json::from_str(fixture).unwrap();
            let keys = value.as_object().unwrap().keys().collect::<Vec<_>>();
            for forbidden in [
                "route",
                "venue",
                "layer",
                "plan",
                "collar",
                "limit_price",
                "internal",
                "l3",
                "footprint",
            ] {
                assert!(
                    keys.iter().all(|key| !key.contains(forbidden)),
                    "execution contract exposed forbidden field containing {forbidden}"
                );
            }
        }
    }
}
