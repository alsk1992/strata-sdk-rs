//! Strata's public product contract.
//!
//! This crate is intentionally isolated from Sonar implementation types. A
//! server must explicitly convert its internal result into these DTOs, making
//! accidental disclosure a compile-time-visible change instead of a serde
//! side-effect.

use serde::{Deserialize, Serialize};

pub mod platform;

pub const CONTRACT_MAJOR: u16 = 1;
pub const CONTRACT_VERSION: &str = "1.1";
/// Default maximum tolerance: zero, so a quote is exact unless the caller opts
/// into a lower floor. Tolerance is the caller's choice; it is not price impact.
pub const DEFAULT_MAXIMUM_TOLERANCE_BPS: u16 = 0;
/// Legacy name for [`DEFAULT_MAXIMUM_TOLERANCE_BPS`].
pub const DEFAULT_SLIPPAGE_BPS: u16 = DEFAULT_MAXIMUM_TOLERANCE_BPS;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_tool: Option<String>,
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
    /// Exact-input quote: atomic input amount encoded as a base-10 string.
    /// Public money values never cross JSON as floating-point numbers. Provide
    /// exactly one of `amount_in_atoms` and `amount_out_atoms`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_in_atoms: Option<String>,
    /// Exact-output quote: the atomic output the caller wants. Strata inverts
    /// its best route at quote time and returns the input that delivers it as
    /// `amount_in_atoms` (no cushion of its own); `minimum_output_atoms` is
    /// this amount lowered by `maximum_tolerance_bps` exactly as for exact
    /// input — zero by default, so execution delivers the requested amount or
    /// fails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_out_atoms: Option<String>,
    /// The most the caller accepts below the quoted output, in basis points.
    /// This is the caller's choice and has nothing to do with
    /// `price_impact_pct`, which is measured from the book. Zero (the
    /// default) means the quoted output exactly. `slippage_bps` is accepted
    /// as a legacy spelling.
    #[serde(alias = "slippage_bps")]
    pub maximum_tolerance_bps: u16,
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
    /// User-net output after `output_fee_atoms`. Gross pre-fee output is their
    /// exact atomic sum.
    pub amount_out_atoms: String,
    /// User-net execution floor: `amount_out_atoms` lowered by
    /// `maximum_tolerance_bps` (after fees). Execution delivers at least this
    /// or does not happen.
    pub minimum_output_atoms: String,
    /// Fees charged in the request's input asset. Sonar can charge fees on
    /// either side, so a single unlabelled fee is unsafe.
    pub input_fee_atoms: String,
    /// Strata fee charged in the response's output asset. It is reported
    /// separately so pre-fee and all-in user economics cannot be mixed.
    pub output_fee_atoms: String,
    /// The caller's tolerance echoed back: the most they accept below
    /// `amount_out_atoms`, already applied in `minimum_output_atoms`. It is a
    /// choice, not a measurement — compare `price_impact_pct`.
    pub maximum_tolerance_bps: u16,
    /// Display-only decimal strings. SDKs may parse these for presentation but
    /// must not use them for settlement or signing bounds.
    /// `reference_price` is the best price before the order; `price_impact_pct`
    /// is how far the quoted fills' average price sits from it, measured from
    /// the book. It is not a setting and is unrelated to `maximum_tolerance_bps`.
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
    /// Omit it and Strata resolves the next sequence from the Vault's confirmed
    /// market account when the challenge is issued.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_sequence: Option<String>,
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

/// A prepared execution challenge, signed: the two-step path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPrepareAuthorization {
    pub challenge_id: String,
    /// Base58 Ed25519 signature over `authorization_payload_base64`.
    pub authorization_signature: String,
}

/// Prepare a quote-bound execution transaction: a signed challenge
/// (`Authorized`) or the quote binding itself (`Direct`, one signature — the
/// session's transaction signature is the authorization). The response is
/// identical.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ExecutionPrepareRequest {
    Authorized(ExecutionPrepareAuthorization),
    Direct(ExecutionChallengeRequest),
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
                    "1.1",
                    Beta,
                    "market:read",
                    Read,
                    true,
                    true,
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
                    "1.1",
                    Beta,
                    "account:read",
                    Read,
                    true,
                    true,
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
                capability(
                    "orders.prepare",
                    "1.1",
                    Beta,
                    "orders:prepare",
                    Prepare,
                    false,
                    true,
                    McpPrepare,
                ),
                capability(
                    "orders.submit",
                    "1.1",
                    Beta,
                    "orders:submit",
                    Submit,
                    false,
                    true,
                    McpSubmit,
                ),
                capability(
                    "mm.strand.manage",
                    "1.1",
                    Beta,
                    "mm:write",
                    Submit,
                    false,
                    true,
                    McpSubmit,
                ),
                capability(
                    "mm.current.manage",
                    "1.1",
                    Beta,
                    "mm:write",
                    Submit,
                    false,
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
        let operation = |method: &str, path: &str, mcp_tool: Option<&str>| ActionOperation {
            method: method.to_owned(),
            path: path.to_owned(),
            mcp_tool: mcp_tool.map(str::to_owned),
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
                    Some(operation("GET", "/sonar/capabilities", Some("strata_capabilities"))),
                ),
                node(
                    "discover_markets",
                    ActionNodeKind::Discovery,
                    "Discover ready markets, token decimals, and public operation paths.",
                    &["markets.read"],
                    Some(operation("GET", "/sonar/markets", Some("strata_markets"))),
                ),
                node(
                    "discover_action_graph",
                    ActionNodeKind::Discovery,
                    "Read the executable topology, live node availability, external signing steps, and transition conditions.",
                    &[],
                    Some(operation("GET", "/sonar/action-graph", Some("strata_action_graph"))),
                ),
                node(
                    "discover_platform_capabilities",
                    ActionNodeKind::Discovery,
                    "Read the versioned capabilities available through the official SDK.",
                    &[],
                    Some(operation("GET", "/v2/capabilities", None)),
                ),
                node(
                    "discover_platform_markets",
                    ActionNodeKind::Discovery,
                    "Discover opaque market IDs and current market status.",
                    &["markets.read"],
                    Some(operation("GET", "/v2/markets", None)),
                ),
                node(
                    "read_book",
                    ActionNodeKind::Read,
                    "Read a sequenced Strata book snapshot.",
                    &["books.read"],
                    Some(operation("GET", "/v2/markets/{market_id}/book", None)),
                ),
                node(
                    "read_market_status",
                    ActionNodeKind::Read,
                    "Read tick size, the smallest valid base-atom size, and current market status.",
                    &["books.read"],
                    Some(operation("GET", "/v2/markets/{market_id}/status", None)),
                ),
                node(
                    "read_best_bid_ask",
                    ActionNodeKind::Read,
                    "Read the current best bid and ask.",
                    &["books.read"],
                    Some(operation("GET", "/v2/markets/{market_id}/bbo", None)),
                ),
                node(
                    "read_fees",
                    ActionNodeKind::Read,
                    "Read the market fee schedule.",
                    &["books.read"],
                    Some(operation("GET", "/v2/markets/{market_id}/fees", None)),
                ),
                node(
                    "read_trades",
                    ActionNodeKind::Read,
                    "Read recent anonymized trades.",
                    &["books.read"],
                    Some(operation("GET", "/v2/markets/{market_id}/trades", None)),
                ),
                node(
                    "stream_market",
                    ActionNodeKind::Read,
                    "Subscribe to book changes, trades, and heartbeats with automatic recovery.",
                    &["books.read"],
                    Some(operation("WEBSOCKET", "/v2/markets/{market_id}/stream", None)),
                ),
                node(
                    "authorize_account_read",
                    ActionNodeKind::ExternalSignature,
                    "The agent owner's configured signer authorizes the exact account request or stream challenge.",
                    &[],
                    None,
                ),
                node(
                    "read_account",
                    ActionNodeKind::Read,
                    "Read the owner's sanitized open orders and fills for a Strata market.",
                    &["account.read"],
                    Some(operation(
                        "GET",
                        "/v2/markets/{market_id}/account/{wallet_address}",
                        None,
                    )),
                ),
                node(
                    "stream_account",
                    ActionNodeKind::Read,
                    "Subscribe to signed, sequenced order and fill state for the owner.",
                    &["account.read"],
                    Some(operation(
                        "WEBSOCKET",
                        "/v2/markets/{market_id}/account/{wallet_address}/stream",
                        None,
                    )),
                ),
                node(
                    "request_quote",
                    ActionNodeKind::Read,
                    "Request economics bound to a market, side, exact input atoms, and tolerance.",
                    &["quotes.read"],
                    Some(operation(
                        "POST",
                        "/sonar/markets/{market}/quote",
                        Some("strata_quote"),
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
                        Some("strata_execution_challenge"),
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
                        Some("strata_execution_prepare"),
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
                        Some("strata_execution_submit"),
                    )),
                ),
                node(
                    "receive_receipt",
                    ActionNodeKind::Receipt,
                    "Receive the execution ID, Solana signature, and submitted status.",
                    &[],
                    None,
                ),
                node(
                    "request_order_challenge",
                    ActionNodeKind::Prepare,
                    "Bind a product-level place, cancel, bounded cancel-all, atomic replace, or atomic batch operation to canonical authorization bytes.",
                    &["orders.prepare"],
                    Some(operation(
                        "POST",
                        "/v2/markets/{market_id}/orders/challenge",
                        Some("strata_order_challenge"),
                    )),
                ),
                node(
                    "sign_order_authorization",
                    ActionNodeKind::ExternalSignature,
                    "The agent owner's configured session signer verifies the exact order set and signs externally.",
                    &[],
                    None,
                ),
                node(
                    "prepare_order_control",
                    ActionNodeKind::Prepare,
                    "Exchange the order authorization signature for a partially signed transaction.",
                    &["orders.prepare"],
                    Some(operation(
                        "POST",
                        "/v2/markets/{market_id}/orders/prepare",
                        Some("strata_order_prepare"),
                    )),
                ),
                node(
                    "sign_order_transaction",
                    ActionNodeKind::ExternalSignature,
                    "The external session signer verifies and fills only its transaction signature slot.",
                    &[],
                    None,
                ),
                node(
                    "submit_order_control",
                    ActionNodeKind::Submit,
                    "Submit the unchanged signed order transaction with an idempotency key.",
                    &["orders.submit"],
                    Some(operation(
                        "POST",
                        "/v2/markets/{market_id}/orders/submit",
                        Some("strata_order_submit"),
                    )),
                ),
                node(
                    "receive_order_receipt",
                    ActionNodeKind::Receipt,
                    "Receive the opaque order IDs, transaction signature, and submitted status.",
                    &[],
                    None,
                ),
                node(
                    "open_order_command_stream",
                    ActionNodeKind::Prepare,
                    "Authenticate one persistent sequenced order channel for low-latency commands, explicit self-trade policy, and pushed confirmation.",
                    &["orders.prepare", "orders.submit"],
                    Some(operation(
                        "WEBSOCKET",
                        "/v2/markets/{market_id}/orders/stream",
                        None,
                    )),
                ),
                node(
                    "maintain_dead_man",
                    ActionNodeKind::Submit,
                    "Arm and heartbeat an exact pre-signed cancel-all that executes if the agent stops responding.",
                    &["orders.prepare", "orders.submit"],
                    Some(operation(
                        "WEBSOCKET",
                        "/v2/markets/{market_id}/orders/stream",
                        None,
                    )),
                ),
                node(
                    "certify_order_command_slo",
                    ActionNodeKind::Read,
                    "Measure authenticated command latency, concurrency, sequence integrity, and error rate without submitting a trade.",
                    &["orders.prepare", "orders.submit"],
                    Some(operation(
                        "WEBSOCKET",
                        "/v2/markets/{market_id}/orders/stream",
                        None,
                    )),
                ),
                node(
                    "recover_order_status",
                    ActionNodeKind::Read,
                    "Recover durable submitting, submitted, or failed status after a timeout or restart.",
                    &["orders.submit"],
                    Some(operation(
                        "POST",
                        "/v2/markets/{market_id}/orders/status",
                        Some("strata_order_status"),
                    )),
                ),
            ],
            edges: vec![
                edge("discover_capabilities", "discover_action_graph", "the returned contract version is supported"),
                edge("discover_action_graph", "discover_markets", "markets.read is enabled"),
                edge("discover_action_graph", "discover_platform_capabilities", "the versioned SDK contract is supported"),
                edge("discover_platform_capabilities", "discover_platform_markets", "markets.read is enabled"),
                edge("discover_platform_markets", "read_book", "books.read is enabled and the market is active"),
                edge("discover_platform_markets", "read_market_status", "books.read is enabled"),
                edge("discover_platform_markets", "read_best_bid_ask", "books.read is enabled"),
                edge("discover_platform_markets", "read_fees", "books.read is enabled"),
                edge("discover_platform_markets", "read_trades", "books.read is enabled"),
                edge("read_book", "stream_market", "books.read is enabled and the snapshot sequence is accepted"),
                edge("discover_platform_markets", "authorize_account_read", "account.read is enabled and the owner-configured signer is available"),
                edge("authorize_account_read", "read_account", "the signature binds the wallet, market, request time, and fill limit"),
                edge("read_account", "stream_account", "the stream challenge is signed by the same owner-configured signer"),
                edge("discover_markets", "request_quote", "quotes.read is enabled and the market is ready"),
                edge("request_quote", "request_execution_challenge", "trade.prepare is enabled and the quote is unexpired"),
                edge("request_execution_challenge", "sign_authorization", "the challenge bindings match the quote and signer"),
                edge("sign_authorization", "prepare_execution", "a valid external authorization signature is available"),
                edge("prepare_execution", "sign_transaction", "the prepared transaction preserves the signed bindings"),
                edge("sign_transaction", "submit_execution", "trade.submit is enabled and the signed transaction is unmodified"),
                edge("submit_execution", "receive_receipt", "the execution ID and idempotency key match"),
                edge("discover_platform_markets", "open_order_command_stream", "orders.prepare and orders.submit advertise websocket transport and the owner-configured session signer is available"),
                edge("open_order_command_stream", "request_order_challenge", "signed socket authentication succeeds and an explicit self-trade prevention policy is selected"),
                edge("open_order_command_stream", "maintain_dead_man", "the exact cancel-all authorization and transaction are externally verified and signed"),
                edge("open_order_command_stream", "certify_order_command_slo", "a release or recurring production load certificate is required"),
                edge("discover_platform_markets", "request_order_challenge", "orders.prepare is enabled and the market accepts order control"),
                edge("request_order_challenge", "sign_order_authorization", "the action and exact opaque order set match owner intent"),
                edge("sign_order_authorization", "prepare_order_control", "a valid external authorization signature is available"),
                edge("prepare_order_control", "sign_order_transaction", "the prepared transaction preserves the signed order bindings"),
                edge("sign_order_transaction", "submit_order_control", "orders.submit is enabled and the signed transaction is unmodified"),
                edge("submit_order_control", "receive_order_receipt", "the control ID and idempotency key match"),
                edge("submit_order_control", "recover_order_status", "the submission result is ambiguous or either process restarted"),
                edge("submit_order_control", "maintain_dead_man", "the agent has resting exposure that must fail closed on disconnect"),
                edge("recover_order_status", "receive_order_receipt", "durable status is submitted"),
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
            "maximum_tolerance_bps",
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
        assert_eq!(object["maximum_tolerance_bps"], 50);
        assert_eq!(object["provider"], "Sonar");

        // The request field is `maximum_tolerance_bps`; the legacy spelling
        // still deserializes so older clients keep working.
        let legacy: QuoteRequest = serde_json::from_value(serde_json::json!({
            "market_id": "11111111111111111111111111111111",
            "side": "sell",
            "amount_in_atoms": "10000000",
            "slippage_bps": 25
        }))
        .unwrap();
        assert_eq!(legacy.maximum_tolerance_bps, 25);
        assert!(serde_json::to_string(&legacy)
            .unwrap()
            .contains("\"maximum_tolerance_bps\":25"));
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
