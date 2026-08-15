//! Public SDK 2.0 request and response primitives.

use serde::{Deserialize, Serialize};

use crate::{CapabilityRisk, McpExposure};

pub const PLATFORM_SCHEMA_VERSION: u16 = 2;
pub const PLATFORM_CONTRACT_VERSION: &str = "2.0";
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_CAPABILITIES_FIXTURE: &str =
    include_str!("../fixtures/v2/platform-capabilities.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ASSETS_FIXTURE: &str = include_str!("../fixtures/v2/assets.json");
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
pub const PLATFORM_TRADES_FIXTURE: &str = include_str!("../fixtures/v2/trades.json");
#[cfg(any(test, feature = "fixtures"))]
#[doc(hidden)]
pub const PLATFORM_ACCOUNT_FIXTURE: &str = include_str!("../fixtures/v2/account.json");
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
    pub minimum_order_size_atoms: String,
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
        account_sequence: String,
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
        account_sequence: String,
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
        account_sequence: String,
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
        account_sequence: String,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformOrderPrepareRequest {
    pub challenge_id: String,
    /// Base58 Ed25519 signature over `authorization_payload_base64`.
    pub authorization_signature: String,
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

/// Collision policy for an incoming order that would cross the owner's own
/// resting liquidity. Every mode still preserves Strata's matcher and on-chain
/// self-fill prohibition; this only controls which order is cancelled first.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformSelfTradePrevention {
    CancelTaker,
    CancelMaker,
    CancelBoth,
    SkipOwnLiquidity,
}

/// One command on the persistent order-control connection. Challenge results
/// may contain an effective request that differs from the requested one only
/// by the explicitly selected self-trade prevention transformation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommand {
    Challenge {
        request: PlatformOrderChallengeRequest,
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
/// their existing exact external-signing boundaries.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlatformOrderCommandClientFrame {
    Authenticate {
        owner_wallet: String,
        session_public_key: String,
        /// Base58 Ed25519 signature over the stream authentication payload.
        signature: String,
    },
    Command {
        request_id: String,
        sequence: String,
        command: PlatformOrderCommand,
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
#[serde(tag = "type", rename_all = "snake_case")]
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
        let assets: PlatformAssetsResponse = serde_json::from_str(PLATFORM_ASSETS_FIXTURE).unwrap();
        let markets: PlatformMarketsResponse =
            serde_json::from_str(PLATFORM_MARKETS_FIXTURE).unwrap();
        let book: PlatformBookSnapshotResponse =
            serde_json::from_str(PLATFORM_BOOK_FIXTURE).unwrap();
        let bbo: PlatformBestBidAskResponse = serde_json::from_str(PLATFORM_BBO_FIXTURE).unwrap();
        let fees: PlatformFeeScheduleResponse =
            serde_json::from_str(PLATFORM_FEES_FIXTURE).unwrap();
        let status: PlatformMarketStatusResponse =
            serde_json::from_str(PLATFORM_STATUS_FIXTURE).unwrap();
        let trades: PlatformTradesResponse = serde_json::from_str(PLATFORM_TRADES_FIXTURE).unwrap();
        let account: PlatformAccountSnapshotResponse =
            serde_json::from_str(PLATFORM_ACCOUNT_FIXTURE).unwrap();
        let order_challenge: PlatformOrderChallengeResponse =
            serde_json::from_str(PLATFORM_ORDER_CHALLENGE_FIXTURE).unwrap();
        let order_prepare: PlatformOrderPrepareResponse =
            serde_json::from_str(PLATFORM_ORDER_PREPARE_FIXTURE).unwrap();
        let order_submit: PlatformOrderSubmitResponse =
            serde_json::from_str(PLATFORM_ORDER_SUBMIT_FIXTURE).unwrap();
        let order_status: PlatformOrderStatusResponse =
            serde_json::from_str(PLATFORM_ORDER_STATUS_FIXTURE).unwrap();

        assert_eq!(discovery.schema_version, PLATFORM_SCHEMA_VERSION);
        assert_eq!(discovery.capabilities.len(), 5);
        assert!(!discovery.authority.accepts_private_keys);
        assert_eq!(assets.assets.len(), 2);
        assert_eq!(markets.markets.len(), 1);
        assert_eq!(markets.markets[0].base_asset_id, assets.assets[0].asset_id);
        assert_eq!(markets.markets[0].quote_asset_id, assets.assets[1].asset_id);
        assert_eq!(book.sequence, "42");
        assert_eq!(bbo.best_bid.unwrap().price_atoms, "149990000");
        assert_eq!(fees.maximum_immediate_execution_fee_bps, 10);
        assert_eq!(status.status, PlatformMarketState::Active);
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
            operations[1],
            PlatformOrderBatchOperation::Replace { .. }
        ));

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
    fn persistent_order_commands_are_strict_and_explicit_about_self_trade_policy() {
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
        assert!(
            serde_json::from_value::<PlatformOrderCommandClientFrame>(serde_json::json!({
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
            .is_err()
        );
    }
}
