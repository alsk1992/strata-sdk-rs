//! Public SDK 2.0 request and response primitives.

use serde::{Deserialize, Serialize};

use crate::{CapabilityRisk, McpExposure};

pub const PLATFORM_SCHEMA_VERSION: u16 = 2;
pub const PLATFORM_CONTRACT_VERSION: &str = "2.0";
pub const PLATFORM_ACTION_GRAPH: &str = include_str!("../fixtures/v2/platform-action-graph.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_SERVICE_STATUS_FIXTURE: &str =
    include_str!("../fixtures/v2/platform-status.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_CAPABILITIES_FIXTURE: &str =
    include_str!("../fixtures/v2/platform-capabilities.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ASSETS_FIXTURE: &str = include_str!("../fixtures/v2/assets.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_SWAP_QUOTE_FIXTURE: &str = include_str!("../fixtures/v2/swap-quote.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_MARKETS_FIXTURE: &str = include_str!("../fixtures/v2/markets.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_BOOK_FIXTURE: &str = include_str!("../fixtures/v2/book.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_BBO_FIXTURE: &str = include_str!("../fixtures/v2/bbo.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_FEES_FIXTURE: &str = include_str!("../fixtures/v2/fees.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_STATUS_FIXTURE: &str = include_str!("../fixtures/v2/status.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_CANDLES_FIXTURE: &str = include_str!("../fixtures/v2/candles.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_MARK_FIXTURE: &str = include_str!("../fixtures/v2/mark.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_EXECUTION_STATUS_FIXTURE: &str =
    include_str!("../fixtures/v2/execution-status.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_TWAPS_FIXTURE: &str = include_str!("../fixtures/v2/twaps.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_TWAP_CHALLENGE_FIXTURE: &str =
    include_str!("../fixtures/v2/twap-challenge.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_TWAP_PREPARE_FIXTURE: &str = include_str!("../fixtures/v2/twap-prepare.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_TWAP_SUBMIT_FIXTURE: &str = include_str!("../fixtures/v2/twap-submit.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_PORTFOLIO_HISTORY_FIXTURE: &str =
    include_str!("../fixtures/v2/portfolio-history.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_PORTFOLIO_FIXTURE: &str = include_str!("../fixtures/v2/portfolio.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_REWARDS_FIXTURE: &str = include_str!("../fixtures/v2/rewards.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_REFERRALS_FIXTURE: &str = include_str!("../fixtures/v2/referrals.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_REFERRAL_LINK_FIXTURE: &str = include_str!("../fixtures/v2/referral-link.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_REFERRAL_CLAIM_FIXTURE: &str =
    include_str!("../fixtures/v2/referral-claim.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_VAULT_STATUS_FIXTURE: &str = include_str!("../fixtures/v2/vault-status.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_VAULT_PAUSE_PREPARE_FIXTURE: &str =
    include_str!("../fixtures/v2/vault-pause-prepare.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_VAULT_SETUP_PREPARE_FIXTURE: &str =
    include_str!("../fixtures/v2/vault-setup-prepare.json");
pub const PLATFORM_VAULT_DELEGATE_PREPARE_FIXTURE: &str =
    include_str!("../fixtures/v2/vault-delegate-prepare.json");
pub const PLATFORM_VAULT_POLICY_PREPARE_FIXTURE: &str =
    include_str!("../fixtures/v2/vault-policy-prepare.json");
pub const PLATFORM_VAULT_DEPOSIT_PREPARE_FIXTURE: &str =
    include_str!("../fixtures/v2/vault-deposit-prepare.json");
pub const PLATFORM_VAULT_WITHDRAW_PREPARE_FIXTURE: &str =
    include_str!("../fixtures/v2/vault-withdraw-prepare.json");
pub const PLATFORM_VAULT_SUBMIT_FIXTURE: &str = include_str!("../fixtures/v2/vault-submit.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_BUGS_FIXTURE: &str = include_str!("../fixtures/v2/bugs.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_BUG_SUBMIT_FIXTURE: &str = include_str!("../fixtures/v2/bug-submit.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_TRADES_FIXTURE: &str = include_str!("../fixtures/v2/trades.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ACCOUNT_FIXTURE: &str = include_str!("../fixtures/v2/account.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_MAKER_REPUTATION_FIXTURE: &str =
    include_str!("../fixtures/v2/maker-reputation.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_MAKER_STATUS_FIXTURE: &str = include_str!("../fixtures/v2/maker-status.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_MAKER_STREAM_FIXTURE: &str = include_str!("../fixtures/v2/maker-stream.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_TWAP_STREAM_FIXTURE: &str = include_str!("../fixtures/v2/twap-stream.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_EXECUTION_STREAM_FIXTURE: &str =
    include_str!("../fixtures/v2/execution-stream.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ORDER_CHALLENGE_FIXTURE: &str =
    include_str!("../fixtures/v2/order-challenge.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ORDER_PREPARE_FIXTURE: &str = include_str!("../fixtures/v2/order-prepare.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ORDER_SUBMIT_FIXTURE: &str = include_str!("../fixtures/v2/order-submit.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ORDER_STATUS_FIXTURE: &str = include_str!("../fixtures/v2/order-status.json");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionSource {
    ExternalAgentOwner,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SigningLocation {
    External,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAuthority {
    pub permission_source: PermissionSource,
    pub signing_location: SigningLocation,
    pub accepts_private_keys: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTransport {
    Http,
    Websocket,
    Mcp,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMarketState {
    Active,
    ReadOnly,
    QuoteOnly,
    CancelOnly,
    Paused,
    Warming,
    Degraded,
    Unavailable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOrderState {
    Created,
    Accepted,
    Open,
    PartiallyFilled,
    Filled,
    CancelPending,
    Cancelled,
    Expired,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformSettlementState {
    NotApplicable,
    Pending,
    Confirmed,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformPublicErrorCode {
    InvalidRequest,
    UnsupportedCapability,
    MarketUnavailable,
    MarketWarming,
    QuoteUnavailable,
    QuoteExpired,
    PriceBoundFailed,
    InsufficientBalance,
    PolicyRejected,
    SessionExpired,
    SequenceConflict,
    DuplicateClientId,
    OrderRejected,
    OrderNotFound,
    CancelTooLate,
    SelfTradePrevented,
    DeadManExpired,
    RateLimited,
    TemporarilyUnavailable,
    SubmissionAmbiguous,
    SettlementPending,
    SettlementFailed,
}

/// Exact asset amount. Public money never crosses the contract as a float.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactAmount {
    pub asset_id: String,
    pub atoms: String,
}

/// Sequence metadata shared by all recoverable state streams.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SequenceEnvelope {
    pub stream_id: String,
    pub sequence: String,
    pub previous_sequence: Option<String>,
    pub server_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PageInfo {
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicOperationError {
    pub code: PlatformPublicErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<String>,
}

/// One operation currently callable through the live v2 gateway.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivePlatformCapability {
    pub id: String,
    pub risk: CapabilityRisk,
    pub required_scope: String,
    pub transports: Vec<PlatformTransport>,
    pub mcp_exposure: McpExposure,
}

/// Operations currently available to the client.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformDiscoveryResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub authority: PlatformAuthority,
    pub capabilities: Vec<LivePlatformCapability>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformServiceState {
    Operational,
    Degraded,
}

/// Product-level readiness without leaking private implementation details.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformServiceStatusResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub status: PlatformServiceState,
    pub available_operations: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformActionKind {
    Discovery,
    Read,
    Prepare,
    ExternalSignature,
    Submit,
    Receipt,
    Stream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformGraphRelation {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformGraphModule {
    pub id: String,
    pub client_property: String,
    pub capability_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOperationTransport {
    pub transport: PlatformTransport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOperation {
    pub id: String,
    pub capability_id: String,
    pub kind: PlatformActionKind,
    pub summary: String,
    pub transports: Vec<PlatformOperationTransport>,
    pub available: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformWorkflowNode {
    pub id: String,
    pub kind: PlatformActionKind,
    pub capability_id: Option<String>,
    pub operation_ids: Vec<String>,
    pub available: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformWorkflowEdge {
    pub from: String,
    pub to: String,
    pub condition: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformWorkflow {
    pub id: String,
    pub entry_node: String,
    pub nodes: Vec<PlatformWorkflowNode>,
    pub edges: Vec<PlatformWorkflowEdge>,
}

/// Complete customer-safe product graph. Static package support is projected
/// against live capability discovery before this response is served.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformActionGraphResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub graph_version: String,
    pub entry_operation_id: String,
    pub authority: PlatformAuthority,
    pub entities: Vec<String>,
    pub relations: Vec<PlatformGraphRelation>,
    pub modules: Vec<PlatformGraphModule>,
    pub operations: Vec<PlatformOperation>,
    pub workflows: Vec<PlatformWorkflow>,
}

impl PlatformActionGraphResponse {
    pub fn foundation() -> Self {
        serde_json::from_str(PLATFORM_ACTION_GRAPH)
            .expect("embedded platform action graph must be valid")
    }

    /// Live discovery is the authority. Package support alone never makes a
    /// callable operation or workflow node available.
    pub fn project_availability(
        &mut self,
        live_capability_ids: &std::collections::BTreeSet<String>,
    ) {
        for operation in &mut self.operations {
            operation.available = live_capability_ids.contains(&operation.capability_id);
        }
        for workflow in &mut self.workflows {
            for node in &mut workflow.nodes {
                node.available = node
                    .capability_id
                    .as_ref()
                    .is_none_or(|capability_id| live_capability_ids.contains(capability_id));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformNetwork {
    Solana,
}

/// Asset identity used by ordinary SDK operations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAsset {
    pub asset_id: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
    pub network: PlatformNetwork,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAssetsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub assets: Vec<PlatformAsset>,
    pub page: PageInfo,
}

/// Exact-input asset swap request. Asset identifiers come from
/// [`PlatformAssetsResponse`]; implementation-specific identifiers are not
/// part of this contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformSwapQuoteRequest {
    pub input_asset_id: String,
    pub output_asset_id: String,
    pub amount_in_atoms: String,
    #[serde(default)]
    pub maximum_tolerance_bps: u16,
}

/// Short-lived customer economics for an exact-input asset swap.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformSwapQuoteResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub quote_id: String,
    pub server_time_ms: u64,
    pub expires_at_ms: u64,
    pub input_asset_id: String,
    pub output_asset_id: String,
    pub amount_in_atoms: String,
    pub amount_in_consumed_atoms: String,
    pub amount_out_atoms: String,
    pub minimum_output_atoms: String,
    pub input_fee_atoms: String,
    pub output_fee_atoms: String,
    pub maximum_tolerance_bps: u16,
    pub reference_price: String,
    pub price_impact_pct: String,
    pub provider: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMarketAction {
    Quote,
    ExecuteImmediate,
    PlaceOrder,
    ScheduleTwap,
}

/// Stable market metadata for public SDK operations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMarket {
    pub market_id: String,
    pub label: String,
    pub base_asset_id: String,
    pub quote_asset_id: String,
    pub status: PlatformMarketState,
    pub available_actions: Vec<PlatformMarketAction>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMarketsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub markets: Vec<PlatformMarket>,
    pub page: PageInfo,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBookLevel {
    /// Quote atoms per whole base unit, encoded as an unsigned decimal string.
    pub price_atoms: String,
    /// Available base quantity in base atoms, encoded as an unsigned decimal string.
    pub size_atoms: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformBookSide {
    Bid,
    Ask,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBookChange {
    pub side: PlatformBookSide,
    pub price_atoms: String,
    /// Zero removes the price level.
    pub size_atoms: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBookSnapshotResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub stream_id: String,
    pub sequence: String,
    pub server_time_ms: u64,
    pub snapshot_id: String,
    pub bids: Vec<PlatformBookLevel>,
    pub asks: Vec<PlatformBookLevel>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBestBidAskResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub stream_id: String,
    pub sequence: String,
    pub server_time_ms: u64,
    pub best_bid: Option<PlatformBookLevel>,
    pub best_ask: Option<PlatformBookLevel>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformFeeScheduleResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub server_time_ms: u64,
    pub passive_maker_fee_bps: u16,
    pub maximum_immediate_execution_fee_bps: u16,
    pub book_prices_include_trading_fees: bool,
    pub exact_fee_returned_by_quote: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMarketStatusResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub server_time_ms: u64,
    pub status: PlatformMarketState,
    pub tick_size_atoms: String,
    /// Smallest accepted base-asset quantity. Strata orders are atom-denominated,
    /// so this is `1`; it is a size, never a price or `Market.base_lot_size`.
    pub minimum_order_size_atoms: String,
}

/// Decimal prices are strings so no SDK boundary silently rounds money.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformCandle {
    pub started_at_ms: u64,
    pub open_price: String,
    pub high_price: String,
    pub low_price: String,
    pub close_price: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformCandlesResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub server_time_ms: u64,
    pub resolution_seconds: u32,
    pub candles: Vec<PlatformCandle>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMarkResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub server_time_ms: u64,
    pub price_atoms_per_base_unit: Option<String>,
    pub quote_decimals: u8,
    pub stale: bool,
    pub age_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformExecutionState {
    Prepared,
    Confirmed,
}

/// Recoverable immediate-execution receipt. Confirmed rows are journalled and
/// survive a market-service restart.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformExecutionStatusResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub execution_id: String,
    pub market_id: String,
    pub status: PlatformExecutionState,
    pub signature: Option<String>,
    pub settlement: PlatformSettlementState,
    pub updated_at_ms: u64,
}

/// One watched immediate execution as the stream sees it: the same fields as
/// the recoverable HTTP receipt without the envelope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformExecutionRow {
    pub execution_id: String,
    pub market_id: String,
    pub status: PlatformExecutionState,
    pub signature: Option<String>,
    pub settlement: PlatformSettlementState,
    pub updated_at_ms: u64,
}

/// Client frame for the execution stream: watch one or more opaque execution
/// handles issued by `execution.prepare` in this market.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformExecutionCommand {
    Watch { execution_ids: Vec<String> },
}

/// Sequenced execution stream (`execution.stream`) for one market. The client
/// opens the socket and sends a `watch` frame; the server answers with one
/// `executions_snapshot` for the watched handles, then `execution_update`
/// whenever a watched execution is prepared, confirmed on chain, or expires
/// unconfirmed, `execution_unknown` for handles this market never issued or
/// no longer remembers, and heartbeats. Later `watch` frames add handles and
/// produce update/unknown events for them.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformExecutionEvent {
    ExecutionsSnapshot {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
        executions: Vec<PlatformExecutionRow>,
        unknown_execution_ids: Vec<String>,
    },
    ExecutionUpdate {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        execution: PlatformExecutionRow,
    },
    ExecutionExpired {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        execution_id: String,
    },
    ExecutionUnknown {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        execution_id: String,
    },
    Heartbeat {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTwapState {
    Active,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapFill {
    pub fill_id: String,
    pub size_atoms: String,
    pub price_atoms: String,
    pub gross_quote_atoms: String,
    pub base_fee_atoms: String,
    pub quote_fee_atoms: String,
    pub signature: Option<String>,
    pub observed_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwap {
    pub twap_id: String,
    pub side: PlatformTradeSide,
    pub status: PlatformTwapState,
    pub slices_total: u16,
    pub slices_executed: u16,
    pub interval_slots: u32,
    pub maximum_tolerance_bps: u16,
    pub limit_price_atoms: String,
    pub total_size_atoms: String,
    pub executed_size_atoms: String,
    pub gross_quote_executed_atoms: String,
    pub complete_execution_value: bool,
    pub created_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub placed_signature: Option<String>,
    pub terminal_signature: Option<String>,
    pub fills: Vec<PlatformTwapFill>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub wallet_address: String,
    pub server_time_ms: u64,
    pub twaps: Vec<PlatformTwap>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTwapControlAction {
    Place,
    Cancel,
}

/// Request exact authorization bytes for one Vault-owned TWAP action. The
/// external owner chooses the session signer; Strata never receives its key.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformTwapChallengeRequest {
    Place {
        owner_wallet: String,
        session_public_key: String,
        side: PlatformTradeSide,
        total_size_atoms: String,
        slices_total: u16,
        maximum_tolerance_bps: u16,
        /// Slots between slices. Slot time is a cluster parameter (400 ms
        /// today, stepping down to 200 ms under SIMD-0525), so a schedule
        /// expressed in slots runs faster in wall time as slots shorten.
        interval_slots: u32,
        limit_price_atoms: String,
    },
    Cancel {
        owner_wallet: String,
        session_public_key: String,
        twap_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapChallengeResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub challenge_id: String,
    pub market_id: String,
    pub action: PlatformTwapControlAction,
    pub twap_id: String,
    pub authorization_payload_base64: String,
    pub server_time_ms: u64,
    pub expires_at_ms: u64,
}

/// A prepared TWAP challenge, signed: the two-step path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapPrepareAuthorization {
    pub challenge_id: String,
    /// Base58 Ed25519 signature over `authorization_payload_base64`.
    pub authorization_signature: String,
}

/// Prepare a TWAP-control transaction: a signed challenge (`Authorized`) or
/// the action itself (`Direct`, one signature — the transaction signature is
/// the authorization). The response is identical.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PlatformTwapPrepareRequest {
    Authorized(PlatformTwapPrepareAuthorization),
    Direct(PlatformTwapChallengeRequest),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub twap_control_id: String,
    pub market_id: String,
    pub action: PlatformTwapControlAction,
    pub twap_id: String,
    /// Backend-partially-signed transaction. The external session signer
    /// verifies and fills only its signature slot.
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapSubmitRequest {
    pub twap_control_id: String,
    pub signed_transaction_base64: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTwapSubmitResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub twap_control_id: String,
    pub market_id: String,
    pub action: PlatformTwapControlAction,
    pub twap_id: String,
    pub signature: String,
    pub status: PlatformOrderSubmissionStatus,
}

/// Sequenced wallet-scoped TWAP progress stream (`algos.twap.stream`) for one
/// market. It starts with a `twaps_snapshot`, then sends one `twap_update`
/// carrying the complete sanitized TWAP row whenever a schedule is created,
/// executes a slice, or reaches a terminal state, plus heartbeats. Every event
/// carries the stream identity and previous sequence; a recovery snapshot
/// advances the sequence on the same identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum PlatformTwapEvent {
    TwapsSnapshot {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
        twaps: Vec<PlatformTwap>,
    },
    TwapUpdate {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        twap: PlatformTwap,
    },
    Heartbeat {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PlatformPortfolioHistoryRange {
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "7d")]
    Week,
    #[serde(rename = "30d")]
    Month,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioHistoryPoint {
    pub recorded_at_ms: u64,
    pub equity_usd_micros: String,
    pub available_usd_micros: String,
    pub locked_usd_micros: String,
    pub market_count: u32,
}

/// Stored account-equity history. It never fabricates data before collection
/// began and keeps all currency values in exact USD micros.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioHistoryResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub wallet_address: String,
    pub server_time_ms: u64,
    pub range: PlatformPortfolioHistoryRange,
    pub points: Vec<PlatformPortfolioHistoryPoint>,
    pub collecting: bool,
    pub first_sample_ms: Option<u64>,
    pub last_sample_ms: Option<u64>,
}

/// One asset the owner holds on Strata, across every live market. Assets
/// with no holdings are omitted. A balance is a balance: `total` is what the
/// owner has, `available` is what is free to trade or withdraw, `locked` is
/// what resting orders reserve.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioBalance {
    pub asset_id: String,
    /// Holdings not reserved by resting orders.
    pub available_atoms: String,
    /// Holdings reserved by resting orders.
    pub locked_atoms: String,
    /// `available_atoms + locked_atoms`.
    pub total_atoms: String,
    /// Exact USD micros for `total_atoms` when a fresh public mark exists.
    pub value_usd_micros: Option<String>,
}

/// The owner's Vault position in one live market. Only markets where the
/// Vault holds a market account are listed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioPosition {
    pub market_id: String,
    pub base_asset_id: String,
    pub quote_asset_id: String,
    pub base_available_atoms: String,
    pub base_locked_atoms: String,
    pub quote_available_atoms: String,
    pub quote_locked_atoms: String,
}

/// One open order, tagged with the market it rests in.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioOrder {
    pub market_id: String,
    pub order_id: String,
    pub side: PlatformTradeSide,
    pub order_type: PlatformOrderType,
    pub state: PlatformOrderState,
    pub limit_price_atoms: String,
    pub original_size_atoms: String,
    pub remaining_size_atoms: String,
}

/// One recent fill, tagged with the market it happened in.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioFill {
    pub market_id: String,
    pub fill_id: String,
    pub side: PlatformTradeSide,
    pub price_atoms: String,
    pub size_atoms: String,
    pub fee_quote_atoms: String,
    pub fee_is_final: bool,
    pub settlement: PlatformSettlementState,
    pub executed_at_ms: u64,
    pub confirmed_at_ms: Option<u64>,
    pub transaction_id: Option<String>,
    pub realized_pnl_quote_atoms: String,
}

/// The owner's whole account in one public read, by wallet address: balances,
/// per-market positions, open orders, and recent fills across every live
/// market, plus USD totals. No signature and no market selection is needed.
/// Amounts are exact atomic strings; USD totals are null whenever any held
/// asset lacks a fresh public mark, so a partial valuation is never presented
/// as complete.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformPortfolioResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub wallet_address: String,
    pub server_time_ms: u64,
    /// When the on-chain state behind this snapshot was observed.
    pub observed_at_ms: u64,
    /// Chain slot the snapshot was observed at.
    pub observed_slot: String,
    /// Live markets included in the snapshot.
    pub market_count: u32,
    pub balances: Vec<PlatformPortfolioBalance>,
    pub positions: Vec<PlatformPortfolioPosition>,
    /// Every open order across every live market.
    pub open_orders: Vec<PlatformPortfolioOrder>,
    /// Recent fills across every live market, newest first (bounded).
    pub recent_fills: Vec<PlatformPortfolioFill>,
    /// Markets whose orders and fills could not be read for this snapshot;
    /// balances and positions are still complete.
    pub unavailable_market_ids: Vec<String>,
    /// Sum of every balance's `value_usd_micros`; null unless the valuation is complete.
    pub equity_usd_micros: Option<String>,
    /// Exact USD value of every available balance; null unless the valuation is complete.
    pub available_usd_micros: Option<String>,
    /// `equity_usd_micros - available_usd_micros`; null unless the valuation is complete.
    pub locked_usd_micros: Option<String>,
    pub valuation_complete: bool,
    pub unpriced_asset_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultState {
    Absent,
    Active,
    Paused,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultSessionState {
    Absent,
    Active,
    Expired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultWithdrawalMode {
    Unrestricted,
    Blocked,
    Restricted,
}

/// One asset-specific execution limit. A null maximum means that the session
/// is permitted to use the asset without a per-execution amount ceiling.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultSpendingLimit {
    pub asset_id: String,
    pub maximum_per_execution_atoms: Option<String>,
}

/// Sanitized state for the requested external session key. It intentionally
/// omits all construction accounts and price-source identities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultSessionStatus {
    pub session_public_key: String,
    pub state: PlatformVaultSessionState,
    pub expires_at_ms: Option<u64>,
    pub permanent: bool,
    pub minimum_interval_seconds: u32,
    pub maximum_tolerance_bps: u16,
    pub last_execution_at_ms: Option<u64>,
    pub market_execution_ready: bool,
    pub price_protection_active: bool,
    pub spending_limits: Vec<PlatformVaultSpendingLimit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultWithdrawalAccess {
    pub mode: PlatformVaultWithdrawalMode,
    pub allowed_wallet_addresses: Vec<String>,
}

/// Product-level Vault state for an owner and, when requested, one external
/// session key. Chain construction identities never cross this boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultStatusResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub state: PlatformVaultState,
    pub session: Option<PlatformVaultSessionStatus>,
    pub withdrawal_access: PlatformVaultWithdrawalAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultPausePrepareRequest {
    pub wallet_address: String,
    pub paused: bool,
}

/// An unsigned owner transaction. The external owner must verify its wallet
/// and requested state before signing and broadcasting it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultPausePrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub paused: bool,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub owner_signature_required: bool,
    /// Opaque handle for this prepared transaction. Hand it back with the
    /// owner-signed transaction to `vault.relay` and Strata submits it — no
    /// RPC or SOL needed on the owner side.
    pub preparation_id: String,
    /// `true` when Strata is the transaction fee payer and covers any rent the
    /// action creates, so the owner needs no SOL at all. `false` means the
    /// owner wallet is the fee payer (Strata still submits it on request).
    pub sponsored: bool,
    /// The prepared transaction must be submitted before this server time.
    pub submit_by_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultSetupMode {
    Create,
    ReplaceSession,
}

/// Session policy applied when onboarding does not state one: strategy timing
/// is unrestricted and the maximum price tolerance is 1%.
pub const PLATFORM_SESSION_DEFAULT_MINIMUM_INTERVAL_SECONDS: u32 = 0;
pub const PLATFORM_SESSION_DEFAULT_MAXIMUM_TOLERANCE_BPS: u16 = 100;
/// A session carries at most this many spending limits.
pub const PLATFORM_SESSION_MAX_SPENDING_LIMITS: usize = 4;

/// One-signature onboarding: only the wallet and the external session key are
/// required. One session then trades every market. Everything else is an
/// optional policy on top; absent values take the product defaults.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultSetupPrepareRequest {
    pub wallet_address: String,
    pub session_public_key: String,
    /// Optional old session key to revoke in the same transaction that
    /// registers `session_public_key`. If it is already absent, setup still
    /// succeeds. This makes local credential rotation one owner signature.
    #[serde(default)]
    pub replace_session_public_key: Option<String>,
    /// Optional. Names the market whose price protection the session pins
    /// when the product has one; the session trades every market either way.
    #[serde(default)]
    pub market_id: Option<String>,
    /// Null or absent requests the permanent-session expiry supported by the
    /// product.
    #[serde(default)]
    pub expires_at_ms: Option<u64>,
    /// Optional legacy hard cadence floor. Absent/zero leaves timing to the strategy.
    #[serde(default)]
    pub minimum_interval_seconds: Option<u32>,
    /// Absent takes `PLATFORM_SESSION_DEFAULT_MAXIMUM_TOLERANCE_BPS`.
    #[serde(default)]
    pub maximum_tolerance_bps: Option<u16>,
    /// Optional per-asset ceilings, at most `PLATFORM_SESSION_MAX_SPENDING_LIMITS`.
    /// Assets without a limit are unlimited.
    #[serde(default)]
    pub spending_limits: Vec<PlatformVaultSpendingLimit>,
}

/// Owner-bound onboarding or session-replacement transaction. Product inputs
/// are echoed exactly so an external verifier can reject changed intent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultSetupPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub session_public_key: String,
    /// The old session requested for atomic replacement, if any.
    pub replace_session_public_key: Option<String>,
    /// The market named in the request, if any.
    pub market_id: Option<String>,
    pub mode: PlatformVaultSetupMode,
    pub expires_at_ms: Option<u64>,
    pub permanent: bool,
    /// The applied policy, defaults resolved.
    pub minimum_interval_seconds: u32,
    pub maximum_tolerance_bps: u16,
    pub spending_limits: Vec<PlatformVaultSpendingLimit>,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub owner_signature_required: bool,
    /// Opaque handle for this prepared transaction. Hand it back with the
    /// owner-signed transaction to `vault.relay` and Strata submits it — no
    /// RPC or SOL needed on the owner side.
    pub preparation_id: String,
    /// `true` when Strata is the transaction fee payer and covers any rent the
    /// action creates, so the owner needs no SOL at all. `false` means the
    /// owner wallet is the fee payer (Strata still submits it on request).
    pub sponsored: bool,
    /// The prepared transaction must be submitted before this server time.
    pub submit_by_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultDelegateAction {
    Revoke,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultDelegatePrepareRequest {
    pub wallet_address: String,
    pub session_public_key: String,
    pub action: PlatformVaultDelegateAction,
}

/// Unsigned session-lifecycle control. The owner verifies both identities and
/// the destructive action before signing and broadcasting it externally.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultDelegatePrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub session_public_key: String,
    pub action: PlatformVaultDelegateAction,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub owner_signature_required: bool,
    /// Opaque handle for this prepared transaction. Hand it back with the
    /// owner-signed transaction to `vault.relay` and Strata submits it — no
    /// RPC or SOL needed on the owner side.
    pub preparation_id: String,
    /// `true` when Strata is the transaction fee payer and covers any rent the
    /// action creates, so the owner needs no SOL at all. `false` means the
    /// owner wallet is the fee payer (Strata still submits it on request).
    pub sponsored: bool,
    /// The prepared transaction must be submitted before this server time.
    pub submit_by_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultPolicyPrepareRequest {
    pub wallet_address: String,
    pub withdrawal_access: PlatformVaultWithdrawalAccess,
}

/// An owner-bound withdrawal-access change. Unrestricted access is a status
/// state rather than a preparable action in this contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultPolicyPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub withdrawal_access: PlatformVaultWithdrawalAccess,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub owner_signature_required: bool,
    /// Opaque handle for this prepared transaction. Hand it back with the
    /// owner-signed transaction to `vault.relay` and Strata submits it — no
    /// RPC or SOL needed on the owner side.
    pub preparation_id: String,
    /// `true` when Strata is the transaction fee payer and covers any rent the
    /// action creates, so the owner needs no SOL at all. `false` means the
    /// owner wallet is the fee payer (Strata still submits it on request).
    pub sponsored: bool,
    /// The prepared transaction must be submitted before this server time.
    pub submit_by_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultDepositPrepareRequest {
    pub wallet_address: String,
    pub market_id: String,
    pub asset_id: String,
    pub amount_atoms: String,
    /// Optional external session key. When it is not yet registered for this
    /// wallet, the same deposit transaction registers it with the default
    /// session policy — a first deposit is the whole onboarding, one owner
    /// signature. An already-registered key changes nothing.
    #[serde(default)]
    pub session_public_key: Option<String>,
}

/// Exact owner-funded deposit transaction. Asset construction and custody
/// identities remain internal; the public intent is echoed for verification.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultDepositPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub market_id: String,
    pub asset_id: String,
    pub amount_atoms: String,
    /// SOL Strata already spent on this owner's sponsored actions, recovered
    /// in the deposit asset inside this same transaction (a second transfer
    /// from the owner's account to Strata). "0" when nothing is owed. It is
    /// only ever charged when the owner had no SOL and Strata paid instead,
    /// and never exceeds 1% of the deposit.
    pub network_cost_atoms: String,
    /// The session key named in the request, if any.
    pub session_public_key: Option<String>,
    /// `true` when this transaction also registers `session_public_key` with
    /// the default session policy (the deposit doubles as onboarding);
    /// `false` when the key was already registered or none was named.
    pub registers_session: bool,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub owner_signature_required: bool,
    /// Opaque handle for this prepared transaction. Hand it back with the
    /// owner-signed transaction to `vault.relay` and Strata submits it — no
    /// RPC or SOL needed on the owner side.
    pub preparation_id: String,
    /// `true` when Strata is the transaction fee payer and covers any rent the
    /// action creates, so the owner needs no SOL at all. `false` means the
    /// owner wallet is the fee payer (Strata still submits it on request).
    pub sponsored: bool,
    /// The prepared transaction must be submitted before this server time.
    pub submit_by_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultWithdrawPrepareRequest {
    pub wallet_address: String,
    pub market_id: String,
    pub asset_id: String,
    pub destination_wallet_address: String,
    pub amount_atoms: String,
}

/// Exact owner-authorized withdrawal transaction. The destination is a wallet
/// identity; account construction and private balance routing remain internal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultWithdrawPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub market_id: String,
    pub asset_id: String,
    pub destination_wallet_address: String,
    pub amount_atoms: String,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub owner_signature_required: bool,
    /// Opaque handle for this prepared transaction. Hand it back with the
    /// owner-signed transaction to `vault.relay` and Strata submits it — no
    /// RPC or SOL needed on the owner side.
    pub preparation_id: String,
    /// `true` when Strata is the transaction fee payer and covers any rent the
    /// action creates, so the owner needs no SOL at all. `false` means the
    /// owner wallet is the fee payer (Strata still submits it on request).
    pub sponsored: bool,
    /// The prepared transaction must be submitted before this server time.
    pub submit_by_ms: u64,
}

/// Which prepared Vault action a submission carries.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultAction {
    Setup,
    Deposit,
    Withdraw,
    Delegate,
    Policy,
    Pause,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformVaultSubmissionStatus {
    /// Accepted by the cluster; confirmation pending.
    Submitted,
    Confirmed,
    Failed,
}

/// Submit an owner-signed prepared Vault transaction. Strata verifies it is
/// exactly the prepared transaction, adds its own fee-payer signature when
/// the preparation was sponsored, and broadcasts it. Idempotent per
/// `idempotency_key`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultSubmitRequest {
    pub preparation_id: String,
    pub signed_transaction_base64: String,
    pub idempotency_key: String,
}

/// Durable outcome of a Vault submission, also returned by the status read.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformVaultSubmitResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub preparation_id: String,
    pub action: PlatformVaultAction,
    pub wallet_address: String,
    pub sponsored: bool,
    pub signature: String,
    pub status: PlatformVaultSubmissionStatus,
    /// Present only when `status` is `failed`.
    pub failure_code: Option<String>,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformRewardStanding {
    pub rank: u32,
    pub wallet_address: String,
    pub points: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOwnerRewards {
    pub wallet_address: String,
    pub rank: Option<u32>,
    pub points: String,
    pub trading_points: String,
    pub making_points: String,
    pub bug_points: String,
    pub referral_points: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformRewardsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub season: String,
    pub total_wallets: u32,
    pub owner: Option<PlatformOwnerRewards>,
    pub standings: Vec<PlatformRewardStanding>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformReferralsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub enabled: bool,
    pub cash_rewards_enabled: bool,
    pub referral_code: Option<String>,
    pub referred_wallets: u32,
    pub referral_points: String,
    pub referred_by: Option<String>,
    pub referral_locked: bool,
    pub cash_accrued_atoms: String,
    pub cash_paid_atoms: String,
    pub cash_claimable_atoms: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformReferralLinkRequest {
    pub wallet_address: String,
    pub referral_code: String,
    pub authorization_signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformReferralLinkResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub referral_code: String,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformReferralClaimRequest {
    pub wallet_address: String,
    pub payout_wallet_address: Option<String>,
    pub authorization_signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformReferralClaimResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub payout_wallet_address: String,
    pub claimable_atoms: String,
    pub status: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformBugStatus {
    Pending,
    Confirmed,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBugReport {
    pub bug_id: String,
    pub status: PlatformBugStatus,
    pub severity: u8,
    pub points: String,
    pub created_at_ms: u64,
    pub triaged_at_ms: Option<u64>,
    pub completed_at_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBugsResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub wallet_address: String,
    pub points: String,
    pub confirmed_reports: u32,
    pub reports: Vec<PlatformBugReport>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBugSubmitRequest {
    pub owner_wallet: String,
    pub message: String,
    /// Hex Ed25519 signature over `strata-bug-report:v1:` followed by the
    /// trimmed report message. Signing always happens outside Strata.
    pub authorization_signature: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformBugSubmitResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub server_time_ms: u64,
    pub bug_id: String,
    pub status: PlatformBugStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTradeSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTrade {
    pub trade_id: String,
    pub side: PlatformTradeSide,
    pub price_atoms: String,
    pub size_atoms: String,
    pub executed_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformTradesResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub server_time_ms: u64,
    pub trades: Vec<PlatformTrade>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOrderType {
    GoodUntilCancelled,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}

/// Externally authorized resting-order operation. The public contract exposes
/// product intent only; private construction details never cross the SDK
/// boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOrderAction {
    Place,
    Cancel,
    CancelAll,
    /// Atomically cancel one existing order and place its explicitly bound
    /// successor in the same transaction.
    Replace,
    /// Atomically execute a bounded heterogeneous set of place, cancel, and
    /// replace operations in one transaction.
    Batch,
}

/// One operation inside an atomic order-control batch. Owner and session
/// identity live on the enclosing challenge so no item can widen authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderBatchOperation {
    Place {
        /// Vault market account sequence for this order. Omit it and Strata
        /// resolves the next sequence from the Vault's confirmed market
        /// account when the challenge is issued (consecutive places in one
        /// batch receive consecutive sequences); supply it to pin a sequence
        /// tracked locally. A batch must either supply every sequence or none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account_sequence: Option<String>,
        client_order_id: String,
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: String,
        size_atoms: String,
    },
    Cancel {
        order_id: String,
    },
    Replace {
        order_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account_sequence: Option<String>,
        client_order_id: String,
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: String,
        size_atoms: String,
    },
}

/// Request canonical bytes for one externally signed order-control operation.
/// Variant-specific fields are sealed so an authorization cannot be widened
/// between challenge and transaction preparation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderChallengeRequest {
    Place {
        owner_wallet: String,
        session_public_key: String,
        /// Vault market account sequence. Omit it and Strata resolves the next
        /// sequence from the Vault's confirmed market account when the
        /// challenge is issued; supply it to pin a sequence tracked locally.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account_sequence: Option<String>,
        client_order_id: String,
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: String,
        size_atoms: String,
    },
    Cancel {
        owner_wallet: String,
        session_public_key: String,
        order_id: String,
    },
    CancelAll {
        owner_wallet: String,
        session_public_key: String,
    },
    Replace {
        owner_wallet: String,
        session_public_key: String,
        order_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        account_sequence: Option<String>,
        client_order_id: String,
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: String,
        size_atoms: String,
    },
    Batch {
        owner_wallet: String,
        session_public_key: String,
        operations: Vec<PlatformOrderBatchOperation>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderChallengeResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub challenge_id: String,
    pub market_id: String,
    pub action: PlatformOrderAction,
    /// Exact opaque order set bound by the authorization. Replace returns the
    /// old then new ID. Batch flattens item IDs in request order, with replace
    /// contributing old then new. A batch contains at most six operations.
    pub order_ids: Vec<String>,
    pub authorization_payload_base64: String,
    pub server_time_ms: u64,
    pub expires_at_ms: u64,
}

/// A prepared challenge, signed: the two-step path.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderPrepareAuthorization {
    pub challenge_id: String,
    /// Base58 Ed25519 signature over `authorization_payload_base64`. Required
    /// over HTTP. Over the session-authenticated order command channel it may
    /// be omitted: the socket already proved the session and the challenge is
    /// bound to it, so the session signs only the transaction (one signature).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization_signature: Option<String>,
}

/// Prepare an order-control transaction. Either hand back a signed challenge
/// (`Authorized`, two signatures per action) or send the operation itself
/// (`Direct`, one signature per action): Strata builds the transaction from
/// the operation immediately and the session's signature over that
/// transaction is the whole authorization. The response is identical.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PlatformOrderPrepareRequest {
    Authorized(PlatformOrderPrepareAuthorization),
    Direct(PlatformOrderChallengeRequest),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub order_control_id: String,
    pub market_id: String,
    pub action: PlatformOrderAction,
    pub order_ids: Vec<String>,
    /// Backend-partially-signed Solana v0 transaction. The external session
    /// signer verifies and fills only its signature slot.
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderSubmitRequest {
    pub order_control_id: String,
    pub signed_transaction_base64: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOrderSubmissionStatus {
    Submitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderSubmitResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub order_control_id: String,
    pub market_id: String,
    pub action: PlatformOrderAction,
    pub order_ids: Vec<String>,
    pub signature: String,
    pub status: PlatformOrderSubmissionStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderStatusRequest {
    pub order_control_id: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOrderControlStatus {
    Submitting,
    Submitted,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderStatusResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub order_control_id: String,
    pub market_id: String,
    pub action: PlatformOrderAction,
    pub order_ids: Vec<String>,
    pub signature: String,
    pub status: PlatformOrderControlStatus,
    pub failure_code: Option<String>,
    pub updated_at_ms: u64,
}

/// Optional collision policy for an incoming order that would cross the
/// owner's own resting liquidity. Every mode still preserves Strata's matcher
/// and on-chain self-fill prohibition; active policies only control which
/// order is cancelled first.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformSelfTradePrevention {
    None,
    CancelTaker,
    CancelMaker,
    CancelBoth,
    SkipOwnLiquidity,
}

impl Default for PlatformSelfTradePrevention {
    fn default() -> Self {
        Self::None
    }
}

/// One command on the persistent order-control connection. Challenge results
/// may contain an effective request that differs from the requested one only
/// by the explicitly selected self-trade prevention transformation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommand {
    /// Authenticated non-trading round trip used for latency certification.
    Probe {
        nonce: String,
    },
    Challenge {
        request: PlatformOrderChallengeRequest,
        #[serde(default)]
        self_trade_prevention: PlatformSelfTradePrevention,
    },
    Prepare {
        request: PlatformOrderPrepareRequest,
    },
    Submit {
        request: PlatformOrderSubmitRequest,
    },
    Status {
        request: PlatformOrderStatusRequest,
    },
    DeadManArm {
        timeout_ms: u64,
        request: PlatformOrderSubmitRequest,
    },
    DeadManStatus,
    DeadManHeartbeat,
    DeadManDisarm,
}

/// Frames sent by an external agent. Authentication proves possession of the
/// declared session key; individual order authorizations and transactions keep
/// their existing exact external-signing boundaries. Authentication is a
/// singleton frame. After authentication, the transport accepts either one
/// command or a bounded array of commands; every command retains its own
/// request ID and contiguous sequence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommandClientFrame {
    Authenticate {
        owner_wallet: String,
        session_public_key: String,
        /// Base58 Ed25519 signature over the stream authentication payload.
        signature: String,
        /// Optional negotiated result framing. Omitted clients retain the
        /// complete-event array format.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        batch_format: Option<PlatformOrderCommandBatchFormat>,
    },
    Command {
        request_id: String,
        sequence: String,
        command: PlatformOrderCommand,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOrderCommandBatchFormat {
    CompactV1,
}

/// One result inside a compact event batch. Shared stream identity, time and
/// sequence metadata live on the enclosing frame; request correlation and
/// command-specific results remain independent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommandBatchEvent {
    ProbeResult {
        request_id: String,
        nonce: String,
    },
    ChallengeResult {
        request_id: String,
        self_trade_prevention: PlatformSelfTradePrevention,
        prevented_order_ids: Vec<String>,
        effective_request: PlatformOrderChallengeRequest,
        response: PlatformOrderChallengeResponse,
    },
    PrepareResult {
        request_id: String,
        response: PlatformOrderPrepareResponse,
    },
    SubmitResult {
        request_id: String,
        response: PlatformOrderSubmitResponse,
    },
    StatusResult {
        request_id: String,
        response: PlatformOrderStatusResponse,
    },
    DeadManResult {
        request_id: String,
        state: PlatformDeadManState,
    },
    CommandError {
        request_id: String,
        error: PublicOperationError,
    },
    Heartbeat,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommandServerFrame {
    EventBatch {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        first_sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        events: Vec<PlatformOrderCommandBatchEvent>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformDeadManStatus {
    Armed,
    Triggering,
    Triggered,
    Disarmed,
    Expired,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformDeadManState {
    pub status: PlatformDeadManStatus,
    pub timeout_ms: u64,
    pub heartbeat_deadline_ms: u64,
    pub order_control_id: Option<String>,
    pub signature: Option<String>,
    pub failure_code: Option<String>,
    pub updated_at_ms: u64,
}

/// One sequenced event emitted by the persistent order-control connection.
/// After authentication, the transport carries bounded arrays of these events
/// so concurrent results share frame overhead without weakening per-event
/// sequence or request correlation. Terminal chain status may arrive later
/// without blocking command submission.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommandEvent {
    AuthChallenge {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        challenge: String,
        server_time_ms: u64,
        expires_at_ms: u64,
    },
    Ready {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
    },
    ProbeResult {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        nonce: String,
        server_time_ms: u64,
    },
    ChallengeResult {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        self_trade_prevention: PlatformSelfTradePrevention,
        prevented_order_ids: Vec<String>,
        effective_request: PlatformOrderChallengeRequest,
        response: PlatformOrderChallengeResponse,
        server_time_ms: u64,
    },
    PrepareResult {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        response: PlatformOrderPrepareResponse,
        server_time_ms: u64,
    },
    SubmitResult {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        response: PlatformOrderSubmitResponse,
        server_time_ms: u64,
    },
    StatusResult {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        response: PlatformOrderStatusResponse,
        server_time_ms: u64,
    },
    DeadManResult {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        state: PlatformDeadManState,
        server_time_ms: u64,
    },
    CommandError {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        request_id: String,
        error: PublicOperationError,
        server_time_ms: u64,
    },
    Heartbeat {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAccountOrder {
    pub order_id: String,
    pub side: PlatformTradeSide,
    pub order_type: PlatformOrderType,
    pub state: PlatformOrderState,
    pub limit_price_atoms: String,
    pub original_size_atoms: String,
    pub remaining_size_atoms: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAccountFill {
    pub fill_id: String,
    pub side: PlatformTradeSide,
    pub price_atoms: String,
    pub size_atoms: String,
    pub fee_quote_atoms: String,
    pub fee_is_final: bool,
    pub settlement: PlatformSettlementState,
    pub executed_at_ms: u64,
    pub confirmed_at_ms: Option<u64>,
    pub transaction_id: Option<String>,
    pub realized_pnl_quote_atoms: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformAccountSnapshotResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub wallet_address: String,
    pub server_time_ms: u64,
    pub orders: Vec<PlatformAccountOrder>,
    pub fills: Vec<PlatformAccountFill>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerReputationTier {
    Probation,
    Bronze,
    Silver,
    Gold,
    Platinum,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerTierProgress {
    pub next_tier: Option<PlatformMakerReputationTier>,
    pub reputation_score_required: Option<u16>,
    pub reputation_score_remaining: u16,
    pub quote_requests_required: Option<String>,
    pub quote_requests_remaining: String,
    pub stake_atoms_required: Option<String>,
    pub stake_atoms_remaining: String,
    pub tenure_slots_required: Option<String>,
    pub tenure_slots_remaining: String,
}

/// Authenticated, privacy-preserving reliability and participation record for the
/// requesting maker. All potentially large counters and atomic quantities are
/// decimal strings so JavaScript agents never lose integer precision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerReputationResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub maker_id: String,
    pub wallet_address: String,
    pub active: bool,
    pub tier: PlatformMakerReputationTier,
    pub reputation_score: u16,
    pub total_quote_requests: String,
    pub successful_fills: String,
    pub missed_quote_requests: String,
    pub fill_rate_bps: u16,
    pub consecutive_misses: u16,
    pub lifetime_filled_quote_atoms: String,
    pub distinct_counterparties: u16,
    pub recent_average_latency_ms: u16,
    pub configured_minimum_spread_bps: u16,
    pub weighted_average_spread_bps: u16,
    pub stake_atoms: String,
    pub epoch_start_stake_atoms: String,
    pub epoch_slashed_atoms: String,
    pub epoch_slashed_bps: u16,
    pub lifetime_auto_slashed_atoms: String,
    pub registered_slot: String,
    pub last_active_slot: String,
    pub last_settled_slot: String,
    pub revoked_at_slot: Option<String>,
    pub tenure_slots: String,
    pub signed_quote_stream_eligible: bool,
    pub minimum_quote_interval_ms: Option<u16>,
    pub tier_progress: PlatformMakerTierProgress,
    pub server_time_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerSide {
    Buy,
    Sell,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformOracleHealth {
    Fresh,
    Stale,
    Unknown,
}

/// The maker's resting firm orders in this market, summarised by side.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerFirmOrderSummary {
    pub resting_orders: u32,
    pub bid_orders: u32,
    pub ask_orders: u32,
    pub bid_size_atoms: String,
    pub ask_size_atoms: String,
}

/// One of the maker's own live signed quotes in the streaming lane.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerSignedQuote {
    pub side: PlatformMakerSide,
    pub price_atoms: String,
    pub size_atoms: String,
    pub nonce: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

/// The maker's own intent product in this market.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerIntentStatus {
    pub active: bool,
    pub side: PlatformMakerIntentSide,
    pub minimum_price_atoms: String,
    pub maximum_price_atoms: String,
    pub maximum_fill_size_atoms: String,
    /// Fill budget still available after in-flight reservations.
    pub remaining_fill_size_atoms: String,
    pub minimum_spread_bps: u16,
    pub stake_atoms: String,
}

/// The maker's signed-quote lane: eligibility and its own live quotes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerSignedQuoteLane {
    pub eligible: bool,
    pub live_quotes: Vec<PlatformMakerSignedQuote>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerStrandLevel {
    /// Null when the configured offset overflows the price range.
    pub price_atoms: Option<String>,
    pub size_atoms: String,
    pub remaining_size_atoms: String,
}

/// One of the maker's own Strands in this market.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerStrandStatus {
    pub enabled: bool,
    pub async_only: bool,
    /// True once the chain would reject fills because `valid_until_slot` passed.
    pub expired: bool,
    pub mid_price_atoms: String,
    pub tick_size_atoms: String,
    /// Null means the Strand never expires.
    pub valid_until_slot: Option<String>,
    pub bids: Vec<PlatformMakerStrandLevel>,
    pub asks: Vec<PlatformMakerStrandLevel>,
    pub maximum_exposure_atoms: String,
    pub remaining_exposure_atoms: String,
}

/// One of the maker's own Currents in this market.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerCurrentStatus {
    pub enabled: bool,
    pub async_only: bool,
    pub expired: bool,
    pub half_spread_bps: u16,
    pub band_step_bps: u16,
    pub maximum_confidence_bps: u16,
    pub maximum_oracle_age_seconds: u32,
    pub sync_spread_bps: u16,
    /// Null means the Current never expires.
    pub valid_until_slot: Option<String>,
    pub bid_depth_atoms: Vec<String>,
    pub ask_depth_atoms: Vec<String>,
    pub maximum_exposure_atoms: String,
    pub remaining_exposure_atoms: String,
    /// Freshness class of the live Strata mark used to price this Current.
    pub oracle_health: PlatformOracleHealth,
}

/// One durable dead-man guard the owner armed for a session in this market.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerDeadManGuard {
    pub session_public_key: String,
    pub status: PlatformDeadManStatus,
    pub timeout_ms: u64,
    pub heartbeat_deadline_ms: u64,
    pub updated_at_ms: u64,
}

/// Authenticated, owner-scoped view of the maker's Strata products in one
/// market: firm orders, intent, Strands, Currents, the signed-quote lane, live
/// exposure, health, and kill state. Nothing about other makers, takers, or
/// liquidity sources crosses this boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerStatusResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub maker_id: String,
    pub wallet_address: String,
    pub server_time_ms: u64,
    pub current_slot: String,
    pub firm_orders: PlatformMakerFirmOrderSummary,
    pub intent: Option<PlatformMakerIntentStatus>,
    pub signed_quotes: PlatformMakerSignedQuoteLane,
    pub strands: Vec<PlatformMakerStrandStatus>,
    pub currents: Vec<PlatformMakerCurrentStatus>,
    pub dead_man_guards: Vec<PlatformMakerDeadManGuard>,
    /// Count of maker products currently able to fill: an active intent, each
    /// enabled unexpired Strand or Current, and resting firm orders (as one).
    pub active_products: u16,
}

/// One maker-owned Strand mutation. Amounts that may exceed JavaScript's safe
/// integer range remain canonical unsigned decimal strings on the wire.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformMakerStrandPrepareRequest {
    Upsert {
        maker_wallet: String,
        enabled: bool,
        async_only: bool,
        sync_spread_ticks: u16,
        mid_price_atoms: String,
        #[serde(alias = "max_exposure_base_lots")]
        max_exposure_base_atoms: String,
        bid_offsets_ticks: Vec<u16>,
        ask_offsets_ticks: Vec<u16>,
        #[serde(alias = "bid_sizes_base_lots")]
        bid_sizes_base_atoms: Vec<String>,
        #[serde(alias = "ask_sizes_base_lots")]
        ask_sizes_base_atoms: Vec<String>,
        valid_until_slot: String,
    },
    Recenter {
        maker_wallet: String,
        new_mid_price_atoms: String,
        valid_until_slot: String,
    },
    SetEnabled {
        maker_wallet: String,
        enabled: bool,
    },
    Cancel {
        maker_wallet: String,
    },
}

/// One maker-owned Current mutation. Current is parameterized around the
/// market's live Strata mark and therefore has no recenter action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformMakerCurrentPrepareRequest {
    Upsert {
        maker_wallet: String,
        enabled: bool,
        async_only: bool,
        half_spread_bps: u16,
        band_step_bps: u16,
        max_conf_bps: u16,
        max_oracle_dev_bps: u16,
        max_oracle_age_secs: u32,
        sync_spread_bps: u16,
        max_exposure_base_atoms: String,
        bid_depth_base_atoms: Vec<String>,
        ask_depth_base_atoms: Vec<String>,
        valid_until_slot: String,
    },
    Cancel {
        maker_wallet: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerControlProduct {
    Strand,
    Current,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerControlAction {
    StrandUpsert,
    StrandRecenter,
    StrandSetEnabled,
    StrandCancel,
    CurrentUpsert,
    CurrentCancel,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerControlPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub maker_control_id: String,
    pub market_id: String,
    pub maker_wallet: String,
    pub product: PlatformMakerControlProduct,
    pub action: PlatformMakerControlAction,
    /// Unsigned Solana transaction in the format negotiated by the prepare
    /// endpoint. The maker verifies the exact instruction and fills its only
    /// signature slot externally.
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerControlSubmitRequest {
    pub maker_control_id: String,
    pub signed_transaction_base64: String,
    pub idempotency_key: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerControlSubmissionStatus {
    Submitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerControlSubmitResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub maker_control_id: String,
    pub market_id: String,
    pub maker_wallet: String,
    pub product: PlatformMakerControlProduct,
    pub action: PlatformMakerControlAction,
    pub signature: String,
    pub status: PlatformMakerControlSubmissionStatus,
}

/// Side exposed by the existing on-chain IntentRecord. `Both` commits the
/// same maker seat on both sides and is cap-checked against both assets.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerIntentSide {
    Buy,
    Sell,
    Both,
}

/// Vault-session control of an already admin-registered IntentRecord. This
/// does not create a new intent product or registration mechanism: it exposes
/// the existing post/revoke lifecycle through the owner's approved session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformMakerIntentPrepareRequest {
    Post {
        market_id: String,
        owner_wallet: String,
        session_public_key: String,
        side: PlatformMakerIntentSide,
        min_price_atoms: String,
        max_price_atoms: String,
        max_fill_size_atoms: String,
    },
    Revoke {
        market_id: String,
        owner_wallet: String,
        session_public_key: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerIntentAction {
    Post,
    Revoke,
}

/// Canonical sponsored Vault transaction. The external session verifies the
/// echoed bindings and fills only its signature slot; the owner wallet does
/// not sign each intent update.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerIntentPrepareResponse {
    pub schema_version: u16,
    pub contract_version: String,
    pub market_id: String,
    pub owner_wallet: String,
    pub vault_address: String,
    pub session_public_key: String,
    pub intent_address: String,
    pub action: PlatformMakerIntentAction,
    pub transaction_base64: String,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
    pub expires_at_ms: u64,
    /// Strata is the fee payer. The confirmed network cost is recorded for
    /// bounded recovery from a later owner deposit.
    pub sponsored: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerIntentSubmitRequest {
    pub signed_transaction_base64: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerIntentSubmitResponse {
    pub signature: String,
}

/// Which Strata maker product produced a maker-side fill.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMakerProduct {
    FirmOrder,
    Intent,
    Strand,
    Current,
}

/// One maker-side fill: the same sanitized settlement view as an account fill
/// plus the maker product that produced it. No counterparty or venue.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformMakerFill {
    pub fill_id: String,
    pub product: PlatformMakerProduct,
    pub side: PlatformTradeSide,
    pub price_atoms: String,
    pub size_atoms: String,
    pub fee_quote_atoms: String,
    pub fee_is_final: bool,
    pub settlement: PlatformSettlementState,
    pub executed_at_ms: u64,
    pub confirmed_at_ms: Option<u64>,
    pub transaction_id: Option<String>,
    pub realized_pnl_quote_atoms: String,
}

/// Authenticated, sequenced owner-only maker stream (`mm.fills.stream`).
/// After the signed challenge the server sends one `maker_snapshot`, then
/// sequenced `maker_fill`, `maker_status` (exposure/product change), and
/// `heartbeat` events; a recovery snapshot advances the sequence on the same
/// stream identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformMakerEvent {
    AuthChallenge {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        challenge: String,
        server_time_ms: u64,
        expires_at_ms: u64,
    },
    MakerSnapshot {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
        status: PlatformMakerStatusResponse,
        fills: Vec<PlatformMakerFill>,
    },
    MakerFill {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        fill: PlatformMakerFill,
    },
    MakerStatus {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        status: PlatformMakerStatusResponse,
    },
    Heartbeat {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformAccountEvent {
    AuthChallenge {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        challenge: String,
        server_time_ms: u64,
        expires_at_ms: u64,
    },
    AccountSnapshot {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
        orders: Vec<PlatformAccountOrder>,
        fills: Vec<PlatformAccountFill>,
    },
    OrdersSnapshot {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        orders: Vec<PlatformAccountOrder>,
    },
    Fill {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        fill: PlatformAccountFill,
    },
    Heartbeat {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        wallet_address: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformMarketDataEvent {
    BookSnapshot {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
        snapshot_id: String,
        bids: Vec<PlatformBookLevel>,
        asks: Vec<PlatformBookLevel>,
    },
    BookDelta {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        previous_sequence: String,
        server_time_ms: u64,
        changes: Vec<PlatformBookChange>,
    },
    BestBidAsk {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        stream_id: String,
        sequence: String,
        server_time_ms: u64,
        best_bid: Option<PlatformBookLevel>,
        best_ask: Option<PlatformBookLevel>,
    },
    Trade {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        server_time_ms: u64,
        trade: PlatformTrade,
    },
    MarketStatus {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        server_time_ms: u64,
        status: PlatformMarketState,
    },
    Heartbeat {
        schema_version: u16,
        contract_version: String,
        market_id: String,
        server_time_ms: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_platform_fixtures_decode_strictly() {
        let discovery: PlatformDiscoveryResponse =
            serde_json::from_str(PLATFORM_CAPABILITIES_FIXTURE).unwrap();
        let service_status: PlatformServiceStatusResponse =
            serde_json::from_str(PLATFORM_SERVICE_STATUS_FIXTURE).unwrap();
        let graph = PlatformActionGraphResponse::foundation();
        let assets: PlatformAssetsResponse = serde_json::from_str(PLATFORM_ASSETS_FIXTURE).unwrap();
        let swap_quote: PlatformSwapQuoteResponse =
            serde_json::from_str(PLATFORM_SWAP_QUOTE_FIXTURE).unwrap();
        let markets: PlatformMarketsResponse =
            serde_json::from_str(PLATFORM_MARKETS_FIXTURE).unwrap();
        let book: PlatformBookSnapshotResponse =
            serde_json::from_str(PLATFORM_BOOK_FIXTURE).unwrap();
        let bbo: PlatformBestBidAskResponse = serde_json::from_str(PLATFORM_BBO_FIXTURE).unwrap();
        let fees: PlatformFeeScheduleResponse =
            serde_json::from_str(PLATFORM_FEES_FIXTURE).unwrap();
        let status: PlatformMarketStatusResponse =
            serde_json::from_str(PLATFORM_STATUS_FIXTURE).unwrap();
        let candles: PlatformCandlesResponse =
            serde_json::from_str(PLATFORM_CANDLES_FIXTURE).unwrap();
        let mark: PlatformMarkResponse = serde_json::from_str(PLATFORM_MARK_FIXTURE).unwrap();
        let execution_status: PlatformExecutionStatusResponse =
            serde_json::from_str(PLATFORM_EXECUTION_STATUS_FIXTURE).unwrap();
        let twaps: PlatformTwapsResponse = serde_json::from_str(PLATFORM_TWAPS_FIXTURE).unwrap();
        let twap_challenge: PlatformTwapChallengeResponse =
            serde_json::from_str(PLATFORM_TWAP_CHALLENGE_FIXTURE).unwrap();
        let twap_prepare: PlatformTwapPrepareResponse =
            serde_json::from_str(PLATFORM_TWAP_PREPARE_FIXTURE).unwrap();
        let twap_submit: PlatformTwapSubmitResponse =
            serde_json::from_str(PLATFORM_TWAP_SUBMIT_FIXTURE).unwrap();
        let portfolio_history: PlatformPortfolioHistoryResponse =
            serde_json::from_str(PLATFORM_PORTFOLIO_HISTORY_FIXTURE).unwrap();
        let portfolio: PlatformPortfolioResponse =
            serde_json::from_str(PLATFORM_PORTFOLIO_FIXTURE).unwrap();
        let rewards: PlatformRewardsResponse =
            serde_json::from_str(PLATFORM_REWARDS_FIXTURE).unwrap();
        let referrals: PlatformReferralsResponse =
            serde_json::from_str(PLATFORM_REFERRALS_FIXTURE).unwrap();
        let referral_link: PlatformReferralLinkResponse =
            serde_json::from_str(PLATFORM_REFERRAL_LINK_FIXTURE).unwrap();
        let referral_claim: PlatformReferralClaimResponse =
            serde_json::from_str(PLATFORM_REFERRAL_CLAIM_FIXTURE).unwrap();
        let vault_status: PlatformVaultStatusResponse =
            serde_json::from_str(PLATFORM_VAULT_STATUS_FIXTURE).unwrap();
        let vault_pause: PlatformVaultPausePrepareResponse =
            serde_json::from_str(PLATFORM_VAULT_PAUSE_PREPARE_FIXTURE).unwrap();
        let vault_setup: PlatformVaultSetupPrepareResponse =
            serde_json::from_str(PLATFORM_VAULT_SETUP_PREPARE_FIXTURE).unwrap();
        let vault_delegate: PlatformVaultDelegatePrepareResponse =
            serde_json::from_str(PLATFORM_VAULT_DELEGATE_PREPARE_FIXTURE).unwrap();
        let vault_policy: PlatformVaultPolicyPrepareResponse =
            serde_json::from_str(PLATFORM_VAULT_POLICY_PREPARE_FIXTURE).unwrap();
        let vault_deposit: PlatformVaultDepositPrepareResponse =
            serde_json::from_str(PLATFORM_VAULT_DEPOSIT_PREPARE_FIXTURE).unwrap();
        let vault_withdraw: PlatformVaultWithdrawPrepareResponse =
            serde_json::from_str(PLATFORM_VAULT_WITHDRAW_PREPARE_FIXTURE).unwrap();
        let vault_submit: PlatformVaultSubmitResponse =
            serde_json::from_str(PLATFORM_VAULT_SUBMIT_FIXTURE).unwrap();
        let bugs: PlatformBugsResponse = serde_json::from_str(PLATFORM_BUGS_FIXTURE).unwrap();
        let bug_submit: PlatformBugSubmitResponse =
            serde_json::from_str(PLATFORM_BUG_SUBMIT_FIXTURE).unwrap();
        let trades: PlatformTradesResponse = serde_json::from_str(PLATFORM_TRADES_FIXTURE).unwrap();
        let account: PlatformAccountSnapshotResponse =
            serde_json::from_str(PLATFORM_ACCOUNT_FIXTURE).unwrap();
        let maker_reputation: PlatformMakerReputationResponse =
            serde_json::from_str(PLATFORM_MAKER_REPUTATION_FIXTURE).unwrap();
        let maker_status: PlatformMakerStatusResponse =
            serde_json::from_str(PLATFORM_MAKER_STATUS_FIXTURE).unwrap();
        let maker_stream: PlatformMakerEvent =
            serde_json::from_str(PLATFORM_MAKER_STREAM_FIXTURE).unwrap();
        let twap_stream: PlatformTwapEvent =
            serde_json::from_str(PLATFORM_TWAP_STREAM_FIXTURE).unwrap();
        let execution_stream: PlatformExecutionEvent =
            serde_json::from_str(PLATFORM_EXECUTION_STREAM_FIXTURE).unwrap();
        let order_challenge: PlatformOrderChallengeResponse =
            serde_json::from_str(PLATFORM_ORDER_CHALLENGE_FIXTURE).unwrap();
        let order_prepare: PlatformOrderPrepareResponse =
            serde_json::from_str(PLATFORM_ORDER_PREPARE_FIXTURE).unwrap();
        let order_submit: PlatformOrderSubmitResponse =
            serde_json::from_str(PLATFORM_ORDER_SUBMIT_FIXTURE).unwrap();
        let order_status: PlatformOrderStatusResponse =
            serde_json::from_str(PLATFORM_ORDER_STATUS_FIXTURE).unwrap();

        assert_eq!(discovery.schema_version, PLATFORM_SCHEMA_VERSION);
        assert_eq!(service_status.status, PlatformServiceState::Operational);
        assert_eq!(service_status.available_operations, 59);
        assert_eq!(graph.entry_operation_id, "platform.capabilities.read");
        assert_eq!(graph.operations.len(), 70);
        assert_eq!(maker_reputation.tier, PlatformMakerReputationTier::Gold);
        assert_eq!(maker_status.active_products, 3);
        match &maker_stream {
            PlatformMakerEvent::MakerSnapshot { status, fills, .. } => {
                assert_eq!(status.active_products, maker_status.active_products);
                assert_eq!(fills.len(), 1);
                assert_eq!(fills[0].product, PlatformMakerProduct::Strand);
            }
            other => panic!("maker stream fixture must be a snapshot, got {other:?}"),
        }
        match &twap_stream {
            PlatformTwapEvent::TwapsSnapshot {
                twaps: streamed, ..
            } => {
                assert_eq!(streamed, &twaps.twaps);
            }
            other => panic!("twap stream fixture must be a snapshot, got {other:?}"),
        }
        match &execution_stream {
            PlatformExecutionEvent::ExecutionsSnapshot {
                executions,
                unknown_execution_ids,
                ..
            } => {
                assert_eq!(executions.len(), 2);
                assert_eq!(executions[0].execution_id, execution_status.execution_id);
                assert_eq!(unknown_execution_ids.len(), 1);
            }
            other => panic!("execution stream fixture must be a snapshot, got {other:?}"),
        }
        assert_eq!(maker_status.strands.len(), 1);
        assert_eq!(maker_status.currents.len(), 1);
        assert!(maker_status
            .intent
            .as_ref()
            .is_some_and(|intent| intent.active));
        assert_eq!(portfolio.balances.len(), 2);
        assert_eq!(portfolio.positions.len(), 1);
        assert!(portfolio.valuation_complete);
        assert_eq!(portfolio.equity_usd_micros.as_deref(), Some("439989500"));
        assert!(graph
            .operations
            .iter()
            .any(|operation| operation.id == "twap.place.submit"));
        assert!(graph
            .operations
            .iter()
            .any(|operation| operation.id == "twap.cancel.submit"));
        assert_eq!(discovery.capabilities.len(), 5);
        assert!(!discovery.authority.accepts_private_keys);
        assert_eq!(assets.assets.len(), 2);
        assert_eq!(swap_quote.input_asset_id, assets.assets[0].asset_id);
        assert_eq!(swap_quote.output_asset_id, assets.assets[1].asset_id);
        assert_eq!(markets.markets.len(), 1);
        assert_eq!(markets.markets[0].base_asset_id, assets.assets[0].asset_id);
        assert_eq!(markets.markets[0].quote_asset_id, assets.assets[1].asset_id);
        assert_eq!(book.sequence, "42");
        assert_eq!(bbo.best_bid.unwrap().price_atoms, "149990000");
        assert_eq!(fees.maximum_immediate_execution_fee_bps, 10);
        assert_eq!(status.status, PlatformMarketState::Active);
        assert_eq!(candles.candles.len(), 2);
        assert_eq!(mark.price_atoms_per_base_unit.as_deref(), Some("149995000"));
        assert_eq!(execution_status.status, PlatformExecutionState::Confirmed);
        assert_eq!(
            execution_status.settlement,
            PlatformSettlementState::Confirmed
        );
        assert_eq!(twaps.twaps[0].fills.len(), 1);
        assert_eq!(twaps.twaps[0].slices_executed, 2);
        assert_eq!(twap_challenge.action, PlatformTwapControlAction::Place);
        assert_eq!(twap_prepare.twap_id, twap_challenge.twap_id);
        assert_eq!(twap_submit.twap_control_id, twap_prepare.twap_control_id);
        assert_eq!(portfolio_history.points.len(), 2);
        assert_eq!(rewards.standings.len(), 2);
        assert!(referrals.enabled);
        assert_eq!(referral_link.status, "pending_first_fill");
        assert_eq!(referral_claim.status, "requested");
        assert_eq!(vault_status.state, PlatformVaultState::Active);
        assert_eq!(
            vault_status.session.as_ref().unwrap().state,
            PlatformVaultSessionState::Active
        );
        assert!(vault_pause.paused);
        assert!(vault_pause.owner_signature_required);
        assert_eq!(vault_setup.mode, PlatformVaultSetupMode::Create);
        assert!(vault_setup.permanent);
        assert_eq!(vault_delegate.action, PlatformVaultDelegateAction::Revoke);
        assert!(vault_delegate.owner_signature_required);
        assert_eq!(
            vault_policy.withdrawal_access.mode,
            PlatformVaultWithdrawalMode::Restricted
        );
        assert!(vault_policy.owner_signature_required);
        assert_eq!(vault_deposit.amount_atoms, "10000000");
        assert!(vault_deposit.owner_signature_required);
        assert_eq!(vault_withdraw.amount_atoms, "5000000");
        assert!(vault_withdraw.owner_signature_required);
        assert!(vault_withdraw.sponsored);
        assert!(vault_withdraw.preparation_id.starts_with("vp_"));
        assert_eq!(vault_submit.action, PlatformVaultAction::Deposit);
        assert_eq!(
            vault_submit.status,
            PlatformVaultSubmissionStatus::Submitted
        );
        assert!(vault_submit.sponsored);
        assert_eq!(vault_submit.failure_code, None);
        assert_eq!(bugs.reports[0].status, PlatformBugStatus::Confirmed);
        assert_eq!(bug_submit.status, PlatformBugStatus::Pending);
        assert_eq!(trades.trades.len(), 1);
        assert_eq!(account.orders.len(), 1);
        assert_eq!(account.fills.len(), 1);
        assert_eq!(order_challenge.action, PlatformOrderAction::Place);
        assert_eq!(order_prepare.order_ids, order_challenge.order_ids);
        assert_eq!(order_submit.order_ids, order_challenge.order_ids);
        assert_eq!(order_status.order_control_id, order_submit.order_control_id);
        assert_eq!(order_status.status, PlatformOrderControlStatus::Submitting);
    }

    #[test]
    fn public_platform_response_rejects_unreviewed_fields() {
        let mut value: serde_json::Value =
            serde_json::from_str(PLATFORM_CAPABILITIES_FIXTURE).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected_field".to_owned(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<PlatformDiscoveryResponse>(value).is_err());

        let mut account_event: serde_json::Value =
            serde_json::from_str(PLATFORM_ACCOUNT_FIXTURE).unwrap();
        let event = account_event.as_object_mut().unwrap();
        event.insert("type".to_owned(), serde_json::json!("account_snapshot"));
        event.insert(
            "stream_id".to_owned(),
            serde_json::json!("account_stream_66666666666666666666666666666666"),
        );
        event.insert("sequence".to_owned(), serde_json::json!("1"));
        event.insert("unexpected_field".to_owned(), serde_json::json!(true));
        assert!(serde_json::from_value::<PlatformAccountEvent>(account_event).is_err());
    }

    #[test]
    fn platform_graph_availability_is_projected_from_live_capabilities() {
        let mut graph = PlatformActionGraphResponse::foundation();
        let live = std::collections::BTreeSet::from([
            "platform.discover".to_owned(),
            "graphs.read".to_owned(),
            "orders.replace".to_owned(),
        ]);

        graph.project_availability(&live);

        for operation in &graph.operations {
            assert_eq!(
                operation.available,
                live.contains(&operation.capability_id),
                "operation {} did not follow capability {}",
                operation.id,
                operation.capability_id,
            );
        }
        assert!(graph
            .workflows
            .iter()
            .flat_map(|workflow| &workflow.nodes)
            .filter(|node| node.kind != PlatformActionKind::ExternalSignature)
            .all(|node| {
                node.available
                    == node
                        .capability_id
                        .as_ref()
                        .is_some_and(|capability_id| live.contains(capability_id))
            }));
        assert!(graph
            .workflows
            .iter()
            .flat_map(|workflow| &workflow.nodes)
            .filter(|node| node.kind == PlatformActionKind::ExternalSignature)
            .all(|node| node.available));
    }

    #[test]
    fn atomic_order_batch_request_is_strict_and_typed() {
        let request: PlatformOrderChallengeRequest = serde_json::from_value(serde_json::json!({
            "action": "batch",
            "owner_wallet": "11111111111111111111111111111111",
            "session_public_key": "22222222222222222222222222222222",
            "operations": [
                {
                    "action": "cancel",
                    "order_id": "order_11111111111111111111111111111111"
                },
                {
                    "action": "replace",
                    "order_id": "order_22222222222222222222222222222222",
                    "account_sequence": "8",
                    "client_order_id": "replacement-8",
                    "side": "sell",
                    "order_type": "post_only",
                    "limit_price_atoms": "151000000",
                    "size_atoms": "2000000"
                }
            ]
        }))
        .unwrap();
        let PlatformOrderChallengeRequest::Batch { operations, .. } = request else {
            panic!("expected batch request");
        };
        assert_eq!(operations.len(), 2);
        assert!(matches!(
            &operations[1],
            PlatformOrderBatchOperation::Replace { account_sequence: Some(sequence), .. }
                if sequence == "8"
        ));

        // The account sequence is optional: Strata resolves it from the Vault's
        // confirmed market account when omitted, and omitted stays omitted on
        // the wire so older servers reject rather than misread it.
        let place: PlatformOrderChallengeRequest = serde_json::from_value(serde_json::json!({
            "action": "place",
            "owner_wallet": "11111111111111111111111111111111",
            "session_public_key": "22222222222222222222222222222222",
            "client_order_id": "first-order",
            "side": "buy",
            "order_type": "post_only",
            "limit_price_atoms": "150000000",
            "size_atoms": "1000000"
        }))
        .unwrap();
        assert!(matches!(
            place,
            PlatformOrderChallengeRequest::Place {
                account_sequence: None,
                ..
            }
        ));
        assert!(!serde_json::to_string(&place)
            .unwrap()
            .contains("account_sequence"));

        assert!(
            serde_json::from_value::<PlatformOrderChallengeRequest>(serde_json::json!({
                "action": "batch",
                "owner_wallet": "11111111111111111111111111111111",
                "session_public_key": "22222222222222222222222222222222",
                "operations": [{
                    "action": "cancel",
                    "order_id": "order_11111111111111111111111111111111",
                    "implementation": "hidden"
                }]
            }))
            .is_err()
        );
    }

    #[test]
    fn persistent_order_commands_default_to_no_self_trade_policy() {
        let frame: PlatformOrderCommandClientFrame = serde_json::from_value(serde_json::json!({
            "type": "command",
            "request_id": "agent-1",
            "sequence": "1",
            "command": {
                "type": "challenge",
                "self_trade_prevention": "cancel_maker",
                "request": {
                    "action": "cancel_all",
                    "owner_wallet": "11111111111111111111111111111111",
                    "session_public_key": "22222222222222222222222222222222"
                }
            }
        }))
        .unwrap();
        assert!(matches!(
            frame,
            PlatformOrderCommandClientFrame::Command {
                command: PlatformOrderCommand::Challenge {
                    self_trade_prevention: PlatformSelfTradePrevention::CancelMaker,
                    ..
                },
                ..
            }
        ));
        let default_frame: PlatformOrderCommandClientFrame =
            serde_json::from_value(serde_json::json!({
                "type": "command",
                "request_id": "agent-1",
                "sequence": "1",
                "command": {
                    "type": "challenge",
                    "request": {
                        "action": "cancel_all",
                        "owner_wallet": "11111111111111111111111111111111",
                        "session_public_key": "22222222222222222222222222222222"
                    }
                }
            }))
            .unwrap();
        assert!(matches!(
            default_frame,
            PlatformOrderCommandClientFrame::Command {
                command: PlatformOrderCommand::Challenge {
                    self_trade_prevention: PlatformSelfTradePrevention::None,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn prepare_requests_accept_a_signed_challenge_or_the_operation_itself() {
        let signed: PlatformOrderPrepareRequest = serde_json::from_value(serde_json::json!({
            "challenge_id": "oc_0123456789abcdef0123456789abcdef",
            "authorization_signature": "1111",
        }))
        .unwrap();
        assert!(matches!(signed, PlatformOrderPrepareRequest::Authorized(_)));
        let direct: PlatformOrderPrepareRequest = serde_json::from_value(serde_json::json!({
            "action": "cancel_all",
            "owner_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
            "session_public_key": "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
        }))
        .unwrap();
        assert!(matches!(
            direct,
            PlatformOrderPrepareRequest::Direct(PlatformOrderChallengeRequest::CancelAll { .. })
        ));
        // Neither shape tolerates a stray field.
        assert!(
            serde_json::from_value::<PlatformOrderPrepareRequest>(serde_json::json!({
                "challenge_id": "oc_0123456789abcdef0123456789abcdef",
                "authorization_signature": "1111",
                "extra": true,
            }))
            .is_err()
        );
        let twap: PlatformTwapPrepareRequest = serde_json::from_value(serde_json::json!({
            "action": "cancel",
            "owner_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
            "session_public_key": "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
            "twap_id": "twap_0123456789abcdef0123456789abcdef",
        }))
        .unwrap();
        assert!(matches!(twap, PlatformTwapPrepareRequest::Direct(_)));
        let execution: crate::ExecutionPrepareRequest = serde_json::from_value(serde_json::json!({
            "quote_id": "quote_0123456789abcdef0123456789abcdef",
            "owner_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
            "session_public_key": "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
        }))
        .unwrap();
        assert!(matches!(
            execution,
            crate::ExecutionPrepareRequest::Direct(_)
        ));
    }

    #[test]
    fn maker_control_requests_are_tagged_exact_and_amount_safe() {
        let strand_upsert: PlatformMakerStrandPrepareRequest =
            serde_json::from_value(serde_json::json!({
                "action": "upsert",
                "maker_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
                "enabled": true,
                "async_only": false,
                "sync_spread_ticks": 1,
                "mid_price_atoms": "123000000",
                "max_exposure_base_atoms": "1000000",
                "bid_offsets_ticks": vec![1; 16],
                "ask_offsets_ticks": vec![1; 16],
                "bid_sizes_base_atoms": vec!["1"; 16],
                "ask_sizes_base_atoms": vec!["1"; 16],
                "valid_until_slot": "0"
            }))
            .unwrap();
        let serialized = serde_json::to_value(&strand_upsert).unwrap();
        assert_eq!(serialized["max_exposure_base_atoms"], "1000000");
        assert!(serialized.get("max_exposure_base_lots").is_none());

        // 0.2.1 clients remain accepted, but every response and current client
        // uses the corrected base-atom vocabulary.
        let legacy_strand: PlatformMakerStrandPrepareRequest =
            serde_json::from_value(serde_json::json!({
                "action": "upsert",
                "maker_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
                "enabled": true,
                "async_only": false,
                "sync_spread_ticks": 1,
                "mid_price_atoms": "123000000",
                "max_exposure_base_lots": "1000000",
                "bid_offsets_ticks": vec![1; 16],
                "ask_offsets_ticks": vec![1; 16],
                "bid_sizes_base_lots": vec!["1"; 16],
                "ask_sizes_base_lots": vec!["1"; 16],
                "valid_until_slot": "0"
            }))
            .unwrap();
        assert_eq!(serde_json::to_value(legacy_strand).unwrap(), serialized);

        let strand: PlatformMakerStrandPrepareRequest = serde_json::from_value(serde_json::json!({
            "action": "recenter",
            "maker_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
            "new_mid_price_atoms": "123000000",
            "valid_until_slot": "0"
        }))
        .unwrap();
        assert!(matches!(
            strand,
            PlatformMakerStrandPrepareRequest::Recenter { .. }
        ));

        let current: PlatformMakerCurrentPrepareRequest =
            serde_json::from_value(serde_json::json!({
                "action": "cancel",
                "maker_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL"
            }))
            .unwrap();
        assert!(matches!(
            current,
            PlatformMakerCurrentPrepareRequest::Cancel { .. }
        ));
        assert!(
            serde_json::from_value::<PlatformMakerCurrentPrepareRequest>(serde_json::json!({
                "action": "cancel",
                "maker_wallet": "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
                "oracle_price": 123.45
            }))
            .is_err()
        );
    }
}
