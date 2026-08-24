//! Official Rust client for Strata markets and Sonar quotes.
//!
//! It provides typed requests and responses and validates compatibility, quote
//! binding, and economic fields before returning data to the application.

mod account_stream;
mod execution_stream;
mod maker_stream;
mod market_stream;
mod order_stream;
pub mod transaction_verifier;
mod twap_stream;

pub use account_stream::{account_stream_auth_message, AccountStream, ACCOUNT_STREAM_AUTH_DOMAIN};
pub use execution_stream::{ExecutionStream, MAX_WATCHED_EXECUTIONS};
pub use maker_stream::{maker_stream_auth_message, MakerStream, MAKER_STREAM_AUTH_DOMAIN};
pub use market_stream::MarketDataStream;
pub use order_stream::{
    DeadManGuard, OrderChallengeResult, OrderCommandStream, ORDER_STREAM_AUTH_DOMAIN,
};
pub use transaction_verifier::{
    decode_transaction, verify_execution_transaction, verify_maker_transaction,
    verify_order_transaction, verify_signed_transaction_message, verify_twap_transaction,
    DecodedInstruction, DecodedTransaction, DefaultTransactionVerifier, TransactionVersion,
};
pub use twap_stream::TwapStream;

use async_trait::async_trait;
use base64::Engine as _;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use strata_public_contract::{ErrorResponse, CONTRACT_MAJOR, CONTRACT_VERSION};
use thiserror::Error;

pub use strata_public_contract::platform::{
    LivePlatformCapability, PageInfo, PageRequest, PermissionSource, PlatformAccountEvent,
    PlatformAccountFill, PlatformAccountOrder, PlatformAccountSnapshotResponse,
    PlatformActionGraphResponse, PlatformAsset, PlatformAssetsResponse, PlatformAuthority,
    PlatformBestBidAskResponse, PlatformBookChange, PlatformBookLevel, PlatformBookSide,
    PlatformBookSnapshotResponse, PlatformBugReport, PlatformBugStatus, PlatformBugSubmitRequest,
    PlatformBugSubmitResponse, PlatformBugsResponse, PlatformCandle, PlatformCandlesResponse,
    PlatformDeadManState, PlatformDeadManStatus, PlatformDiscoveryResponse,
    PlatformExecutionCommand, PlatformExecutionEvent, PlatformExecutionRow, PlatformExecutionState,
    PlatformExecutionStatusResponse, PlatformFeeScheduleResponse, PlatformGraphModule,
    PlatformGraphRelation, PlatformMakerControlAction, PlatformMakerControlPrepareResponse,
    PlatformMakerControlProduct, PlatformMakerControlSubmissionStatus,
    PlatformMakerControlSubmitRequest, PlatformMakerControlSubmitResponse,
    PlatformMakerCurrentPrepareRequest, PlatformMakerEvent, PlatformMakerFill,
    PlatformMakerProduct, PlatformMakerReputationResponse, PlatformMakerReputationTier,
    PlatformMakerStatusResponse, PlatformMakerStrandPrepareRequest, PlatformMakerTierProgress,
    PlatformMarkResponse, PlatformMarket, PlatformMarketAction, PlatformMarketDataEvent,
    PlatformMarketState, PlatformMarketStatusResponse, PlatformMarketsResponse, PlatformOperation,
    PlatformOperationTransport, PlatformOrderAction, PlatformOrderBatchOperation,
    PlatformOrderChallengeRequest, PlatformOrderChallengeResponse, PlatformOrderCommand,
    PlatformOrderCommandBatchEvent, PlatformOrderCommandBatchFormat,
    PlatformOrderCommandClientFrame, PlatformOrderCommandEvent, PlatformOrderCommandServerFrame,
    PlatformOrderControlStatus, PlatformOrderPrepareAuthorization, PlatformOrderPrepareRequest,
    PlatformOrderPrepareResponse, PlatformOrderState, PlatformOrderStatusRequest,
    PlatformOrderStatusResponse, PlatformOrderSubmissionStatus, PlatformOrderSubmitRequest,
    PlatformOrderSubmitResponse, PlatformOrderType, PlatformOwnerRewards,
    PlatformPortfolioHistoryPoint, PlatformPortfolioHistoryRange, PlatformPortfolioHistoryResponse,
    PlatformPortfolioResponse, PlatformReferralClaimRequest, PlatformReferralClaimResponse,
    PlatformReferralLinkRequest, PlatformReferralLinkResponse, PlatformReferralsResponse,
    PlatformRewardStanding, PlatformRewardsResponse, PlatformSelfTradePrevention,
    PlatformServiceState, PlatformServiceStatusResponse, PlatformSettlementState,
    PlatformSwapQuoteRequest, PlatformSwapQuoteResponse, PlatformTrade, PlatformTradeSide,
    PlatformTradesResponse, PlatformTransport, PlatformTwap, PlatformTwapChallengeRequest,
    PlatformTwapChallengeResponse, PlatformTwapControlAction, PlatformTwapEvent, PlatformTwapFill,
    PlatformTwapPrepareAuthorization, PlatformTwapPrepareRequest, PlatformTwapPrepareResponse,
    PlatformTwapState, PlatformTwapSubmitRequest, PlatformTwapSubmitResponse,
    PlatformTwapsResponse, PlatformVaultAction, PlatformVaultDelegateAction,
    PlatformVaultDelegatePrepareRequest, PlatformVaultDelegatePrepareResponse,
    PlatformVaultDepositPrepareRequest, PlatformVaultDepositPrepareResponse,
    PlatformVaultPausePrepareRequest, PlatformVaultPausePrepareResponse,
    PlatformVaultPolicyPrepareRequest, PlatformVaultPolicyPrepareResponse,
    PlatformVaultSessionState, PlatformVaultSessionStatus, PlatformVaultSetupMode,
    PlatformVaultSetupPrepareRequest, PlatformVaultSetupPrepareResponse,
    PlatformVaultSpendingLimit, PlatformVaultState, PlatformVaultStatusResponse,
    PlatformVaultSubmissionStatus, PlatformVaultSubmitRequest, PlatformVaultSubmitResponse,
    PlatformVaultWithdrawPrepareRequest, PlatformVaultWithdrawPrepareResponse,
    PlatformVaultWithdrawalAccess, PlatformVaultWithdrawalMode, PlatformWorkflow,
    PlatformWorkflowEdge, PlatformWorkflowNode, SigningLocation,
    PLATFORM_SESSION_DEFAULT_MAXIMUM_TOLERANCE_BPS,
    PLATFORM_SESSION_DEFAULT_MINIMUM_INTERVAL_SECONDS, PLATFORM_SESSION_MAX_SPENDING_LIMITS,
};
pub use strata_public_contract::{
    ActionAuthorityModel, ActionEdge, ActionGraph, ActionNode, ActionNodeKind, ActionOperation,
    CapabilityCatalog, CapabilityDescriptor, CapabilityRisk, CapabilityStability,
    ExecutionChallengeRequest, ExecutionChallengeResponse, ExecutionPrepareAuthorization,
    ExecutionPrepareRequest, ExecutionPrepareResponse, ExecutionStatus, ExecutionSubmitRequest,
    ExecutionSubmitResponse, Market, MarketsResponse, McpExposure, QuoteRequest, QuoteResponse,
    QuoteSide, DEFAULT_MAXIMUM_TOLERANCE_BPS, DEFAULT_SLIPPAGE_BPS,
};

pub const DEFAULT_API_BASE: &str = "https://api.stratabook.app";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_PLATFORM_CAPABILITY_CACHE: Duration = Duration::from_secs(5);
const PUBLIC_EXECUTION_AUTH_DOMAIN: &[u8] = b"strata-sonar-execution:v1\0";
const PUBLIC_ORDER_AUTH_DOMAIN: &[u8] = b"strata-platform-order-control:v1\0";
const PUBLIC_TWAP_AUTH_DOMAIN: &[u8] = b"strata-twap-control:v1\0";
const MAX_PLATFORM_PAGE_SIZE: u32 = 200;
const DEFAULT_ACCOUNT_FILL_LIMIT: u16 = 100;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformBookRequest {
    pub depth: Option<u16>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformTradesRequest {
    pub limit: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformCandlesRequest {
    pub from_ms: u64,
    pub to_ms: u64,
    pub resolution_seconds: Option<u32>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformRewardsRequest {
    pub wallet_address: Option<String>,
    pub limit: Option<u16>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformVaultStatusRequest {
    pub session_public_key: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformAccountMarketRequest {
    pub fill_limit: Option<u16>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformAccountRequest {
    pub fill_limit: Option<u16>,
    /// Omit to read every currently discoverable public Strata market.
    pub market_ids: Option<Vec<String>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformAccountSnapshot {
    pub wallet_address: String,
    pub server_time_ms: u64,
    pub markets: Vec<PlatformAccountSnapshotResponse>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformMakerReputationAuthorizedRequest {
    pub market_id: String,
    pub wallet_address: String,
    pub authorization_time_ms: u64,
    pub authorization_signature: String,
}

/// Detached external authorization for the owner-scoped maker status read.
pub type PlatformMakerStatusAuthorizedRequest = PlatformMakerReputationAuthorizedRequest;

#[async_trait]
pub trait AccountSigner: Send + Sync {
    /// Canonical base58 wallet address whose account state is being read.
    fn public_key(&self) -> &str;

    /// Sign only the exact SDK-generated, short-lived account-read message.
    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, String>;
}

/// External maker-wallet adapter. Strata never accepts or stores its key.
#[async_trait]
pub trait MakerTransactionSigner: Send + Sync {
    fn public_key(&self) -> &str;
    async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, String>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformMakerQuickstartSide {
    Both,
    Buy,
    Sell,
}

/// Human-facing maker configuration. `spread_bps` is the distance from the
/// Strata mark to the first quote on each side. `size` is an exact decimal
/// base amount, optionally suffixed by its symbol (for example `0.01 SOL`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformMakerQuickstartRequest {
    pub market: String,
    pub product: PlatformMakerControlProduct,
    pub spread_bps: u16,
    pub size: String,
    /// `None` means ten minutes; otherwise `30s`, `10m`, `2h`, or `1d`.
    pub duration: Option<String>,
    /// `None` means three levels.
    pub levels: Option<u8>,
    /// `None` means the same value as `spread_bps`.
    pub level_step_bps: Option<u16>,
    pub side: PlatformMakerQuickstartSide,
    pub async_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformMakerQuickstartOperation {
    Strand(PlatformMakerStrandPrepareRequest),
    Current(PlatformMakerCurrentPrepareRequest),
}

impl PlatformMakerQuickstartOperation {
    fn maker_wallet(&self) -> &str {
        match self {
            Self::Strand(operation) => strand_prepare_wallet_raw(operation),
            Self::Current(operation) => current_prepare_wallet_raw(operation),
        }
    }

    fn is_cancel(&self) -> bool {
        matches!(
            self,
            Self::Strand(PlatformMakerStrandPrepareRequest::Cancel { .. })
                | Self::Current(PlatformMakerCurrentPrepareRequest::Cancel { .. })
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformMakerQuickstartPrepared {
    pub market: PlatformMarket,
    pub base_asset: Option<PlatformAsset>,
    pub product: PlatformMakerControlProduct,
    pub operation: PlatformMakerQuickstartOperation,
    pub prepared: PlatformMakerControlPrepareResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformMakerQuickstartResult {
    pub prepared: PlatformMakerQuickstartPrepared,
    pub receipt: PlatformMakerControlSubmitResponse,
    pub maker_status: PlatformMakerStatusResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformMakerStopResult {
    pub market: PlatformMarket,
    pub product: PlatformMakerControlProduct,
    pub prepared: Option<PlatformMakerQuickstartPrepared>,
    pub receipt: Option<PlatformMakerControlSubmitResponse>,
    pub maker_status: PlatformMakerStatusResponse,
    pub already_stopped: bool,
}

pub struct MakerVerificationContext<'a> {
    pub market_id: &'a str,
    pub maker_wallet: &'a str,
    pub operation: &'a PlatformMakerQuickstartOperation,
    pub prepared: &'a PlatformMakerControlPrepareResponse,
}

/// Type-level placeholder for "no signer" (public reads).
pub struct NoSigner;

#[async_trait]
impl AccountSigner for NoSigner {
    fn public_key(&self) -> &str {
        ""
    }

    async fn sign_message(&self, _message: &[u8]) -> Result<Vec<u8>, String> {
        Err("no signer".to_owned())
    }
}

#[async_trait]
pub trait SessionSigner: Send + Sync {
    /// Canonical base58 Ed25519 public key registered as the Vault delegate.
    fn public_key(&self) -> &str;

    /// Sign the exact SDK-validated public operation authorization. Only the
    /// two-step challenge path needs it; the one-call `execute_*` helpers and
    /// the order command channel are one signature over the transaction and
    /// never call it.
    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, String>;

    /// Add only the session signature to an already-verified transaction.
    async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderExecuteOperation {
    Place {
        owner_wallet: String,
        /// Vault market account sequence. `None` lets Strata resolve the next
        /// sequence from the Vault's confirmed market account when the
        /// transaction is prepared.
        account_sequence: Option<String>,
        client_order_id: String,
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: String,
        size_atoms: String,
    },
    Cancel {
        owner_wallet: String,
        order_id: String,
    },
    CancelAll {
        owner_wallet: String,
    },
    Replace {
        owner_wallet: String,
        order_id: String,
        account_sequence: Option<String>,
        client_order_id: String,
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: String,
        size_atoms: String,
    },
    Batch {
        owner_wallet: String,
        operations: Vec<PlatformOrderBatchOperation>,
    },
}

impl OrderExecuteOperation {
    pub(crate) fn challenge_request(
        &self,
        session_public_key: String,
    ) -> PlatformOrderChallengeRequest {
        match self {
            Self::Place {
                owner_wallet,
                account_sequence,
                client_order_id,
                side,
                order_type,
                limit_price_atoms,
                size_atoms,
            } => PlatformOrderChallengeRequest::Place {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
                account_sequence: account_sequence.clone(),
                client_order_id: client_order_id.clone(),
                side: *side,
                order_type: *order_type,
                limit_price_atoms: limit_price_atoms.clone(),
                size_atoms: size_atoms.clone(),
            },
            Self::Cancel {
                owner_wallet,
                order_id,
            } => PlatformOrderChallengeRequest::Cancel {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
                order_id: order_id.clone(),
            },
            Self::CancelAll { owner_wallet } => PlatformOrderChallengeRequest::CancelAll {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
            },
            Self::Replace {
                owner_wallet,
                order_id,
                account_sequence,
                client_order_id,
                side,
                order_type,
                limit_price_atoms,
                size_atoms,
            } => PlatformOrderChallengeRequest::Replace {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
                order_id: order_id.clone(),
                account_sequence: account_sequence.clone(),
                client_order_id: client_order_id.clone(),
                side: *side,
                order_type: *order_type,
                limit_price_atoms: limit_price_atoms.clone(),
                size_atoms: size_atoms.clone(),
            },
            Self::Batch {
                owner_wallet,
                operations,
            } => PlatformOrderChallengeRequest::Batch {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
                operations: operations.clone(),
            },
        }
    }
}

/// Everything a verifier needs to decide whether the session may sign one
/// prepared resting-order transaction.
#[derive(Debug)]
pub struct OrderVerificationContext<'a> {
    /// Present only on the two-step (challenge) path.
    pub challenge: Option<&'a PlatformOrderChallengeResponse>,
    /// The bound operation: exactly as sent (direct path) or as made
    /// effective by the challenge (order command channel).
    pub operation: &'a PlatformOrderChallengeRequest,
    pub market_id: &'a str,
    pub prepared: &'a PlatformOrderPrepareResponse,
    pub owner_wallet: &'a str,
    pub session_public_key: &'a str,
}

#[async_trait]
pub trait OrderVerifier: Send + Sync {
    /// Reject unless the prepared transaction implements the exact signed
    /// order operation for this Vault session.
    async fn verify(&self, context: &OrderVerificationContext<'_>) -> Result<(), String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TwapExecuteOperation {
    Place {
        owner_wallet: String,
        side: PlatformTradeSide,
        total_size_atoms: String,
        slices_total: u16,
        maximum_tolerance_bps: u16,
        interval_slots: u32,
        limit_price_atoms: String,
    },
    Cancel {
        owner_wallet: String,
        twap_id: String,
    },
}

impl TwapExecuteOperation {
    fn challenge_request(&self, session_public_key: String) -> PlatformTwapChallengeRequest {
        match self {
            Self::Place {
                owner_wallet,
                side,
                total_size_atoms,
                slices_total,
                maximum_tolerance_bps,
                interval_slots,
                limit_price_atoms,
            } => PlatformTwapChallengeRequest::Place {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
                side: *side,
                total_size_atoms: total_size_atoms.clone(),
                slices_total: *slices_total,
                maximum_tolerance_bps: *maximum_tolerance_bps,
                interval_slots: *interval_slots,
                limit_price_atoms: limit_price_atoms.clone(),
            },
            Self::Cancel {
                owner_wallet,
                twap_id,
            } => PlatformTwapChallengeRequest::Cancel {
                owner_wallet: owner_wallet.clone(),
                session_public_key,
                twap_id: twap_id.clone(),
            },
        }
    }
}

/// Everything a verifier needs to decide whether the session may sign one
/// prepared TWAP-control transaction.
#[derive(Debug)]
pub struct TwapVerificationContext<'a> {
    /// Present only on the two-step (challenge) path.
    pub challenge: Option<&'a PlatformTwapChallengeResponse>,
    /// The requested action, exactly as sent.
    pub operation: &'a PlatformTwapChallengeRequest,
    pub market_id: &'a str,
    pub prepared: &'a PlatformTwapPrepareResponse,
    pub owner_wallet: &'a str,
    pub session_public_key: &'a str,
}

#[async_trait]
pub trait TwapVerifier: Send + Sync {
    /// Reject unless the prepared transaction implements the exact bounded
    /// TWAP action authorized by the external owner.
    async fn verify(&self, context: &TwapVerificationContext<'_>) -> Result<(), String>;
}

/// Everything a verifier needs to decide whether the session may sign one
/// prepared immediate execution.
#[derive(Debug)]
pub struct ExecutionVerificationContext<'a> {
    pub quote: &'a QuoteResponse,
    /// Present only on the two-step (challenge) path.
    pub challenge: Option<&'a ExecutionChallengeResponse>,
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
    #[error("persistent order command stream failed: {0}")]
    Stream(String),
    #[error("order command rejected ({code}): {message}")]
    Command {
        code: String,
        message: String,
        retryable: bool,
    },
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
}

#[derive(Clone, Debug)]
pub struct StrataClient {
    base_url: Url,
    http: reqwest::Client,
    platform_capability_cache: Arc<Mutex<Option<CachedPlatformDiscovery>>>,
}

#[derive(Clone, Debug)]
struct CachedPlatformDiscovery {
    value: PlatformDiscoveryResponse,
    expires_at: Instant,
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
        Ok(Self {
            base_url,
            http,
            platform_capability_cache: Arc::new(Mutex::new(None)),
        })
    }

    /// Open one authenticated, persistent order-command connection. The
    /// external session signer is used for authentication and is not retained.
    pub async fn connect_order_commands<S: SessionSigner + ?Sized>(
        &self,
        market_id: &str,
        owner_wallet: &str,
        signer: &S,
    ) -> Result<OrderCommandStream, SdkError> {
        self.require_platform_capability(
            "orders.prepare",
            CapabilityRisk::Prepare,
            PlatformTransport::Websocket,
        )
        .await?;
        self.require_platform_capability(
            "orders.submit",
            CapabilityRisk::Submit,
            PlatformTransport::Websocket,
        )
        .await?;
        OrderCommandStream::connect(self, market_id, owner_wallet, signer).await
    }

    /// Open the sequenced Strata market-data stream. A sequence gap fails
    /// closed so the caller can reconnect and recover from a new snapshot.
    pub async fn connect_market_data(&self, market_id: &str) -> Result<MarketDataStream, SdkError> {
        self.require_platform_capability(
            "market_data.book.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        self.require_platform_capability(
            "market_data.bbo.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        self.require_platform_capability(
            "market_data.trades.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        self.require_platform_capability(
            "market_data.marks.read",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        MarketDataStream::connect(self, market_id).await
    }

    /// Open the sequenced execution stream for one market, watching the opaque
    /// handles issued by `execution.prepare`. It begins with a snapshot; a gap
    /// fails closed so the caller reconnects and recovers.
    pub async fn connect_executions(
        &self,
        market_id: &str,
        execution_ids: &[String],
    ) -> Result<ExecutionStream, SdkError> {
        self.require_platform_capability(
            "execution.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        ExecutionStream::connect(self, market_id, execution_ids).await
    }

    /// Open the sequenced TWAP progress stream for a wallet in one market. It
    /// begins with a snapshot and then delivers one complete sanitized TWAP row
    /// per change; a gap fails closed so the caller reconnects and recovers.
    pub async fn connect_twaps(
        &self,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<TwapStream, SdkError> {
        self.require_platform_capability(
            "algos.twap.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        TwapStream::connect(self, market_id, wallet_address).await
    }

    /// Open the maker stream for one market by wallet address — public, no
    /// signature: a maker snapshot followed by sequenced maker fills,
    /// product/exposure changes, and heartbeats.
    pub async fn connect_maker_for_wallet(
        &self,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<MakerStream, SdkError> {
        self.require_platform_capability(
            "mm.fills.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        MakerStream::connect(self, market_id, wallet_address, None::<&NoSigner>).await
    }

    /// Same stream, addressed by a signer's public key; the server's
    /// compatibility challenge is answered with the signer's signature.
    pub async fn connect_maker<S: AccountSigner + ?Sized>(
        &self,
        market_id: &str,
        signer: &S,
    ) -> Result<MakerStream, SdkError> {
        self.require_platform_capability(
            "mm.fills.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        MakerStream::connect(self, market_id, signer.public_key(), Some(signer)).await
    }

    /// Open one externally authenticated private account stream. The signer
    /// is used only for the server challenge and is not retained by the SDK.
    pub async fn connect_account<S: AccountSigner + ?Sized>(
        &self,
        market_id: &str,
        signer: &S,
    ) -> Result<AccountStream, SdkError> {
        self.require_platform_capability(
            "account.stream",
            CapabilityRisk::Read,
            PlatformTransport::Websocket,
        )
        .await?;
        AccountStream::connect(self, market_id, signer).await
    }

    /// Read the operations currently enabled through the public 2.0 product
    /// contract. This response contains product capabilities only.
    pub async fn platform_capabilities(&self) -> Result<PlatformDiscoveryResponse, SdkError> {
        let discovery: PlatformDiscoveryResponse = self.get("v2/capabilities", &[]).await?;
        validate_platform_discovery(&discovery)?;
        self.store_platform_capabilities(discovery.clone())?;
        Ok(discovery)
    }

    async fn cached_platform_capabilities(&self) -> Result<PlatformDiscoveryResponse, SdkError> {
        let cached = self
            .platform_capability_cache
            .lock()
            .map_err(|_| SdkError::InvalidResponse("capability cache is unavailable".to_owned()))?
            .as_ref()
            .filter(|cached| cached.expires_at > Instant::now())
            .map(|cached| cached.value.clone());
        match cached {
            Some(discovery) => Ok(discovery),
            None => self.platform_capabilities().await,
        }
    }

    fn store_platform_capabilities(
        &self,
        discovery: PlatformDiscoveryResponse,
    ) -> Result<(), SdkError> {
        *self.platform_capability_cache.lock().map_err(|_| {
            SdkError::InvalidResponse("capability cache is unavailable".to_owned())
        })? = Some(CachedPlatformDiscovery {
            value: discovery,
            expires_at: Instant::now() + DEFAULT_PLATFORM_CAPABILITY_CACHE,
        });
        Ok(())
    }

    async fn require_platform_capability(
        &self,
        capability_id: &str,
        risk: CapabilityRisk,
        transport: PlatformTransport,
    ) -> Result<PlatformDiscoveryResponse, SdkError> {
        let discovery = self.cached_platform_capabilities().await?;
        let available = discovery.capabilities.iter().any(|capability| {
            capability.id == capability_id
                && capability.risk == risk
                && capability.transports.contains(&transport)
        });
        if !available {
            return Err(SdkError::OperationUnavailable(format!(
                "live capability is not available: {capability_id}"
            )));
        }
        Ok(discovery)
    }

    /// Read the complete customer-safe entity, operation, and workflow graph
    /// projected against the capabilities that are live now.
    pub async fn platform_action_graph(&self) -> Result<PlatformActionGraphResponse, SdkError> {
        let graph: PlatformActionGraphResponse = self.get("v2/action-graph", &[]).await?;
        validate_platform_action_graph(&graph)?;
        Ok(graph)
    }

    /// Read product-level readiness without exposing internal services.
    pub async fn platform_status(&self) -> Result<PlatformServiceStatusResponse, SdkError> {
        self.require_platform_capability(
            "platform.status.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let status: PlatformServiceStatusResponse = self.get("v2/status", &[]).await?;
        validate_platform_version(status.schema_version, &status.contract_version)?;
        Ok(status)
    }

    pub async fn platform_assets(
        &self,
        request: PageRequest,
    ) -> Result<PlatformAssetsResponse, SdkError> {
        self.require_platform_capability(
            "assets.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let query = normalize_page_request(request)?;
        let response: PlatformAssetsResponse = self.get("v2/assets", &query).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        validate_page_info(&response.page)?;
        if response.assets.iter().any(|asset| {
            asset.asset_id.trim().is_empty()
                || asset.symbol.trim().is_empty()
                || asset.name.trim().is_empty()
                || asset.decimals > 18
        }) {
            return Err(SdkError::InvalidResponse(
                "asset discovery contains an invalid public asset".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Request a short-lived exact-input quote between two assets returned by
    /// [`Self::platform_assets`].
    pub async fn platform_swap_quote(
        &self,
        request: PlatformSwapQuoteRequest,
    ) -> Result<PlatformSwapQuoteResponse, SdkError> {
        self.require_platform_capability(
            "quotes.swap.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let input_asset_id = validate_platform_asset_id(&request.input_asset_id)?;
        let output_asset_id = validate_platform_asset_id(&request.output_asset_id)?;
        if input_asset_id == output_asset_id {
            return Err(SdkError::InvalidRequest(
                "input and output asset IDs must differ".to_owned(),
            ));
        }
        let amount_in =
            canonical_request_atoms(&request.amount_in_atoms, "amount_in_atoms", false)?
                .parse::<u64>()
                .expect("canonical atomic request was already range checked");
        if request.maximum_tolerance_bps > 1_000 {
            return Err(SdkError::InvalidRequest(
                "maximum_tolerance_bps must be between 0 and 1,000".to_owned(),
            ));
        }
        let quote: PlatformSwapQuoteResponse = self.post("v2/quotes", &request).await?;
        validate_platform_version(quote.schema_version, &quote.contract_version)?;
        if quote.provider != "Sonar"
            || quote.input_asset_id != input_asset_id
            || quote.output_asset_id != output_asset_id
            || quote.amount_in_atoms != request.amount_in_atoms
            || quote.maximum_tolerance_bps != request.maximum_tolerance_bps
            || !valid_handle(&quote.quote_id, "sq_")
            || quote.expires_at_ms <= quote.server_time_ms
        {
            return Err(SdkError::InvalidResponse(
                "swap quote binding or lifetime is invalid".to_owned(),
            ));
        }
        let consumed = validate_response_atoms(
            &quote.amount_in_consumed_atoms,
            "amount_in_consumed_atoms",
            false,
        )?;
        let output = validate_response_atoms(&quote.amount_out_atoms, "amount_out_atoms", false)?;
        let minimum =
            validate_response_atoms(&quote.minimum_output_atoms, "minimum_output_atoms", true)?;
        validate_response_atoms(&quote.input_fee_atoms, "input_fee_atoms", true)?;
        validate_response_atoms(&quote.output_fee_atoms, "output_fee_atoms", true)?;
        canonical_decimal(&quote.reference_price, "reference_price")?;
        canonical_decimal(&quote.price_impact_pct, "price_impact_pct")?;
        if consumed > amount_in || minimum > output {
            return Err(SdkError::InvalidResponse(
                "swap quote economics are internally inconsistent".to_owned(),
            ));
        }
        Ok(quote)
    }

    pub async fn platform_markets(
        &self,
        request: PageRequest,
    ) -> Result<PlatformMarketsResponse, SdkError> {
        self.require_platform_capability(
            "markets.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let query = normalize_page_request(request)?;
        let response: PlatformMarketsResponse = self.get("v2/markets", &query).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        validate_page_info(&response.page)?;
        let mut ids = HashSet::new();
        if response.markets.iter().any(|market| {
            validate_platform_market_id(&market.market_id).is_err()
                || market.label.trim().is_empty()
                || market.base_asset_id.trim().is_empty()
                || market.quote_asset_id.trim().is_empty()
                || !ids.insert(market.market_id.as_str())
        }) {
            return Err(SdkError::InvalidResponse(
                "market discovery contains an invalid public market".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Resolve an opaque market ID or case-insensitive label across all pages.
    pub async fn platform_resolve_market(
        &self,
        reference: &str,
    ) -> Result<PlatformMarket, SdkError> {
        let requested = reference.trim();
        if requested.is_empty() {
            return Err(SdkError::InvalidRequest(
                "market must be a market ID or label".to_owned(),
            ));
        }
        let mut cursor = None;
        let mut matches = Vec::new();
        loop {
            let page = self
                .platform_markets(PageRequest {
                    cursor,
                    limit: Some(MAX_PLATFORM_PAGE_SIZE),
                })
                .await?;
            matches.extend(page.markets.into_iter().filter(|market| {
                market.market_id == requested || market.label.eq_ignore_ascii_case(requested)
            }));
            if !page.page.has_more {
                break;
            }
            cursor = Some(page.page.next_cursor.ok_or_else(|| {
                SdkError::InvalidResponse("market discovery pagination is incomplete".to_owned())
            })?);
        }
        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(SdkError::MarketNotFound(reference.to_owned())),
            _ => Err(SdkError::InvalidRequest(
                "market label is ambiguous; use its opaque market ID".to_owned(),
            )),
        }
    }

    /// Resolve an opaque asset ID or unambiguous symbol across all pages.
    pub async fn platform_resolve_asset(&self, reference: &str) -> Result<PlatformAsset, SdkError> {
        let requested = reference.trim();
        if requested.is_empty() {
            return Err(SdkError::InvalidRequest(
                "asset must be an asset ID or symbol".to_owned(),
            ));
        }
        let mut cursor = None;
        let mut matches = Vec::new();
        loop {
            let page = self
                .platform_assets(PageRequest {
                    cursor,
                    limit: Some(MAX_PLATFORM_PAGE_SIZE),
                })
                .await?;
            matches.extend(page.assets.into_iter().filter(|asset| {
                asset.asset_id == requested || asset.symbol.eq_ignore_ascii_case(requested)
            }));
            if !page.page.has_more {
                break;
            }
            cursor = Some(page.page.next_cursor.ok_or_else(|| {
                SdkError::InvalidResponse("asset discovery pagination is incomplete".to_owned())
            })?);
        }
        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(SdkError::InvalidRequest(format!(
                "asset is not available: {reference}"
            ))),
            _ => Err(SdkError::InvalidRequest(
                "asset symbol is ambiguous; use its opaque asset ID".to_owned(),
            )),
        }
    }

    pub async fn platform_book(
        &self,
        market_id: &str,
        request: PlatformBookRequest,
    ) -> Result<PlatformBookSnapshotResponse, SdkError> {
        self.require_platform_capability(
            "market_data.book.snapshot",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let query = match request.depth {
            Some(depth @ 1..=2_000) => vec![("depth".to_owned(), depth.to_string())],
            Some(_) => {
                return Err(SdkError::InvalidRequest(
                    "depth must be between 1 and 2,000".to_owned(),
                ))
            }
            None => Vec::new(),
        };
        let response: PlatformBookSnapshotResponse = self
            .get(&format!("v2/markets/{market_id}/book"), &query)
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        validate_book_levels(&response.bids, &response.asks)?;
        validate_response_atoms(&response.sequence, "sequence", false)?;
        if response.stream_id.trim().is_empty() || response.snapshot_id.trim().is_empty() {
            return Err(SdkError::InvalidResponse(
                "book snapshot identity is invalid".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_best_bid_ask(
        &self,
        market_id: &str,
    ) -> Result<PlatformBestBidAskResponse, SdkError> {
        self.require_platform_capability(
            "books.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let response: PlatformBestBidAskResponse = self
            .get(&format!("v2/markets/{market_id}/bbo"), &[])
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if let Some(level) = &response.best_bid {
            validate_book_level(level)?;
        }
        if let Some(level) = &response.best_ask {
            validate_book_level(level)?;
        }
        validate_response_atoms(&response.sequence, "sequence", false)?;
        Ok(response)
    }

    pub async fn platform_fees(
        &self,
        market_id: &str,
    ) -> Result<PlatformFeeScheduleResponse, SdkError> {
        self.require_platform_capability(
            "fees.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let response: PlatformFeeScheduleResponse = self
            .get(&format!("v2/markets/{market_id}/fees"), &[])
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if response.passive_maker_fee_bps > 10_000
            || response.maximum_immediate_execution_fee_bps > 10_000
        {
            return Err(SdkError::InvalidResponse(
                "fee schedule is outside public bounds".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_market_status(
        &self,
        market_id: &str,
    ) -> Result<PlatformMarketStatusResponse, SdkError> {
        self.require_platform_capability(
            "markets.status.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let response: PlatformMarketStatusResponse = self
            .get(&format!("v2/markets/{market_id}/status"), &[])
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        validate_response_atoms(&response.tick_size_atoms, "tick_size_atoms", false)?;
        validate_response_atoms(
            &response.minimum_order_size_atoms,
            "minimum_order_size_atoms",
            false,
        )?;
        Ok(response)
    }

    pub async fn platform_trades(
        &self,
        market_id: &str,
        request: PlatformTradesRequest,
    ) -> Result<PlatformTradesResponse, SdkError> {
        self.require_platform_capability(
            "market_data.trades.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let query = match request.limit {
            Some(limit @ 1..=500) => vec![("limit".to_owned(), limit.to_string())],
            Some(_) => {
                return Err(SdkError::InvalidRequest(
                    "trade limit must be between 1 and 500".to_owned(),
                ))
            }
            None => Vec::new(),
        };
        let response: PlatformTradesResponse = self
            .get(&format!("v2/markets/{market_id}/trades"), &query)
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if response.trades.iter().any(|trade| {
            trade.trade_id.trim().is_empty()
                || validate_response_atoms(&trade.price_atoms, "price_atoms", false).is_err()
                || validate_response_atoms(&trade.size_atoms, "size_atoms", false).is_err()
        }) {
            return Err(SdkError::InvalidResponse(
                "trade history contains an invalid trade".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_candles(
        &self,
        market_id: &str,
        request: PlatformCandlesRequest,
    ) -> Result<PlatformCandlesResponse, SdkError> {
        self.require_platform_capability(
            "market_data.candles.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        if request.to_ms <= request.from_ms {
            return Err(SdkError::InvalidRequest(
                "candle timestamps must form an increasing range".to_owned(),
            ));
        }
        let resolution = request.resolution_seconds.unwrap_or(300);
        if !(60..=86_400).contains(&resolution) || !resolution.is_multiple_of(60) {
            return Err(SdkError::InvalidRequest(
                "candle resolution must be whole minutes up to one day".to_owned(),
            ));
        }
        let query = vec![
            ("from_ms".to_owned(), request.from_ms.to_string()),
            ("to_ms".to_owned(), request.to_ms.to_string()),
            ("resolution_seconds".to_owned(), resolution.to_string()),
        ];
        let response: PlatformCandlesResponse = self
            .get(&format!("v2/markets/{market_id}/candles"), &query)
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if response.resolution_seconds != resolution
            || response.candles.iter().any(|candle| {
                candle.started_at_ms < request.from_ms
                    || candle.started_at_ms >= request.to_ms
                    || [
                        &candle.open_price,
                        &candle.high_price,
                        &candle.low_price,
                        &candle.close_price,
                    ]
                    .iter()
                    .any(|price| canonical_decimal(price, "candle price").is_err())
            })
        {
            return Err(SdkError::InvalidResponse(
                "candle response does not match the requested range".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_mark(&self, market_id: &str) -> Result<PlatformMarkResponse, SdkError> {
        self.require_platform_capability(
            "market_data.marks.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let response: PlatformMarkResponse = self
            .get(&format!("v2/markets/{market_id}/marks"), &[])
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if let Some(price) = &response.price_atoms_per_base_unit {
            validate_response_atoms(price, "price_atoms_per_base_unit", false)?;
        }
        if response.stale != response.price_atoms_per_base_unit.is_none()
            || response.quote_decimals > 18
        {
            return Err(SdkError::InvalidResponse(
                "mark staleness metadata is inconsistent".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_execution_status(
        &self,
        market_id: &str,
        execution_id: &str,
    ) -> Result<PlatformExecutionStatusResponse, SdkError> {
        self.require_platform_capability(
            "execution.status.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let execution_id = execution_id.trim();
        if !valid_handle(execution_id, "se_") {
            return Err(SdkError::InvalidRequest(
                "execution_id must be an opaque Strata execution ID".to_owned(),
            ));
        }
        let response: PlatformExecutionStatusResponse = self
            .get(
                &format!("v2/markets/{market_id}/executions/{execution_id}"),
                &[],
            )
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if response.execution_id != execution_id
            || (response.status == PlatformExecutionState::Confirmed
                && response.signature.as_deref().is_none_or(str::is_empty))
        {
            return Err(SdkError::InvalidResponse(
                "execution status does not match the requested execution".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_twaps(
        &self,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<PlatformTwapsResponse, SdkError> {
        self.require_platform_capability(
            "algos.twap.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let response: PlatformTwapsResponse = self
            .get(
                &format!("v2/markets/{market_id}/account/{wallet_address}/twaps"),
                &[],
            )
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if response.wallet_address != wallet_address
            || response
                .twaps
                .iter()
                .any(|twap| !valid_handle(&twap.twap_id, "twap_"))
        {
            return Err(SdkError::InvalidResponse(
                "TWAP history identity does not match the request".to_owned(),
            ));
        }
        Ok(response)
    }

    /// The whole account in one public read, by wallet address: balances
    /// (total / available / locked, exact USD), positions, open orders, and
    /// recent fills across every live market. No signature, no session key,
    /// no market selection. `platform_account` is the same read.
    pub async fn platform_portfolio(
        &self,
        wallet_address: &str,
    ) -> Result<PlatformPortfolioResponse, SdkError> {
        self.require_platform_capability(
            "portfolio.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let response: PlatformPortfolioResponse = self
            .get(&format!("v2/account/{wallet_address}/portfolio"), &[])
            .await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != wallet_address {
            return Err(SdkError::InvalidResponse(
                "portfolio identity does not match the request".to_owned(),
            ));
        }
        validate_platform_portfolio(&response)?;
        Ok(response)
    }

    /// Alias of `platform_portfolio`: the whole account in one public read.
    pub async fn platform_account(
        &self,
        wallet_address: &str,
    ) -> Result<PlatformPortfolioResponse, SdkError> {
        self.platform_portfolio(wallet_address).await
    }

    pub async fn platform_portfolio_history(
        &self,
        wallet_address: &str,
        range: PlatformPortfolioHistoryRange,
    ) -> Result<PlatformPortfolioHistoryResponse, SdkError> {
        self.require_platform_capability(
            "portfolio.history.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let range_value = platform_history_range(range);
        let query = vec![("range".to_owned(), range_value.to_owned())];
        let response: PlatformPortfolioHistoryResponse = self
            .get(
                &format!("v2/account/{wallet_address}/portfolio/history"),
                &query,
            )
            .await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != wallet_address || response.range != range {
            return Err(SdkError::InvalidResponse(
                "portfolio history identity does not match the request".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Read sealed Vault owner state and, optionally, one external session.
    pub async fn platform_vault_status(
        &self,
        wallet_address: &str,
        request: PlatformVaultStatusRequest,
    ) -> Result<PlatformVaultStatusResponse, SdkError> {
        self.require_platform_capability(
            "vault.status.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let session_public_key = request
            .session_public_key
            .as_deref()
            .map(|value| canonical_public_key(value, "session_public_key"))
            .transpose()?;
        let mut query = vec![("wallet_address".to_owned(), wallet_address.clone())];
        if let Some(session_public_key) = &session_public_key {
            query.push(("session_public_key".to_owned(), session_public_key.clone()));
        }
        let response: PlatformVaultStatusResponse = self.get("v2/vault/status", &query).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != wallet_address
            || match (&session_public_key, &response.session) {
                (None, None) => false,
                (Some(expected), Some(session)) => session.session_public_key != *expected,
                _ => true,
            }
        {
            return Err(SdkError::InvalidResponse(
                "Vault status identity does not match the request".to_owned(),
            ));
        }
        let mut asset_ids = HashSet::new();
        if response.session.as_ref().is_some_and(|session| {
            session.spending_limits.len() > 4
                || session.maximum_tolerance_bps > 10_000
                || session.spending_limits.iter().any(|limit| {
                    validate_platform_asset_id(&limit.asset_id).is_err()
                        || !asset_ids.insert(limit.asset_id.clone())
                        || limit
                            .maximum_per_execution_atoms
                            .as_ref()
                            .is_some_and(|atoms| {
                                validate_response_atoms(atoms, "maximum_per_execution_atoms", false)
                                    .is_err()
                            })
                })
                || (session.state != PlatformVaultSessionState::Active
                    && (session.market_execution_ready || session.price_protection_active))
                || (response.state != PlatformVaultState::Active
                    && (session.market_execution_ready || session.price_protection_active))
                || (session.permanent
                    != (session.expires_at_ms.is_none()
                        && session.state != PlatformVaultSessionState::Absent))
                || (session.state == PlatformVaultSessionState::Active
                    && session
                        .expires_at_ms
                        .is_some_and(|expiry| expiry <= response.server_time_ms))
                || (session.state == PlatformVaultSessionState::Expired
                    && session
                        .expires_at_ms
                        .is_none_or(|expiry| expiry > response.server_time_ms))
        }) {
            return Err(SdkError::InvalidResponse(
                "Vault session state is inconsistent".to_owned(),
            ));
        }
        let mut allowed_wallets = HashSet::new();
        if response.withdrawal_access.allowed_wallet_addresses.len() > 8
            || response
                .withdrawal_access
                .allowed_wallet_addresses
                .iter()
                .any(|wallet| {
                    canonical_public_key(wallet, "allowed_wallet_address").is_err()
                        || !allowed_wallets.insert(wallet.clone())
                })
            || ((response.withdrawal_access.mode == PlatformVaultWithdrawalMode::Restricted)
                != !response
                    .withdrawal_access
                    .allowed_wallet_addresses
                    .is_empty())
        {
            return Err(SdkError::InvalidResponse(
                "Vault withdrawal access is inconsistent".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Prepare an owner-authorized Vault pause or resume transaction. The
    /// external owner verifies, signs, and broadcasts the returned bytes.
    pub async fn platform_vault_pause_prepare(
        &self,
        request: PlatformVaultPausePrepareRequest,
    ) -> Result<PlatformVaultPausePrepareResponse, SdkError> {
        self.require_platform_capability(
            "vault.pause",
            CapabilityRisk::Destructive,
            PlatformTransport::Http,
        )
        .await?;
        let request = PlatformVaultPausePrepareRequest {
            wallet_address: canonical_public_key(&request.wallet_address, "wallet_address")?,
            paused: request.paused,
        };
        let response: PlatformVaultPausePrepareResponse =
            self.post("v2/vault/pause/prepare", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != request.wallet_address
            || response.paused != request.paused
            || !response.owner_signature_required
        {
            return Err(SdkError::InvalidResponse(
                "Vault pause preparation does not match the request".to_owned(),
            ));
        }
        canonical_base64(&response.transaction_base64, "transaction_base64")?;
        canonical_public_key(&response.recent_blockhash, "recent_blockhash")?;
        validate_vault_preparation(&response.preparation_id, response.submit_by_ms)?;
        Ok(response)
    }

    /// Prepare one-signature Vault onboarding (or a further session) for
    /// external owner verification, signing, and broadcast. Only the wallet
    /// and the session key are required; the policy fields are optional and
    /// take the product defaults when absent.
    pub async fn platform_vault_setup_prepare(
        &self,
        request: PlatformVaultSetupPrepareRequest,
    ) -> Result<PlatformVaultSetupPrepareResponse, SdkError> {
        self.require_platform_capability(
            "vault.setup",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let session_public_key =
            canonical_public_key(&request.session_public_key, "session_public_key")?;
        if wallet_address == session_public_key {
            return Err(SdkError::InvalidRequest(
                "session_public_key must differ from wallet_address".to_owned(),
            ));
        }
        let market_id = request
            .market_id
            .as_deref()
            .map(validate_platform_market_id)
            .transpose()?;
        let minimum_interval_seconds = request
            .minimum_interval_seconds
            .unwrap_or(PLATFORM_SESSION_DEFAULT_MINIMUM_INTERVAL_SECONDS);
        let maximum_tolerance_bps = request
            .maximum_tolerance_bps
            .unwrap_or(PLATFORM_SESSION_DEFAULT_MAXIMUM_TOLERANCE_BPS);
        let now_ms = unix_ms()?;
        if request
            .expires_at_ms
            .is_some_and(|expiry| expiry % 1_000 != 0 || expiry <= now_ms.saturating_add(60_000))
            || !(1..=86_400).contains(&minimum_interval_seconds)
            || !(1..=1_000).contains(&maximum_tolerance_bps)
            || request.spending_limits.len() > PLATFORM_SESSION_MAX_SPENDING_LIMITS
        {
            return Err(SdkError::InvalidRequest(
                "Vault setup policy is invalid".to_owned(),
            ));
        }
        let mut asset_ids = HashSet::new();
        for limit in &request.spending_limits {
            validate_platform_asset_id(&limit.asset_id)?;
            if !asset_ids.insert(limit.asset_id.clone())
                || limit
                    .maximum_per_execution_atoms
                    .as_ref()
                    .is_some_and(|atoms| {
                        canonical_request_atoms(atoms, "maximum_per_execution_atoms", false)
                            .is_err()
                    })
            {
                return Err(SdkError::InvalidRequest(
                    "Vault setup spending limits are invalid".to_owned(),
                ));
            }
        }
        let request = PlatformVaultSetupPrepareRequest {
            wallet_address,
            session_public_key,
            market_id,
            expires_at_ms: request.expires_at_ms,
            minimum_interval_seconds: Some(minimum_interval_seconds),
            maximum_tolerance_bps: Some(maximum_tolerance_bps),
            spending_limits: request.spending_limits,
        };
        let response: PlatformVaultSetupPrepareResponse =
            self.post("v2/vault/setup/prepare", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != request.wallet_address
            || response.session_public_key != request.session_public_key
            || response.market_id != request.market_id
            || response.expires_at_ms != request.expires_at_ms
            || response.permanent != request.expires_at_ms.is_none()
            || response.minimum_interval_seconds != minimum_interval_seconds
            || response.maximum_tolerance_bps != maximum_tolerance_bps
            || response.spending_limits != request.spending_limits
            || !response.owner_signature_required
        {
            return Err(SdkError::InvalidResponse(
                "Vault setup preparation does not match the request".to_owned(),
            ));
        }
        canonical_base64(&response.transaction_base64, "transaction_base64")?;
        canonical_public_key(&response.recent_blockhash, "recent_blockhash")?;
        validate_vault_preparation(&response.preparation_id, response.submit_by_ms)?;
        Ok(response)
    }

    /// Prepare owner-authorized revocation of one external Vault session. The
    /// SDK never signs or broadcasts this destructive action.
    pub async fn platform_vault_delegate_prepare(
        &self,
        request: PlatformVaultDelegatePrepareRequest,
    ) -> Result<PlatformVaultDelegatePrepareResponse, SdkError> {
        self.require_platform_capability(
            "vault.delegate.manage",
            CapabilityRisk::Destructive,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let session_public_key =
            canonical_public_key(&request.session_public_key, "session_public_key")?;
        if wallet_address == session_public_key {
            return Err(SdkError::InvalidRequest(
                "session_public_key must differ from wallet_address".to_owned(),
            ));
        }
        let request = PlatformVaultDelegatePrepareRequest {
            wallet_address,
            session_public_key,
            action: request.action,
        };
        let response: PlatformVaultDelegatePrepareResponse =
            self.post("v2/vault/delegates/prepare", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != request.wallet_address
            || response.session_public_key != request.session_public_key
            || response.action != request.action
            || !response.owner_signature_required
        {
            return Err(SdkError::InvalidResponse(
                "Vault delegate preparation does not match the request".to_owned(),
            ));
        }
        canonical_base64(&response.transaction_base64, "transaction_base64")?;
        canonical_public_key(&response.recent_blockhash, "recent_blockhash")?;
        validate_vault_preparation(&response.preparation_id, response.submit_by_ms)?;
        Ok(response)
    }

    /// Prepare blocked or restricted Vault withdrawal access. The external
    /// owner verifies, signs, and broadcasts the returned transaction.
    pub async fn platform_vault_policy_prepare(
        &self,
        request: PlatformVaultPolicyPrepareRequest,
    ) -> Result<PlatformVaultPolicyPrepareResponse, SdkError> {
        self.require_platform_capability(
            "vault.policy.manage",
            CapabilityRisk::Destructive,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let allowed = &request.withdrawal_access.allowed_wallet_addresses;
        let mut unique_wallets = HashSet::new();
        if allowed.len() > 8
            || allowed.iter().any(|wallet| {
                canonical_public_key(wallet, "allowed_wallet_address").is_err()
                    || !unique_wallets.insert(wallet.clone())
            })
            || match request.withdrawal_access.mode {
                PlatformVaultWithdrawalMode::Unrestricted => true,
                PlatformVaultWithdrawalMode::Blocked => !allowed.is_empty(),
                PlatformVaultWithdrawalMode::Restricted => allowed.is_empty(),
            }
        {
            return Err(SdkError::InvalidRequest(
                "Vault withdrawal access policy is invalid".to_owned(),
            ));
        }
        let request = PlatformVaultPolicyPrepareRequest {
            wallet_address,
            withdrawal_access: request.withdrawal_access,
        };
        let response: PlatformVaultPolicyPrepareResponse =
            self.post("v2/vault/policies/prepare", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != request.wallet_address
            || response.withdrawal_access != request.withdrawal_access
            || !response.owner_signature_required
        {
            return Err(SdkError::InvalidResponse(
                "Vault policy preparation does not match the request".to_owned(),
            ));
        }
        canonical_base64(&response.transaction_base64, "transaction_base64")?;
        canonical_public_key(&response.recent_blockhash, "recent_blockhash")?;
        validate_vault_preparation(&response.preparation_id, response.submit_by_ms)?;
        Ok(response)
    }

    /// Prepare an exact owner-funded Vault deposit. With `session_public_key`
    /// set, a first deposit also registers that session in the same
    /// transaction (one owner signature onboards and funds the wallet). The
    /// SDK validates the echoed product intent and leaves signing and
    /// broadcast external.
    pub async fn platform_vault_deposit_prepare(
        &self,
        request: PlatformVaultDepositPrepareRequest,
    ) -> Result<PlatformVaultDepositPrepareResponse, SdkError> {
        self.require_platform_capability(
            "vault.deposit",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let session_public_key = request
            .session_public_key
            .as_deref()
            .map(|session| canonical_public_key(session, "session_public_key"))
            .transpose()?;
        if session_public_key.as_deref() == Some(wallet_address.as_str()) {
            return Err(SdkError::InvalidRequest(
                "session_public_key must differ from wallet_address".to_owned(),
            ));
        }
        let request = PlatformVaultDepositPrepareRequest {
            wallet_address,
            market_id: validate_platform_market_id(&request.market_id)?,
            asset_id: validate_platform_asset_id(&request.asset_id)?,
            amount_atoms: canonical_request_atoms(&request.amount_atoms, "amount_atoms", false)?,
            session_public_key,
        };
        let response: PlatformVaultDepositPrepareResponse =
            self.post("v2/vault/deposits/prepare", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        parse_atoms("network_cost_atoms", &response.network_cost_atoms)?;
        if response.wallet_address != request.wallet_address
            || response.market_id != request.market_id
            || response.asset_id != request.asset_id
            || response.amount_atoms != request.amount_atoms
            || response.session_public_key != request.session_public_key
            || (response.registers_session && response.session_public_key.is_none())
            || !response.owner_signature_required
        {
            return Err(SdkError::InvalidResponse(
                "Vault deposit preparation does not match the request".to_owned(),
            ));
        }
        canonical_base64(&response.transaction_base64, "transaction_base64")?;
        canonical_public_key(&response.recent_blockhash, "recent_blockhash")?;
        validate_vault_preparation(&response.preparation_id, response.submit_by_ms)?;
        Ok(response)
    }

    /// Prepare an exact owner-authorized Vault withdrawal to one destination
    /// wallet. Signing and broadcast remain external.
    pub async fn platform_vault_withdraw_prepare(
        &self,
        request: PlatformVaultWithdrawPrepareRequest,
    ) -> Result<PlatformVaultWithdrawPrepareResponse, SdkError> {
        self.require_platform_capability(
            "vault.withdraw",
            CapabilityRisk::Destructive,
            PlatformTransport::Http,
        )
        .await?;
        let request = PlatformVaultWithdrawPrepareRequest {
            wallet_address: canonical_public_key(&request.wallet_address, "wallet_address")?,
            market_id: validate_platform_market_id(&request.market_id)?,
            asset_id: validate_platform_asset_id(&request.asset_id)?,
            destination_wallet_address: canonical_public_key(
                &request.destination_wallet_address,
                "destination_wallet_address",
            )?,
            amount_atoms: canonical_request_atoms(&request.amount_atoms, "amount_atoms", false)?,
        };
        let response: PlatformVaultWithdrawPrepareResponse =
            self.post("v2/vault/withdrawals/prepare", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != request.wallet_address
            || response.market_id != request.market_id
            || response.asset_id != request.asset_id
            || response.destination_wallet_address != request.destination_wallet_address
            || response.amount_atoms != request.amount_atoms
            || !response.owner_signature_required
        {
            return Err(SdkError::InvalidResponse(
                "Vault withdrawal preparation does not match the request".to_owned(),
            ));
        }
        canonical_base64(&response.transaction_base64, "transaction_base64")?;
        canonical_public_key(&response.recent_blockhash, "recent_blockhash")?;
        validate_vault_preparation(&response.preparation_id, response.submit_by_ms)?;
        Ok(response)
    }

    /// Submit an owner-signed prepared Vault transaction. Strata verifies it is
    /// exactly the prepared transaction, pays the fee (and any rent) when the
    /// preparation was sponsored, and broadcasts it. Idempotent per
    /// `idempotency_key`; read the outcome with `platform_vault_submission`.
    pub async fn platform_vault_submit(
        &self,
        request: PlatformVaultSubmitRequest,
    ) -> Result<PlatformVaultSubmitResponse, SdkError> {
        self.require_platform_capability(
            "vault.relay",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        if !valid_handle(&request.preparation_id, "vp_") {
            return Err(SdkError::InvalidRequest(
                "preparation_id is invalid".to_owned(),
            ));
        }
        let request = PlatformVaultSubmitRequest {
            preparation_id: request.preparation_id,
            signed_transaction_base64: canonical_base64(
                &request.signed_transaction_base64,
                "signed_transaction_base64",
            )?,
            idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
        };
        let response: PlatformVaultSubmitResponse = self.post("v2/vault/submit", &request).await?;
        validate_vault_submission(&response, &request.preparation_id)?;
        Ok(response)
    }

    /// Durable outcome of a Vault submission (`submitted` → `confirmed` | `failed`).
    pub async fn platform_vault_submission(
        &self,
        preparation_id: &str,
    ) -> Result<PlatformVaultSubmitResponse, SdkError> {
        self.require_platform_capability(
            "vault.relay",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let preparation_id = preparation_id.trim();
        if !valid_handle(preparation_id, "vp_") {
            return Err(SdkError::InvalidRequest(
                "preparation_id is invalid".to_owned(),
            ));
        }
        let response: PlatformVaultSubmitResponse = self
            .get(&format!("v2/vault/submissions/{preparation_id}"), &[])
            .await?;
        validate_vault_submission(&response, preparation_id)?;
        Ok(response)
    }

    pub async fn platform_rewards(
        &self,
        request: PlatformRewardsRequest,
    ) -> Result<PlatformRewardsResponse, SdkError> {
        self.require_platform_capability(
            "rewards.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let wallet = request
            .wallet_address
            .as_deref()
            .map(|value| canonical_public_key(value, "wallet_address"))
            .transpose()?;
        let mut query = Vec::new();
        if let Some(wallet) = &wallet {
            query.push(("wallet_address".to_owned(), wallet.clone()));
        }
        if let Some(limit @ 1..=100) = request.limit {
            query.push(("limit".to_owned(), limit.to_string()));
        } else if request.limit.is_some() {
            return Err(SdkError::InvalidRequest(
                "reward standings limit must be between 1 and 100".to_owned(),
            ));
        }
        let response: PlatformRewardsResponse = self.get("v2/rewards", &query).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        match (&wallet, &response.owner) {
            (Some(expected), Some(owner)) if owner.wallet_address == *expected => {}
            (None, None) => {}
            _ => {
                return Err(SdkError::InvalidResponse(
                    "reward owner does not match the request".to_owned(),
                ))
            }
        }
        Ok(response)
    }

    pub async fn platform_referrals(
        &self,
        wallet_address: &str,
    ) -> Result<PlatformReferralsResponse, SdkError> {
        self.require_platform_capability(
            "referrals.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let response: PlatformReferralsResponse = self
            .get(&format!("v2/referrals/{wallet_address}"), &[])
            .await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != wallet_address {
            return Err(SdkError::InvalidResponse(
                "referral owner does not match the request".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_referral_link(
        &self,
        request: PlatformReferralLinkRequest,
    ) -> Result<PlatformReferralLinkResponse, SdkError> {
        self.require_platform_capability(
            "referrals.link",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let request = PlatformReferralLinkRequest {
            wallet_address: canonical_public_key(&request.wallet_address, "wallet_address")?,
            referral_code: normalize_referral_code(&request.referral_code)?,
            authorization_signature: canonical_hex_signature(
                &request.authorization_signature,
                "authorization_signature",
            )?,
        };
        let response: PlatformReferralLinkResponse =
            self.post("v2/referrals/link", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != request.wallet_address
            || response.referral_code != request.referral_code
            || response.status != "pending_first_fill"
        {
            return Err(SdkError::InvalidResponse(
                "referral link does not match the request".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_referral_claim(
        &self,
        request: PlatformReferralClaimRequest,
    ) -> Result<PlatformReferralClaimResponse, SdkError> {
        self.require_platform_capability(
            "referrals.claim",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let payout_wallet_address = request
            .payout_wallet_address
            .as_deref()
            .map(|value| canonical_public_key(value, "payout_wallet_address"))
            .transpose()?
            .unwrap_or_else(|| wallet_address.clone());
        let request = PlatformReferralClaimRequest {
            wallet_address: wallet_address.clone(),
            payout_wallet_address: Some(payout_wallet_address.clone()),
            authorization_signature: canonical_hex_signature(
                &request.authorization_signature,
                "authorization_signature",
            )?,
        };
        let response: PlatformReferralClaimResponse =
            self.post("v2/referrals/claim", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        validate_response_atoms(&response.claimable_atoms, "claimable_atoms", false)?;
        if response.wallet_address != wallet_address
            || response.payout_wallet_address != payout_wallet_address
            || response.status != "requested"
        {
            return Err(SdkError::InvalidResponse(
                "referral claim does not match the request".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_bugs(
        &self,
        wallet_address: &str,
    ) -> Result<PlatformBugsResponse, SdkError> {
        self.require_platform_capability(
            "bugs.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        let response: PlatformBugsResponse =
            self.get(&format!("v2/bugs/{wallet_address}"), &[]).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if response.wallet_address != wallet_address {
            return Err(SdkError::InvalidResponse(
                "bug report owner does not match the request".to_owned(),
            ));
        }
        Ok(response)
    }

    pub async fn platform_bug_submit(
        &self,
        request: PlatformBugSubmitRequest,
    ) -> Result<PlatformBugSubmitResponse, SdkError> {
        self.require_platform_capability(
            "bugs.submit",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let request = PlatformBugSubmitRequest {
            owner_wallet: canonical_public_key(&request.owner_wallet, "owner_wallet")?,
            message: normalize_bug_message(&request.message)?,
            authorization_signature: canonical_hex_signature(
                &request.authorization_signature,
                "authorization_signature",
            )?,
        };
        let response: PlatformBugSubmitResponse = self.post("v2/bugs", &request).await?;
        validate_platform_version(response.schema_version, &response.contract_version)?;
        if !valid_handle(&response.bug_id, "bug_") {
            return Err(SdkError::InvalidResponse(
                "bug submission returned an invalid report ID".to_owned(),
            ));
        }
        Ok(response)
    }

    /// Read one market's private account state after proving wallet control.
    /// The wallet signs an exact, server-time-bound read message outside Strata.
    pub async fn platform_account_market<S: AccountSigner + ?Sized>(
        &self,
        market_id: &str,
        signer: &S,
        request: PlatformAccountMarketRequest,
    ) -> Result<PlatformAccountSnapshotResponse, SdkError> {
        let discovery = self
            .require_platform_capability(
                "account.read",
                CapabilityRisk::Read,
                PlatformTransport::Http,
            )
            .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address =
            canonical_public_key(signer.public_key(), "account signer public key")?;
        let fill_limit = normalize_fill_limit(request.fill_limit)?;
        let timestamp_ms = discovery.server_time_ms;
        let message =
            account_http_auth_message(&market_id, &wallet_address, timestamp_ms, fill_limit)?;
        let signature = signer
            .sign_message(&message)
            .await
            .map_err(SdkError::Signer)?;
        if signature.len() != 64 {
            return Err(SdkError::Signer(
                "account signer must return a 64-byte Ed25519 signature".to_owned(),
            ));
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-strata-auth-time",
            HeaderValue::from_str(&timestamp_ms.to_string()).map_err(|_| {
                SdkError::InvalidRequest("account authorization time is invalid".to_owned())
            })?,
        );
        headers.insert(
            "x-strata-auth-signature",
            HeaderValue::from_str(&hex::encode(signature)).map_err(|_| {
                SdkError::InvalidRequest("account authorization signature is invalid".to_owned())
            })?,
        );
        let query = match request.fill_limit {
            Some(_) => vec![("fill_limit".to_owned(), fill_limit.to_string())],
            None => Vec::new(),
        };
        let response: PlatformAccountSnapshotResponse = self
            .get_with_headers(
                &format!("v2/markets/{market_id}/account/{wallet_address}"),
                &query,
                headers,
            )
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            &market_id,
        )?;
        if response.wallet_address != wallet_address {
            return Err(SdkError::InvalidResponse(
                "account response wallet does not match signed request".to_owned(),
            ));
        }
        account_stream::validate_account_state(&response.orders, &response.fills)?;
        Ok(response)
    }

    /// Read private order and fill state across selected markets, or across
    /// every currently discoverable market when `market_ids` is omitted.
    pub async fn platform_account_snapshot<S: AccountSigner + ?Sized>(
        &self,
        signer: &S,
        request: PlatformAccountRequest,
    ) -> Result<PlatformAccountSnapshot, SdkError> {
        let wallet_address =
            canonical_public_key(signer.public_key(), "account signer public key")?;
        let market_ids = match request.market_ids {
            Some(ids) => normalize_market_ids(ids)?,
            None => self.all_platform_market_ids().await?,
        };
        if market_ids.is_empty() {
            return Err(SdkError::OperationUnavailable(
                "no public markets are currently discoverable".to_owned(),
            ));
        }
        let mut markets = Vec::with_capacity(market_ids.len());
        for market_id in market_ids {
            markets.push(
                self.platform_account_market(
                    &market_id,
                    signer,
                    PlatformAccountMarketRequest {
                        fill_limit: request.fill_limit,
                    },
                )
                .await?,
            );
        }
        let server_time_ms = markets
            .iter()
            .map(|market| market.server_time_ms)
            .max()
            .unwrap_or_default();
        Ok(PlatformAccountSnapshot {
            wallet_address,
            server_time_ms,
            markets,
        })
    }

    /// A maker's products, exposure, health, and kill state in one market —
    /// public by wallet address, no signature.
    pub async fn platform_maker_status_for_wallet(
        &self,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<PlatformMakerStatusResponse, SdkError> {
        self.require_platform_capability(
            "mm.status.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        self.read_platform_maker_status(&market_id, &wallet_address, None)
            .await
    }

    /// Same read, addressed by a signer's public key. Reads are public, so the
    /// signer is never asked to sign; kept so existing callers keep compiling.
    pub async fn platform_maker_status<S: AccountSigner + ?Sized>(
        &self,
        market_id: &str,
        signer: &S,
    ) -> Result<PlatformMakerStatusResponse, SdkError> {
        self.platform_maker_status_for_wallet(market_id, signer.public_key())
            .await
    }

    /// Submit a detached external signature for the maker status read. Reads
    /// are public now; a signed request is still accepted (deprecated path).
    pub async fn platform_maker_status_authorized(
        &self,
        request: PlatformMakerStatusAuthorizedRequest,
    ) -> Result<PlatformMakerStatusResponse, SdkError> {
        self.require_platform_capability(
            "mm.status.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(&request.market_id)?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let signature =
            canonical_hex_signature(&request.authorization_signature, "authorization_signature")?;
        self.read_platform_maker_status(
            &market_id,
            &wallet_address,
            Some((request.authorization_time_ms, signature.as_str())),
        )
        .await
    }

    async fn read_platform_maker_status(
        &self,
        market_id: &str,
        wallet_address: &str,
        authorization: Option<(u64, &str)>,
    ) -> Result<PlatformMakerStatusResponse, SdkError> {
        let headers = maker_auth_headers(authorization)?;
        let response: PlatformMakerStatusResponse = self
            .get_with_headers(
                &format!("v2/markets/{market_id}/makers/{wallet_address}"),
                &[],
                headers,
            )
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            market_id,
        )?;
        if response.wallet_address != wallet_address {
            return Err(SdkError::InvalidResponse(
                "maker status wallet does not match signed request".to_owned(),
            ));
        }
        validate_maker_status(&response)?;
        Ok(response)
    }

    /// A maker's reliability record in one market — public by wallet address,
    /// no signature.
    pub async fn platform_maker_reputation_for_wallet(
        &self,
        market_id: &str,
        wallet_address: &str,
    ) -> Result<PlatformMakerReputationResponse, SdkError> {
        self.require_platform_capability(
            "mm.reputation.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
        self.read_platform_maker_reputation(&market_id, &wallet_address, None)
            .await
    }

    /// Same read, addressed by a signer's public key; the signer is never asked
    /// to sign (reads are public). Kept so existing callers keep compiling.
    pub async fn platform_maker_reputation<S: AccountSigner + ?Sized>(
        &self,
        market_id: &str,
        signer: &S,
    ) -> Result<PlatformMakerReputationResponse, SdkError> {
        self.platform_maker_reputation_for_wallet(market_id, signer.public_key())
            .await
    }

    /// Submit a detached external signature. Reads are public now; a signed
    /// request is still accepted (deprecated path).
    pub async fn platform_maker_reputation_authorized(
        &self,
        request: PlatformMakerReputationAuthorizedRequest,
    ) -> Result<PlatformMakerReputationResponse, SdkError> {
        self.require_platform_capability(
            "mm.reputation.read",
            CapabilityRisk::Read,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(&request.market_id)?;
        let wallet_address = canonical_public_key(&request.wallet_address, "wallet_address")?;
        let signature =
            canonical_hex_signature(&request.authorization_signature, "authorization_signature")?;
        self.read_platform_maker_reputation(
            &market_id,
            &wallet_address,
            Some((request.authorization_time_ms, signature.as_str())),
        )
        .await
    }

    async fn read_platform_maker_reputation(
        &self,
        market_id: &str,
        wallet_address: &str,
        authorization: Option<(u64, &str)>,
    ) -> Result<PlatformMakerReputationResponse, SdkError> {
        let headers = maker_auth_headers(authorization)?;
        let response: PlatformMakerReputationResponse = self
            .get_with_headers(
                &format!("v2/markets/{market_id}/makers/{wallet_address}/reputation"),
                &[],
                headers,
            )
            .await?;
        validate_platform_market_response(
            response.schema_version,
            &response.contract_version,
            &response.market_id,
            market_id,
        )?;
        if response.wallet_address != wallet_address {
            return Err(SdkError::InvalidResponse(
                "maker reputation wallet does not match signed request".to_owned(),
            ));
        }
        validate_maker_reputation(&response)?;
        Ok(response)
    }

    /// Prepare one exact maker-signed Strand transaction. Strata never sees
    /// the maker's private key and the returned packet has one signature slot.
    pub async fn platform_maker_strand_prepare(
        &self,
        market_id: &str,
        request: PlatformMakerStrandPrepareRequest,
    ) -> Result<PlatformMakerControlPrepareResponse, SdkError> {
        self.require_platform_capability(
            "mm.strand.manage",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let expected_action = strand_prepare_action(&request);
        let expected_wallet = strand_prepare_wallet(&request)?;
        let request = normalize_strand_prepare_request(request)?;
        let prepared: PlatformMakerControlPrepareResponse = self
            .post(
                &format!("v2/markets/{market_id}/makers/strands/prepare?transaction_version=0"),
                &request,
            )
            .await?;
        validate_maker_control_prepare(
            &prepared,
            &market_id,
            &expected_wallet,
            PlatformMakerControlProduct::Strand,
            expected_action,
        )?;
        Ok(prepared)
    }

    /// Prepare one exact maker-signed Current transaction. Upsert prices its
    /// bands from the market's live Strata mark; cancel stays usable.
    pub async fn platform_maker_current_prepare(
        &self,
        market_id: &str,
        request: PlatformMakerCurrentPrepareRequest,
    ) -> Result<PlatformMakerControlPrepareResponse, SdkError> {
        self.require_platform_capability(
            "mm.current.manage",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let expected_action = current_prepare_action(&request);
        let expected_wallet = current_prepare_wallet(&request)?;
        let request = normalize_current_prepare_request(request)?;
        let prepared: PlatformMakerControlPrepareResponse = self
            .post(
                &format!("v2/markets/{market_id}/makers/currents/prepare?transaction_version=0"),
                &request,
            )
            .await?;
        validate_maker_control_prepare(
            &prepared,
            &market_id,
            &expected_wallet,
            PlatformMakerControlProduct::Current,
            expected_action,
        )?;
        Ok(prepared)
    }

    pub async fn platform_maker_strand_submit(
        &self,
        market_id: &str,
        request: PlatformMakerControlSubmitRequest,
    ) -> Result<PlatformMakerControlSubmitResponse, SdkError> {
        self.platform_maker_control_submit(
            market_id,
            "strands",
            PlatformMakerControlProduct::Strand,
            request,
        )
        .await
    }

    pub async fn platform_maker_current_submit(
        &self,
        market_id: &str,
        request: PlatformMakerControlSubmitRequest,
    ) -> Result<PlatformMakerControlSubmitResponse, SdkError> {
        self.platform_maker_control_submit(
            market_id,
            "currents",
            PlatformMakerControlProduct::Current,
            request,
        )
        .await
    }

    async fn platform_maker_control_submit(
        &self,
        market_id: &str,
        product_path: &str,
        expected_product: PlatformMakerControlProduct,
        request: PlatformMakerControlSubmitRequest,
    ) -> Result<PlatformMakerControlSubmitResponse, SdkError> {
        let capability_id = match expected_product {
            PlatformMakerControlProduct::Strand => "mm.strand.manage",
            PlatformMakerControlProduct::Current => "mm.current.manage",
        };
        self.require_platform_capability(
            capability_id,
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        if !valid_handle(&request.maker_control_id, "mc_") {
            return Err(SdkError::InvalidRequest(
                "maker_control_id is invalid".to_owned(),
            ));
        }
        let request = PlatformMakerControlSubmitRequest {
            maker_control_id: request.maker_control_id,
            signed_transaction_base64: canonical_base64(
                &request.signed_transaction_base64,
                "signed_transaction_base64",
            )?,
            idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
        };
        let submitted: PlatformMakerControlSubmitResponse = self
            .post(
                &format!("v2/markets/{market_id}/makers/{product_path}/submit"),
                &request,
            )
            .await?;
        validate_platform_version(submitted.schema_version, &submitted.contract_version)?;
        if submitted.market_id != market_id
            || submitted.maker_control_id != request.maker_control_id
            || submitted.product != expected_product
            || submitted.status != PlatformMakerControlSubmissionStatus::Submitted
        {
            return Err(SdkError::InvalidResponse(
                "maker-control receipt is invalid".to_owned(),
            ));
        }
        canonical_public_key(&submitted.maker_wallet, "maker_wallet")?;
        canonical_signature(&submitted.signature, "signature")?;
        Ok(submitted)
    }

    /// Wait through a brief restart until a market is active with a fresh
    /// Strata mark, then return its resolved public identity.
    pub async fn platform_wait_for_maker_market(
        &self,
        reference: &str,
        timeout: Duration,
    ) -> Result<PlatformMarket, SdkError> {
        if timeout.is_zero() || timeout > Duration::from_secs(300) {
            return Err(SdkError::InvalidRequest(
                "maker readiness timeout must be between 1ms and 300s".to_owned(),
            ));
        }
        let market = self.platform_resolve_market(reference).await?;
        let deadline = Instant::now() + timeout;
        loop {
            let readiness = tokio::try_join!(
                self.platform_market_status(&market.market_id),
                self.platform_mark(&market.market_id),
            );
            match readiness {
                Ok((status, mark))
                    if status.status == PlatformMarketState::Active
                        && !mark.stale
                        && mark.price_atoms_per_base_unit.is_some() =>
                {
                    return Ok(market)
                }
                Err(
                    error @ SdkError::Api {
                        retryable: false, ..
                    },
                ) => return Err(error),
                Err(error)
                    if !matches!(
                        error,
                        SdkError::Api {
                            retryable: true,
                            ..
                        }
                    ) =>
                {
                    return Err(error)
                }
                _ => {}
            }
            if Instant::now() >= deadline {
                return Err(SdkError::OperationUnavailable(format!(
                    "market did not become active with a fresh Strata mark: {reference}"
                )));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Resolve human inputs and prepare one exact maker-owned transaction.
    pub async fn platform_maker_quickstart_prepare(
        &self,
        maker_wallet: &str,
        request: &PlatformMakerQuickstartRequest,
    ) -> Result<PlatformMakerQuickstartPrepared, SdkError> {
        let maker_wallet = canonical_public_key(maker_wallet, "maker_wallet")?;
        let market = self
            .platform_wait_for_maker_market(&request.market, Duration::from_secs(30))
            .await?;
        let base_asset = self.platform_resolve_asset(&market.base_asset_id).await?;
        let (market_status, mark, maker_status) = tokio::try_join!(
            self.platform_market_status(&market.market_id),
            self.platform_mark(&market.market_id),
            self.platform_maker_status_for_wallet(&market.market_id, &maker_wallet),
        )?;
        let mark_price = mark
            .price_atoms_per_base_unit
            .as_deref()
            .ok_or_else(|| SdkError::OperationUnavailable("Strata mark is stale".to_owned()))?
            .parse::<u64>()
            .map_err(|_| SdkError::InvalidResponse("Strata mark exceeds u64".to_owned()))?;
        let tick_size = market_status
            .tick_size_atoms
            .parse::<u64>()
            .map_err(|_| SdkError::InvalidResponse("market tick size exceeds u64".to_owned()))?;
        let current_slot = maker_status
            .current_slot
            .parse::<u64>()
            .map_err(|_| SdkError::InvalidResponse("maker slot exceeds u64".to_owned()))?;
        let operation = maker_quickstart_operation(
            &maker_wallet,
            request,
            &base_asset,
            &market.label,
            current_slot,
            mark_price,
            tick_size,
        )?;
        let prepared = match &operation {
            PlatformMakerQuickstartOperation::Strand(operation) => {
                self.platform_maker_strand_prepare(&market.market_id, operation.clone())
                    .await?
            }
            PlatformMakerQuickstartOperation::Current(operation) => {
                self.platform_maker_current_prepare(&market.market_id, operation.clone())
                    .await?
            }
        };
        let result = PlatformMakerQuickstartPrepared {
            market,
            base_asset: Some(base_asset),
            product: request.product,
            operation,
            prepared,
        };
        verify_maker_transaction(&MakerVerificationContext {
            market_id: &result.market.market_id,
            maker_wallet: &maker_wallet,
            operation: &result.operation,
            prepared: &result.prepared,
        })
        .map_err(SdkError::Verification)?;
        Ok(result)
    }

    /// Prepare a label-aware Strand or Current cancellation.
    pub async fn platform_maker_stop_prepare(
        &self,
        market: &str,
        product: PlatformMakerControlProduct,
        maker_wallet: &str,
    ) -> Result<PlatformMakerQuickstartPrepared, SdkError> {
        let maker_wallet = canonical_public_key(maker_wallet, "maker_wallet")?;
        let market = self.platform_resolve_market(market).await?;
        let operation = match product {
            PlatformMakerControlProduct::Strand => PlatformMakerQuickstartOperation::Strand(
                PlatformMakerStrandPrepareRequest::Cancel {
                    maker_wallet: maker_wallet.clone(),
                },
            ),
            PlatformMakerControlProduct::Current => PlatformMakerQuickstartOperation::Current(
                PlatformMakerCurrentPrepareRequest::Cancel {
                    maker_wallet: maker_wallet.clone(),
                },
            ),
        };
        let prepared = match &operation {
            PlatformMakerQuickstartOperation::Strand(operation) => {
                self.platform_maker_strand_prepare(&market.market_id, operation.clone())
                    .await?
            }
            PlatformMakerQuickstartOperation::Current(operation) => {
                self.platform_maker_current_prepare(&market.market_id, operation.clone())
                    .await?
            }
        };
        let result = PlatformMakerQuickstartPrepared {
            market,
            base_asset: None,
            product,
            operation,
            prepared,
        };
        verify_maker_transaction(&MakerVerificationContext {
            market_id: &result.market.market_id,
            maker_wallet: &maker_wallet,
            operation: &result.operation,
            prepared: &result.prepared,
        })
        .map_err(SdkError::Verification)?;
        Ok(result)
    }

    /// Submit an externally signed quickstart preparation and wait until the
    /// matcher's chain-derived maker state observes the exact start or stop.
    pub async fn platform_maker_submit_prepared(
        &self,
        prepared: &PlatformMakerQuickstartPrepared,
        signed_transaction_base64: &str,
        idempotency_key: Option<&str>,
        confirmation_timeout: Option<Duration>,
    ) -> Result<PlatformMakerQuickstartResult, SdkError> {
        verify_maker_transaction(&MakerVerificationContext {
            market_id: &prepared.market.market_id,
            maker_wallet: prepared.operation.maker_wallet(),
            operation: &prepared.operation,
            prepared: &prepared.prepared,
        })
        .map_err(SdkError::Verification)?;
        let signed_transaction_base64 =
            canonical_base64(signed_transaction_base64, "signed_transaction_base64")?;
        verify_signed_transaction_message(
            &prepared.prepared.transaction_base64,
            &signed_transaction_base64,
        )
        .map_err(SdkError::Verification)?;
        let request = PlatformMakerControlSubmitRequest {
            maker_control_id: prepared.prepared.maker_control_id.clone(),
            signed_transaction_base64,
            idempotency_key: normalize_idempotency_key(
                idempotency_key.unwrap_or(&prepared.prepared.maker_control_id),
            )?,
        };
        let receipt = match prepared.product {
            PlatformMakerControlProduct::Strand => {
                self.platform_maker_strand_submit(&prepared.market.market_id, request)
                    .await?
            }
            PlatformMakerControlProduct::Current => {
                self.platform_maker_current_submit(&prepared.market.market_id, request)
                    .await?
            }
        };
        let maker_status = self
            .wait_for_maker_product(
                &prepared.market.market_id,
                prepared.operation.maker_wallet(),
                &prepared.operation,
                !prepared.operation.is_cancel(),
                confirmation_timeout.unwrap_or(Duration::from_secs(45)),
                &receipt.signature,
            )
            .await?;
        Ok(PlatformMakerQuickstartResult {
            prepared: prepared.clone(),
            receipt,
            maker_status,
        })
    }

    /// One-call Rust maker start: resolve, prepare, verify, externally sign,
    /// submit idempotently, and return only after chain-derived confirmation.
    pub async fn platform_maker_start<S: MakerTransactionSigner + ?Sized>(
        &self,
        request: &PlatformMakerQuickstartRequest,
        signer: &S,
        confirmation_timeout: Option<Duration>,
    ) -> Result<PlatformMakerQuickstartResult, SdkError> {
        let maker_wallet = canonical_public_key(signer.public_key(), "maker_wallet")?;
        let prepared = self
            .platform_maker_quickstart_prepare(&maker_wallet, request)
            .await?;
        verify_maker_transaction(&MakerVerificationContext {
            market_id: &prepared.market.market_id,
            maker_wallet: &maker_wallet,
            operation: &prepared.operation,
            prepared: &prepared.prepared,
        })
        .map_err(SdkError::Verification)?;
        let signed = signer
            .sign_transaction(&prepared.prepared.transaction_base64)
            .await
            .map_err(SdkError::Signer)?;
        self.platform_maker_submit_prepared(&prepared, &signed, None, confirmation_timeout)
            .await
    }

    /// Idempotent one-call stop. If the product is already absent no wallet
    /// prompt or transaction is produced.
    pub async fn platform_maker_stop<S: MakerTransactionSigner + ?Sized>(
        &self,
        market: &str,
        product: PlatformMakerControlProduct,
        signer: &S,
        confirmation_timeout: Option<Duration>,
    ) -> Result<PlatformMakerStopResult, SdkError> {
        let maker_wallet = canonical_public_key(signer.public_key(), "maker_wallet")?;
        let market = self.platform_resolve_market(market).await?;
        let before = self
            .platform_maker_status_for_wallet(&market.market_id, &maker_wallet)
            .await?;
        if !maker_product_present(&before, product) {
            return Ok(PlatformMakerStopResult {
                market,
                product,
                prepared: None,
                receipt: None,
                maker_status: before,
                already_stopped: true,
            });
        }
        let prepared = self
            .platform_maker_stop_prepare(&market.market_id, product, &maker_wallet)
            .await?;
        verify_maker_transaction(&MakerVerificationContext {
            market_id: &market.market_id,
            maker_wallet: &maker_wallet,
            operation: &prepared.operation,
            prepared: &prepared.prepared,
        })
        .map_err(SdkError::Verification)?;
        let signed = signer
            .sign_transaction(&prepared.prepared.transaction_base64)
            .await
            .map_err(SdkError::Signer)?;
        let completed = self
            .platform_maker_submit_prepared(&prepared, &signed, None, confirmation_timeout)
            .await?;
        Ok(PlatformMakerStopResult {
            market,
            product,
            prepared: Some(prepared),
            receipt: Some(completed.receipt),
            maker_status: completed.maker_status,
            already_stopped: false,
        })
    }

    async fn wait_for_maker_product(
        &self,
        market_id: &str,
        maker_wallet: &str,
        operation: &PlatformMakerQuickstartOperation,
        present: bool,
        timeout: Duration,
        signature: &str,
    ) -> Result<PlatformMakerStatusResponse, SdkError> {
        if timeout.is_zero() || timeout > Duration::from_secs(300) {
            return Err(SdkError::InvalidRequest(
                "maker confirmation timeout must be between 1ms and 300s".to_owned(),
            ));
        }
        let deadline = Instant::now() + timeout;
        loop {
            match self
                .platform_maker_status_for_wallet(market_id, maker_wallet)
                .await
            {
                Ok(status)
                    if (present && maker_product_matches(&status, operation))
                        || (!present
                            && !maker_product_present(
                                &status,
                                match operation {
                                    PlatformMakerQuickstartOperation::Strand(_) => {
                                        PlatformMakerControlProduct::Strand
                                    }
                                    PlatformMakerQuickstartOperation::Current(_) => {
                                        PlatformMakerControlProduct::Current
                                    }
                                },
                            )) =>
                {
                    return Ok(status)
                }
                Err(SdkError::Api {
                    retryable: true, ..
                }) => {}
                Err(error) => return Err(error),
                Ok(_) => {}
            }
            if Instant::now() >= deadline {
                return Err(SdkError::OperationUnavailable(format!(
                    "maker transaction {signature} was submitted but not observed before timeout"
                )));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
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

    /// Return the live operation topology, including capability-gated nodes and
    /// the points where the agent owner's signer acts outside Strata.
    pub async fn action_graph(&self) -> Result<ActionGraph, SdkError> {
        let graph: ActionGraph = self.get("sonar/action-graph", &[]).await?;
        validate_action_graph(&graph)?;
        Ok(graph)
    }

    pub async fn markets(&self) -> Result<MarketsResponse, SdkError> {
        let markets: MarketsResponse = self.get("sonar/markets", &[]).await?;
        validate_version(markets.schema_version, &markets.contract_version)?;
        Ok(markets)
    }

    /// Request a short-lived Sonar quote by human market label or market ID.
    /// Give exactly one of `amount_in_atoms` (spend this much) or
    /// `amount_out_atoms` (receive this much; Strata resolves the input).
    /// `maximum_tolerance_bps` is the most you accept below the quoted output
    /// (default 0); it is your choice and unrelated to the measured
    /// `price_impact_pct` the response reports.
    pub async fn quote(&self, request: QuoteRequest) -> Result<QuoteResponse, SdkError> {
        let target = quote_target(&request)?;
        if request.maximum_tolerance_bps > 1_000 {
            return Err(SdkError::InvalidRequest(
                "maximum_tolerance_bps must be between 0 and 1,000".to_owned(),
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
            amount_in_atoms: matches!(target, QuoteTarget::ExactInput(_))
                .then(|| target.amount().to_string()),
            amount_out_atoms: matches!(target, QuoteTarget::ExactOutput(_))
                .then(|| target.amount().to_string()),
            maximum_tolerance_bps: request.maximum_tolerance_bps,
        };
        let quote: QuoteResponse = self.post(quote_path, &wire).await?;
        validate_quote(&quote, market_pda, &request, target)?;
        Ok(quote)
    }

    /// Request canonical authorization bytes for an external signer. This
    /// operation accepts public identity only; signing material stays external.
    pub async fn execution_challenge(
        &self,
        market: &str,
        request: ExecutionChallengeRequest,
    ) -> Result<ExecutionChallengeResponse, SdkError> {
        let request = normalize_execution_challenge_request(request)?;
        let execution_path = self.execution_path(market).await?;
        let challenge: ExecutionChallengeResponse = self
            .post(&format!("{execution_path}/challenge"), &request)
            .await?;
        validate_version(challenge.schema_version, &challenge.contract_version)?;
        if !valid_handle(&challenge.challenge_id, "sc_") || challenge.quote_id != request.quote_id {
            return Err(SdkError::InvalidResponse(
                "execution challenge does not match the requested quote".to_owned(),
            ));
        }
        Ok(challenge)
    }

    /// Prepare a quote-bound, partially signed transaction: either exchange
    /// an external authorization signature (`Authorized`, two-step path) or
    /// bind the quote directly (`Direct`, one signature — the session's
    /// transaction signature is the authorization).
    pub async fn execution_prepare(
        &self,
        market: &str,
        request: ExecutionPrepareRequest,
    ) -> Result<ExecutionPrepareResponse, SdkError> {
        let request = match request {
            ExecutionPrepareRequest::Authorized(authorization) => {
                if !valid_handle(&authorization.challenge_id, "sc_") {
                    return Err(SdkError::InvalidRequest(
                        "challenge_id is invalid".to_owned(),
                    ));
                }
                let signature = bs58::decode(authorization.authorization_signature.trim())
                    .into_vec()
                    .map_err(|_| {
                        SdkError::InvalidRequest(
                            "authorization_signature must be base58".to_owned(),
                        )
                    })?;
                if signature.len() != 64
                    || bs58::encode(&signature).into_string()
                        != authorization.authorization_signature.trim()
                {
                    return Err(SdkError::InvalidRequest(
                        "authorization_signature must be a canonical Ed25519 signature".to_owned(),
                    ));
                }
                ExecutionPrepareRequest::Authorized(ExecutionPrepareAuthorization {
                    challenge_id: authorization.challenge_id,
                    authorization_signature: bs58::encode(signature).into_string(),
                })
            }
            ExecutionPrepareRequest::Direct(binding) => {
                ExecutionPrepareRequest::Direct(normalize_execution_challenge_request(binding)?)
            }
        };
        let execution_path = self.execution_path(market).await?;
        let prepared: ExecutionPrepareResponse = self
            .post(&format!("{execution_path}/prepare"), &request)
            .await?;
        validate_version(prepared.schema_version, &prepared.contract_version)?;
        if !valid_handle(&prepared.execution_id, "se_") {
            return Err(SdkError::InvalidResponse(
                "prepared execution ID is invalid".to_owned(),
            ));
        }
        if let ExecutionPrepareRequest::Direct(binding) = &request {
            if prepared.quote_id != binding.quote_id {
                return Err(SdkError::InvalidResponse(
                    "prepared execution does not match the requested quote".to_owned(),
                ));
            }
        }
        Ok(prepared)
    }

    /// Submit an externally signed transaction. Reusing the same idempotency
    /// key cannot create a second execution.
    pub async fn execution_submit(
        &self,
        market: &str,
        request: ExecutionSubmitRequest,
    ) -> Result<ExecutionSubmitResponse, SdkError> {
        if !valid_handle(&request.execution_id, "se_") {
            return Err(SdkError::InvalidRequest(
                "execution_id is invalid".to_owned(),
            ));
        }
        let transaction = request.signed_transaction_base64.trim();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(transaction)
            .map_err(|_| {
                SdkError::InvalidRequest(
                    "signed_transaction_base64 must be canonical base64".to_owned(),
                )
            })?;
        if decoded.is_empty()
            || base64::engine::general_purpose::STANDARD.encode(&decoded) != transaction
        {
            return Err(SdkError::InvalidRequest(
                "signed_transaction_base64 must be canonical base64".to_owned(),
            ));
        }
        let request = ExecutionSubmitRequest {
            execution_id: request.execution_id,
            signed_transaction_base64: transaction.to_owned(),
            idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
        };
        let execution_path = self.execution_path(market).await?;
        let submitted: ExecutionSubmitResponse = self
            .post(&format!("{execution_path}/submit"), &request)
            .await?;
        validate_version(submitted.schema_version, &submitted.contract_version)?;
        if submitted.execution_id != request.execution_id
            || submitted.status != ExecutionStatus::Submitted
            || submitted.signature.trim().is_empty()
        {
            return Err(SdkError::InvalidResponse(
                "execution receipt does not match the submitted transaction".to_owned(),
            ));
        }
        Ok(submitted)
    }

    /// Request exact authorization bytes for one product-level resting-order
    /// operation. Private key material never enters this client or Strata.
    pub async fn order_challenge(
        &self,
        market_id: &str,
        request: PlatformOrderChallengeRequest,
    ) -> Result<PlatformOrderChallengeResponse, SdkError> {
        self.require_platform_capability(
            "orders.prepare",
            CapabilityRisk::Prepare,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let request = normalize_order_challenge_request(request)?;
        let expected_action = order_request_action(&request);
        let challenge: PlatformOrderChallengeResponse = self
            .post(
                &format!("v2/markets/{market_id}/orders/challenge"),
                &request,
            )
            .await?;
        validate_platform_version(challenge.schema_version, &challenge.contract_version)?;
        if challenge.market_id != market_id
            || challenge.action != expected_action
            || !valid_handle(&challenge.challenge_id, "oc_")
            || challenge.order_ids.is_empty()
            || challenge.order_ids.len() > 12
            || challenge.expires_at_ms <= challenge.server_time_ms
            || challenge
                .order_ids
                .iter()
                .any(|order_id| !valid_handle(order_id, "order_"))
        {
            return Err(SdkError::InvalidResponse(
                "order challenge bindings are invalid".to_owned(),
            ));
        }
        canonical_base64(
            &challenge.authorization_payload_base64,
            "authorization_payload_base64",
        )?;
        Ok(challenge)
    }

    /// Prepare a backend-partially-signed v0 order-control transaction:
    /// either hand back a signed challenge (`Authorized`, two-step path) or
    /// send the operation itself (`Direct`, one signature — Strata builds the
    /// transaction from the operation and the session's signature over that
    /// transaction is the whole authorization).
    pub async fn order_prepare(
        &self,
        market_id: &str,
        request: PlatformOrderPrepareRequest,
    ) -> Result<PlatformOrderPrepareResponse, SdkError> {
        self.require_platform_capability(
            "orders.prepare",
            CapabilityRisk::Prepare,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let request = match request {
            PlatformOrderPrepareRequest::Authorized(authorization) => {
                PlatformOrderPrepareRequest::Authorized(normalize_order_prepare_authorization(
                    authorization,
                )?)
            }
            PlatformOrderPrepareRequest::Direct(operation) => {
                PlatformOrderPrepareRequest::Direct(normalize_order_challenge_request(operation)?)
            }
        };
        let prepared: PlatformOrderPrepareResponse = self
            .post(&format!("v2/markets/{market_id}/orders/prepare"), &request)
            .await?;
        validate_platform_version(prepared.schema_version, &prepared.contract_version)?;
        if prepared.market_id != market_id
            || !valid_handle(&prepared.order_control_id, "or_")
            || prepared.order_ids.is_empty()
            || prepared.order_ids.len() > 12
            || prepared.transaction_base64.trim().is_empty()
            || prepared.expires_at_ms == 0
        {
            return Err(SdkError::InvalidResponse(
                "prepared order control is invalid".to_owned(),
            ));
        }
        canonical_base64(&prepared.transaction_base64, "transaction_base64")?;
        canonical_base58_32(&prepared.recent_blockhash, "recent_blockhash")?;
        if let PlatformOrderPrepareRequest::Direct(operation) = &request {
            if prepared.action != order_request_action(operation) {
                return Err(SdkError::InvalidResponse(
                    "prepared order action does not match request".to_owned(),
                ));
            }
        }
        Ok(prepared)
    }

    /// Submit an externally signed order-control transaction. The same
    /// control ID and idempotency key return the same receipt.
    pub async fn order_submit(
        &self,
        market_id: &str,
        request: PlatformOrderSubmitRequest,
    ) -> Result<PlatformOrderSubmitResponse, SdkError> {
        self.require_platform_capability(
            "orders.submit",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        if !valid_handle(&request.order_control_id, "or_") {
            return Err(SdkError::InvalidRequest(
                "order_control_id is invalid".to_owned(),
            ));
        }
        let transaction = canonical_base64(
            &request.signed_transaction_base64,
            "signed_transaction_base64",
        )?;
        let request = PlatformOrderSubmitRequest {
            order_control_id: request.order_control_id,
            signed_transaction_base64: transaction,
            idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
        };
        let submitted: PlatformOrderSubmitResponse = self
            .post(&format!("v2/markets/{market_id}/orders/submit"), &request)
            .await?;
        validate_platform_version(submitted.schema_version, &submitted.contract_version)?;
        if submitted.market_id != market_id
            || submitted.order_control_id != request.order_control_id
            || submitted.status != PlatformOrderSubmissionStatus::Submitted
            || submitted.signature.trim().is_empty()
        {
            return Err(SdkError::InvalidResponse(
                "order control receipt is invalid".to_owned(),
            ));
        }
        canonical_signature(&submitted.signature, "signature")?;
        Ok(submitted)
    }

    /// Recover the durable result for a prior submission. The same opaque
    /// control ID and idempotency key are required, so status polling never
    /// broadens authority beyond the original external submission.
    pub async fn order_status(
        &self,
        market_id: &str,
        request: PlatformOrderStatusRequest,
    ) -> Result<PlatformOrderStatusResponse, SdkError> {
        self.require_platform_capability(
            "orders.submit",
            CapabilityRisk::Submit,
            PlatformTransport::Http,
        )
        .await?;
        let market_id = validate_platform_market_id(market_id)?;
        if !valid_handle(&request.order_control_id, "or_") {
            return Err(SdkError::InvalidRequest(
                "order_control_id is invalid".to_owned(),
            ));
        }
        let request = PlatformOrderStatusRequest {
            order_control_id: request.order_control_id,
            idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
        };
        let status: PlatformOrderStatusResponse = self
            .post(&format!("v2/markets/{market_id}/orders/status"), &request)
            .await?;
        validate_platform_version(status.schema_version, &status.contract_version)?;
        if status.market_id != market_id
            || status.order_control_id != request.order_control_id
            || status.order_ids.is_empty()
            || status.order_ids.len() > 12
            || status
                .order_ids
                .iter()
                .any(|order_id| !valid_handle(order_id, "order_"))
            || (status.status == PlatformOrderControlStatus::Failed
                && status.failure_code.as_deref().is_none_or(str::is_empty))
            || (status.status != PlatformOrderControlStatus::Failed
                && status.failure_code.is_some())
        {
            return Err(SdkError::InvalidResponse(
                "order control status is invalid".to_owned(),
            ));
        }
        canonical_signature(&status.signature, "signature")?;
        Ok(status)
    }

    /// Request exact authorization bytes for one bounded TWAP placement or
    /// cancellation. The session private key remains outside Strata.
    pub async fn twap_challenge(
        &self,
        market_id: &str,
        request: PlatformTwapChallengeRequest,
    ) -> Result<PlatformTwapChallengeResponse, SdkError> {
        let capability_id = match twap_request_action(&request) {
            PlatformTwapControlAction::Place => ("algos.twap.place", CapabilityRisk::Submit),
            PlatformTwapControlAction::Cancel => ("algos.twap.cancel", CapabilityRisk::Destructive),
        };
        self.require_platform_capability(capability_id.0, capability_id.1, PlatformTransport::Http)
            .await?;
        let market_id = validate_platform_market_id(market_id)?;
        let request = normalize_twap_challenge_request(request)?;
        let expected_action = twap_request_action(&request);
        let challenge: PlatformTwapChallengeResponse = self
            .post(&format!("v2/markets/{market_id}/twaps/challenge"), &request)
            .await?;
        validate_platform_version(challenge.schema_version, &challenge.contract_version)?;
        if challenge.market_id != market_id
            || challenge.action != expected_action
            || !valid_handle(&challenge.challenge_id, "twc_")
            || !valid_handle(&challenge.twap_id, "twap_")
            || challenge.expires_at_ms <= challenge.server_time_ms
        {
            return Err(SdkError::InvalidResponse(
                "TWAP challenge bindings are invalid".to_owned(),
            ));
        }
        canonical_base64(
            &challenge.authorization_payload_base64,
            "authorization_payload_base64",
        )?;
        Ok(challenge)
    }

    /// Prepare a backend-partially-signed TWAP-control transaction that the
    /// external session must verify: either the exact detached TWAP
    /// authorization (`Authorized`, two-step path) or the action itself
    /// (`Direct`, one signature — the transaction signature is the
    /// authorization).
    pub async fn twap_prepare(
        &self,
        market_id: &str,
        request: PlatformTwapPrepareRequest,
    ) -> Result<PlatformTwapPrepareResponse, SdkError> {
        if let PlatformTwapPrepareRequest::Direct(operation) = &request {
            let capability_id = match twap_request_action(operation) {
                PlatformTwapControlAction::Place => ("algos.twap.place", CapabilityRisk::Submit),
                PlatformTwapControlAction::Cancel => {
                    ("algos.twap.cancel", CapabilityRisk::Destructive)
                }
            };
            self.require_platform_capability(
                capability_id.0,
                capability_id.1,
                PlatformTransport::Http,
            )
            .await?;
        }
        let market_id = validate_platform_market_id(market_id)?;
        let request = match request {
            PlatformTwapPrepareRequest::Authorized(authorization) => {
                if !valid_handle(&authorization.challenge_id, "twc_") {
                    return Err(SdkError::InvalidRequest(
                        "TWAP challenge_id is invalid".to_owned(),
                    ));
                }
                PlatformTwapPrepareRequest::Authorized(PlatformTwapPrepareAuthorization {
                    challenge_id: authorization.challenge_id,
                    authorization_signature: canonical_signature(
                        &authorization.authorization_signature,
                        "authorization_signature",
                    )?,
                })
            }
            PlatformTwapPrepareRequest::Direct(operation) => {
                PlatformTwapPrepareRequest::Direct(normalize_twap_challenge_request(operation)?)
            }
        };
        let prepared: PlatformTwapPrepareResponse = self
            .post(&format!("v2/markets/{market_id}/twaps/prepare"), &request)
            .await?;
        validate_platform_version(prepared.schema_version, &prepared.contract_version)?;
        if prepared.market_id != market_id
            || !valid_handle(&prepared.twap_control_id, "twctl_")
            || !valid_handle(&prepared.twap_id, "twap_")
            || prepared.expires_at_ms == 0
        {
            return Err(SdkError::InvalidResponse(
                "prepared TWAP control is invalid".to_owned(),
            ));
        }
        canonical_base64(&prepared.transaction_base64, "transaction_base64")?;
        canonical_base58_32(&prepared.recent_blockhash, "recent_blockhash")?;
        if let PlatformTwapPrepareRequest::Direct(operation) = &request {
            if prepared.action != twap_request_action(operation) {
                return Err(SdkError::InvalidResponse(
                    "prepared TWAP action does not match request".to_owned(),
                ));
            }
        }
        Ok(prepared)
    }

    /// Submit one externally signed TWAP transaction idempotently.
    pub async fn twap_submit(
        &self,
        market_id: &str,
        request: PlatformTwapSubmitRequest,
    ) -> Result<PlatformTwapSubmitResponse, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        if !valid_handle(&request.twap_control_id, "twctl_") {
            return Err(SdkError::InvalidRequest(
                "twap_control_id is invalid".to_owned(),
            ));
        }
        let request = PlatformTwapSubmitRequest {
            twap_control_id: request.twap_control_id,
            signed_transaction_base64: canonical_base64(
                &request.signed_transaction_base64,
                "signed_transaction_base64",
            )?,
            idempotency_key: normalize_idempotency_key(&request.idempotency_key)?,
        };
        let submitted: PlatformTwapSubmitResponse = self
            .post(&format!("v2/markets/{market_id}/twaps/submit"), &request)
            .await?;
        validate_platform_version(submitted.schema_version, &submitted.contract_version)?;
        if submitted.market_id != market_id
            || submitted.twap_control_id != request.twap_control_id
            || !valid_handle(&submitted.twap_id, "twap_")
            || submitted.status != PlatformOrderSubmissionStatus::Submitted
        {
            return Err(SdkError::InvalidResponse(
                "TWAP control receipt is invalid".to_owned(),
            ));
        }
        canonical_signature(&submitted.signature, "signature")?;
        Ok(submitted)
    }

    /// Complete the externally signed TWAP flow with one signature: the
    /// action is bound and built in one step (direct prepare), the prepared
    /// bindings are checked against the request, the mandatory verifier runs
    /// (see [`DefaultTransactionVerifier`]), and only then is the session's
    /// transaction signature requested. `SessionSigner::sign_message` is not
    /// called on this path.
    pub async fn execute_twap<S, V>(
        &self,
        market_id: &str,
        operation: &TwapExecuteOperation,
        signer: &S,
        verifier: &V,
        idempotency_key: Option<&str>,
    ) -> Result<PlatformTwapSubmitResponse, SdkError>
    where
        S: SessionSigner + ?Sized,
        V: TwapVerifier + ?Sized,
    {
        let market_id = validate_platform_market_id(market_id)?;
        let session_public_key = canonical_public_key(signer.public_key(), "session_public_key")?;
        let request = normalize_twap_challenge_request(
            operation.challenge_request(session_public_key.clone()),
        )?;
        let owner_wallet = twap_request_owner(&request).to_owned();
        // One signature: the action is bound and built in one step and the
        // session signs only the resulting transaction.
        let prepared = self
            .twap_prepare(
                &market_id,
                PlatformTwapPrepareRequest::Direct(request.clone()),
            )
            .await?;
        validate_twap_direct_binding(&prepared, &request, &market_id)?;
        verifier
            .verify(&TwapVerificationContext {
                challenge: None,
                operation: &request,
                market_id: &market_id,
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
        let signed_transaction =
            canonical_base64(&signed_transaction, "signed_transaction_base64")?;
        verify_signed_transaction_message(&prepared.transaction_base64, &signed_transaction)
            .map_err(SdkError::Verification)?;
        self.twap_submit(
            &market_id,
            PlatformTwapSubmitRequest {
                twap_control_id: prepared.twap_control_id.clone(),
                signed_transaction_base64: signed_transaction,
                idempotency_key: normalize_idempotency_key(
                    idempotency_key.unwrap_or(&prepared.twap_control_id),
                )?,
            },
        )
        .await
    }

    /// Execute one resting-order operation with one signature while all
    /// private keys and signing policy remain in the caller's signer adapter.
    /// The operation is bound and built in one step (direct prepare), the
    /// prepared bindings are checked against the request, and the mandatory
    /// verifier (see [`DefaultTransactionVerifier`], which decodes the
    /// transaction and requires it to be exactly this operation) runs before
    /// the transaction signature is requested. `SessionSigner::sign_message`
    /// is not called on this path.
    pub async fn execute_order<S, V>(
        &self,
        market_id: &str,
        operation: &OrderExecuteOperation,
        signer: &S,
        verifier: &V,
        idempotency_key: Option<&str>,
    ) -> Result<PlatformOrderSubmitResponse, SdkError>
    where
        S: SessionSigner + ?Sized,
        V: OrderVerifier + ?Sized,
    {
        let market_id = validate_platform_market_id(market_id)?;
        let session_public_key = canonical_public_key(signer.public_key(), "session_public_key")?;
        let request = normalize_order_challenge_request(
            operation.challenge_request(session_public_key.clone()),
        )?;
        let owner_wallet = order_request_owner(&request).to_owned();
        if owner_wallet == session_public_key {
            return Err(SdkError::InvalidRequest(
                "session_public_key must be distinct from owner_wallet".to_owned(),
            ));
        }
        // One signature: the operation is bound and built in one step and the
        // session signs only the resulting transaction, after the verifier
        // has checked it is exactly this operation.
        let prepared = self
            .order_prepare(
                &market_id,
                PlatformOrderPrepareRequest::Direct(request.clone()),
            )
            .await?;
        validate_order_direct_binding(&prepared, &request, &market_id)?;
        verifier
            .verify(&OrderVerificationContext {
                challenge: None,
                operation: &request,
                market_id: &market_id,
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
        let signed_transaction =
            canonical_base64(&signed_transaction, "signed_transaction_base64")?;
        verify_signed_transaction_message(&prepared.transaction_base64, &signed_transaction)
            .map_err(SdkError::Verification)?;
        self.order_submit(
            &market_id,
            PlatformOrderSubmitRequest {
                order_control_id: prepared.order_control_id.clone(),
                signed_transaction_base64: signed_transaction,
                idempotency_key: normalize_idempotency_key(
                    idempotency_key.unwrap_or(&prepared.order_control_id),
                )?,
            },
        )
        .await
    }

    /// Execute one short-lived Sonar quote with one signature, without giving
    /// the SDK custody of a session private key. The quote is bound and built
    /// in one step (direct prepare), the prepared bindings are checked against
    /// the quote, and the transaction verifier (see
    /// [`DefaultTransactionVerifier`]) always runs before the session adapter
    /// is allowed to sign. `SessionSigner::sign_message` is not called on this
    /// path.
    pub async fn execute_quote<S, V>(
        &self,
        quote: &QuoteResponse,
        owner_wallet: &str,
        account_sequence: Option<u64>,
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
        // One signature: the quote is bound and built in one step and the
        // session signs only the resulting transaction.
        let prepared: ExecutionPrepareResponse = self
            .post(
                &format!("{execution_path}/prepare"),
                &ExecutionPrepareRequest::Direct(ExecutionChallengeRequest {
                    quote_id: quote.quote_id.clone(),
                    owner_wallet: owner_wallet.clone(),
                    session_public_key: session_public_key.clone(),
                    account_sequence: account_sequence.map(|value| value.to_string()),
                }),
            )
            .await?;
        validate_execution_direct_prepare(&prepared, quote)?;
        verifier
            .verify(&ExecutionVerificationContext {
                quote,
                challenge: None,
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
        let signed_transaction =
            canonical_base64(&signed_transaction, "signed_transaction_base64")?;
        verify_signed_transaction_message(&prepared.transaction_base64, &signed_transaction)
            .map_err(SdkError::Verification)?;
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
        query: &[(String, String)],
    ) -> Result<T, SdkError> {
        self.get_with_headers(path, query, HeaderMap::new()).await
    }

    async fn get_with_headers<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
        headers: HeaderMap,
    ) -> Result<T, SdkError> {
        let mut url = self.base_url.join(path).map_err(|error| {
            SdkError::InvalidBaseUrl(format!("could not join public operation: {error}"))
        })?;
        url.query_pairs_mut().extend_pairs(
            query
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );

        let response = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .headers(headers)
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

    async fn all_platform_market_ids(&self) -> Result<Vec<String>, SdkError> {
        let mut market_ids = Vec::new();
        let mut cursor = None;
        let mut seen_cursors = HashSet::new();
        loop {
            let response = self
                .platform_markets(PageRequest {
                    cursor: cursor.clone(),
                    limit: Some(MAX_PLATFORM_PAGE_SIZE),
                })
                .await?;
            market_ids.extend(response.markets.into_iter().map(|market| market.market_id));
            if !response.page.has_more {
                break;
            }
            let next = response.page.next_cursor.ok_or_else(|| {
                SdkError::InvalidResponse(
                    "market pagination omitted the required next cursor".to_owned(),
                )
            })?;
            if !seen_cursors.insert(next.clone()) {
                return Err(SdkError::InvalidResponse(
                    "market pagination repeated a cursor".to_owned(),
                ));
            }
            cursor = Some(next);
        }
        normalize_market_ids(market_ids)
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

    async fn execution_path(&self, requested_market: &str) -> Result<String, SdkError> {
        let markets = self.markets().await?;
        let market = markets
            .markets
            .iter()
            .find(|market| {
                market.label.eq_ignore_ascii_case(requested_market.trim())
                    || market.market_pda.as_deref() == Some(requested_market.trim())
            })
            .ok_or_else(|| SdkError::MarketNotFound(requested_market.to_owned()))?;
        if !market.ready {
            return Err(SdkError::OperationUnavailable(market.label.clone()));
        }
        let quote_path = market
            .quote_path
            .as_deref()
            .filter(|path| valid_public_operation_path(path))
            .ok_or_else(|| SdkError::OperationUnavailable(market.label.clone()))?;
        Ok(format!(
            "{}/execution",
            quote_path
                .strip_suffix("/quote")
                .ok_or_else(|| SdkError::OperationUnavailable(market.label.clone()))?
        ))
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

fn validate_action_graph(graph: &ActionGraph) -> Result<(), SdkError> {
    validate_version(graph.schema_version, &graph.contract_version)?;
    if graph.graph_version != "1.0"
        || graph.authority.permission_source != "external_agent_owner"
        || graph.authority.signing_location != "external"
        || graph.authority.accepts_private_keys
    {
        return Err(SdkError::InvalidResponse(
            "unsupported action graph authority model".to_owned(),
        ));
    }
    let ids = graph
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    if ids.len() != graph.nodes.len() || !ids.contains(graph.entry_node.as_str()) {
        return Err(SdkError::InvalidResponse(
            "action graph node IDs are invalid".to_owned(),
        ));
    }
    if graph.edges.iter().any(|edge| {
        !ids.contains(edge.from.as_str())
            || !ids.contains(edge.to.as_str())
            || edge.condition.trim().is_empty()
    }) {
        return Err(SdkError::InvalidResponse(
            "action graph contains an invalid edge".to_owned(),
        ));
    }
    Ok(())
}

fn validate_version(schema_version: u16, contract_version: &str) -> Result<(), SdkError> {
    if schema_version != CONTRACT_MAJOR || contract_version != CONTRACT_VERSION {
        return Err(SdkError::InvalidResponse(format!(
            "unsupported contract {contract_version} (schema {schema_version})"
        )));
    }
    Ok(())
}

fn validate_platform_version(schema_version: u16, contract_version: &str) -> Result<(), SdkError> {
    if schema_version != strata_public_contract::platform::PLATFORM_SCHEMA_VERSION
        || contract_version != strata_public_contract::platform::PLATFORM_CONTRACT_VERSION
    {
        return Err(SdkError::InvalidResponse(format!(
            "unsupported platform contract {contract_version} (schema {schema_version})"
        )));
    }
    Ok(())
}

fn validate_vault_preparation(preparation_id: &str, submit_by_ms: u64) -> Result<(), SdkError> {
    if !valid_handle(preparation_id, "vp_") || submit_by_ms == 0 {
        return Err(SdkError::InvalidResponse(
            "Vault preparation identity is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_vault_submission(
    response: &PlatformVaultSubmitResponse,
    preparation_id: &str,
) -> Result<(), SdkError> {
    validate_platform_version(response.schema_version, &response.contract_version)?;
    if response.preparation_id != preparation_id
        || (response.status == PlatformVaultSubmissionStatus::Failed)
            != response.failure_code.is_some()
        || response.failure_code.as_deref().is_some_and(|code| {
            code.len() < 3
                || code.len() > 64
                || !code
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
    {
        return Err(SdkError::InvalidResponse(
            "Vault submission receipt is invalid".to_owned(),
        ));
    }
    canonical_public_key(&response.wallet_address, "wallet_address")?;
    canonical_signature(&response.signature, "signature")?;
    Ok(())
}

fn strand_prepare_wallet_raw(request: &PlatformMakerStrandPrepareRequest) -> &str {
    match request {
        PlatformMakerStrandPrepareRequest::Upsert { maker_wallet, .. }
        | PlatformMakerStrandPrepareRequest::Recenter { maker_wallet, .. }
        | PlatformMakerStrandPrepareRequest::SetEnabled { maker_wallet, .. }
        | PlatformMakerStrandPrepareRequest::Cancel { maker_wallet } => maker_wallet,
    }
}

fn current_prepare_wallet_raw(request: &PlatformMakerCurrentPrepareRequest) -> &str {
    match request {
        PlatformMakerCurrentPrepareRequest::Upsert { maker_wallet, .. }
        | PlatformMakerCurrentPrepareRequest::Cancel { maker_wallet } => maker_wallet,
    }
}

fn maker_duration_slots(duration: Option<&str>) -> Result<u64, SdkError> {
    let duration = duration.unwrap_or("10m").trim();
    if duration.len() < 2 {
        return Err(SdkError::InvalidRequest(
            "duration must look like 30s, 10m, 2h, or 1d".to_owned(),
        ));
    }
    let (amount, unit) = duration.split_at(duration.len() - 1);
    let amount = amount.parse::<u64>().map_err(|_| {
        SdkError::InvalidRequest("duration must use a positive whole number".to_owned())
    })?;
    if amount == 0 {
        return Err(SdkError::InvalidRequest(
            "duration must be positive".to_owned(),
        ));
    }
    let scale = match unit.to_ascii_lowercase().as_str() {
        "s" => 1,
        "m" => 60,
        "h" => 3_600,
        "d" => 86_400,
        _ => {
            return Err(SdkError::InvalidRequest(
                "duration must look like 30s, 10m, 2h, or 1d".to_owned(),
            ))
        }
    };
    let seconds = amount
        .checked_mul(scale)
        .filter(|seconds| *seconds <= 604_800)
        .ok_or_else(|| SdkError::InvalidRequest("duration cannot exceed seven days".to_owned()))?;
    seconds
        .checked_mul(5)
        .and_then(|slots| slots.checked_add(1))
        .map(|slots| slots / 2)
        .ok_or_else(|| SdkError::InvalidRequest("duration exceeds slot range".to_owned()))
}

fn human_base_atoms(
    value: &str,
    asset: &PlatformAsset,
    market_label: &str,
) -> Result<u64, SdkError> {
    let market_symbol = market_label.split_once('/').and_then(|(base, quote)| {
        (!base.trim().is_empty() && !quote.trim().is_empty()).then(|| base.trim())
    });
    let display_symbol = market_symbol.unwrap_or(&asset.symbol);
    let fields: Vec<&str> = value.split_whitespace().collect();
    if fields.is_empty() || fields.len() > 2 {
        return Err(SdkError::InvalidRequest(format!(
            "size must be an exact {} amount, for example 0.01 {}",
            display_symbol, display_symbol
        )));
    }
    if fields.len() == 2
        && !fields[1].eq_ignore_ascii_case(&asset.symbol)
        && !market_symbol.is_some_and(|symbol| fields[1].eq_ignore_ascii_case(symbol))
    {
        return Err(SdkError::InvalidRequest(format!(
            "size is denominated in {}, not {}",
            display_symbol, fields[1]
        )));
    }
    let mut parts = fields[0].split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > usize::from(asset.decimals)
    {
        return Err(SdkError::InvalidRequest(format!(
            "size must have at most {} decimal places",
            asset.decimals
        )));
    }
    let scale = 10u128.pow(u32::from(asset.decimals));
    let whole = whole.parse::<u128>().map_err(|_| {
        SdkError::InvalidRequest("size exceeds the supported base-asset range".to_owned())
    })?;
    let mut padded = fraction.to_owned();
    padded.extend(std::iter::repeat_n(
        '0',
        usize::from(asset.decimals).saturating_sub(fraction.len()),
    ));
    let fractional = if padded.is_empty() {
        0
    } else {
        padded.parse::<u128>().map_err(|_| {
            SdkError::InvalidRequest("size has an invalid decimal fraction".to_owned())
        })?
    };
    let atoms = whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fractional))
        .filter(|value| *value > 0 && *value <= u128::from(u64::MAX))
        .ok_or_else(|| {
            SdkError::InvalidRequest("size is outside the supported base-asset range".to_owned())
        })?;
    Ok(atoms as u64)
}

fn split_maker_size(total: u64, levels: usize, width: usize) -> Vec<String> {
    let active = levels.min(total as usize).max(1);
    let quotient = total / active as u64;
    let remainder = total % active as u64;
    (0..width)
        .map(|index| {
            if index >= active {
                "0".to_owned()
            } else {
                (quotient + u64::from((index as u64) < remainder)).to_string()
            }
        })
        .collect()
}

fn maker_quickstart_operation(
    maker_wallet: &str,
    request: &PlatformMakerQuickstartRequest,
    base_asset: &PlatformAsset,
    market_label: &str,
    current_slot: u64,
    mark_price: u64,
    tick_size: u64,
) -> Result<PlatformMakerQuickstartOperation, SdkError> {
    if request.spread_bps == 0 || request.spread_bps > 5_000 {
        return Err(SdkError::InvalidRequest(
            "spread_bps must be between 1 and 5,000".to_owned(),
        ));
    }
    let width = match request.product {
        PlatformMakerControlProduct::Strand => 16usize,
        PlatformMakerControlProduct::Current => 8usize,
    };
    let levels = usize::from(request.levels.unwrap_or(3));
    if levels == 0 || levels > width {
        return Err(SdkError::InvalidRequest(format!(
            "levels must be between 1 and {width}"
        )));
    }
    let level_step = request.level_step_bps.unwrap_or(request.spread_bps);
    if level_step == 0 || level_step > 5_000 {
        return Err(SdkError::InvalidRequest(
            "level_step_bps must be between 1 and 5,000".to_owned(),
        ));
    }
    let furthest =
        u32::from(request.spread_bps) + (levels.saturating_sub(1) as u32) * u32::from(level_step);
    if furthest > u32::from(u16::MAX) {
        return Err(SdkError::InvalidRequest(
            "the furthest maker level exceeds 65,535 bps".to_owned(),
        ));
    }
    let size = human_base_atoms(&request.size, base_asset, market_label)?;
    let valid_until_slot = current_slot
        .checked_add(maker_duration_slots(request.duration.as_deref())?)
        .ok_or_else(|| SdkError::InvalidRequest("duration exceeds slot range".to_owned()))?;
    let depth = split_maker_size(size, levels, width);
    let zero = vec!["0".to_owned(); width];
    let bids = if request.side == PlatformMakerQuickstartSide::Sell {
        zero.clone()
    } else {
        depth.clone()
    };
    let asks = if request.side == PlatformMakerQuickstartSide::Buy {
        zero
    } else {
        depth
    };
    match request.product {
        PlatformMakerControlProduct::Current => Ok(PlatformMakerQuickstartOperation::Current(
            PlatformMakerCurrentPrepareRequest::Upsert {
                maker_wallet: maker_wallet.to_owned(),
                enabled: true,
                async_only: request.async_only,
                half_spread_bps: request.spread_bps,
                band_step_bps: level_step,
                max_conf_bps: 100,
                max_oracle_dev_bps: 500,
                max_oracle_age_secs: 10,
                sync_spread_bps: 0,
                max_exposure_base_atoms: size.to_string(),
                bid_depth_base_atoms: bids,
                ask_depth_base_atoms: asks,
                valid_until_slot: valid_until_slot.to_string(),
            },
        )),
        PlatformMakerControlProduct::Strand => {
            if mark_price == 0 || tick_size == 0 {
                return Err(SdkError::InvalidResponse(
                    "market mark or tick size is invalid".to_owned(),
                ));
            }
            let mid_price = mark_price
                .checked_add(tick_size / 2)
                .map(|value| value / tick_size * tick_size)
                .filter(|value| *value > 0)
                .ok_or_else(|| SdkError::InvalidRequest("mark rounds below one tick".to_owned()))?;
            let mut offsets = vec![0u16; 16];
            for (index, offset) in offsets.iter_mut().take(levels).enumerate() {
                let bps = u128::from(request.spread_bps) + index as u128 * u128::from(level_step);
                let numerator = u128::from(mid_price) * bps;
                let denominator = 10_000u128 * u128::from(tick_size);
                let ticks = numerator.div_ceil(denominator);
                if ticks == 0 || ticks > u128::from(u16::MAX) {
                    return Err(SdkError::InvalidRequest(
                        "a Strand level cannot be represented on this tick grid".to_owned(),
                    ));
                }
                *offset = ticks as u16;
            }
            Ok(PlatformMakerQuickstartOperation::Strand(
                PlatformMakerStrandPrepareRequest::Upsert {
                    maker_wallet: maker_wallet.to_owned(),
                    enabled: true,
                    async_only: request.async_only,
                    sync_spread_ticks: 0,
                    mid_price_atoms: mid_price.to_string(),
                    max_exposure_base_atoms: size.to_string(),
                    bid_offsets_ticks: offsets.clone(),
                    ask_offsets_ticks: offsets,
                    bid_sizes_base_atoms: bids,
                    ask_sizes_base_atoms: asks,
                    valid_until_slot: valid_until_slot.to_string(),
                },
            ))
        }
    }
}

fn maker_product_present(
    status: &PlatformMakerStatusResponse,
    product: PlatformMakerControlProduct,
) -> bool {
    match product {
        PlatformMakerControlProduct::Strand => !status.strands.is_empty(),
        PlatformMakerControlProduct::Current => !status.currents.is_empty(),
    }
}

fn maker_product_matches(
    status: &PlatformMakerStatusResponse,
    operation: &PlatformMakerQuickstartOperation,
) -> bool {
    match operation {
        PlatformMakerQuickstartOperation::Strand(PlatformMakerStrandPrepareRequest::Upsert {
            enabled,
            async_only,
            mid_price_atoms,
            max_exposure_base_atoms,
            bid_sizes_base_atoms,
            ask_sizes_base_atoms,
            valid_until_slot,
            ..
        }) => status.strands.iter().any(|strand| {
            strand.enabled == *enabled
                && strand.async_only == *async_only
                && strand.mid_price_atoms == *mid_price_atoms
                && strand.maximum_exposure_atoms == *max_exposure_base_atoms
                && strand.valid_until_slot.as_deref() == Some(valid_until_slot)
                && same_maker_depth(
                    strand.bids.iter().map(|level| level.size_atoms.as_str()),
                    bid_sizes_base_atoms,
                )
                && same_maker_depth(
                    strand.asks.iter().map(|level| level.size_atoms.as_str()),
                    ask_sizes_base_atoms,
                )
        }),
        PlatformMakerQuickstartOperation::Current(PlatformMakerCurrentPrepareRequest::Upsert {
            enabled,
            async_only,
            half_spread_bps,
            band_step_bps,
            max_conf_bps,
            max_oracle_age_secs,
            sync_spread_bps,
            max_exposure_base_atoms,
            bid_depth_base_atoms,
            ask_depth_base_atoms,
            valid_until_slot,
            ..
        }) => status.currents.iter().any(|current| {
            current.enabled == *enabled
                && current.async_only == *async_only
                && current.half_spread_bps == *half_spread_bps
                && current.band_step_bps == *band_step_bps
                && current.maximum_confidence_bps == *max_conf_bps
                && current.maximum_oracle_age_seconds == *max_oracle_age_secs
                && current.sync_spread_bps == *sync_spread_bps
                && current.maximum_exposure_atoms == *max_exposure_base_atoms
                && current.valid_until_slot.as_deref() == Some(valid_until_slot)
                && same_maker_depth(
                    current.bid_depth_atoms.iter().map(String::as_str),
                    bid_depth_base_atoms,
                )
                && same_maker_depth(
                    current.ask_depth_atoms.iter().map(String::as_str),
                    ask_depth_base_atoms,
                )
        }),
        _ => false,
    }
}

fn same_maker_depth<'a>(actual: impl Iterator<Item = &'a str>, expected: &[String]) -> bool {
    let mut actual = actual.collect::<Vec<_>>();
    let mut expected = expected.iter().map(String::as_str).collect::<Vec<_>>();
    while actual.last() == Some(&"0") {
        actual.pop();
    }
    while expected.last() == Some(&"0") {
        expected.pop();
    }
    actual == expected
}

fn strand_prepare_action(
    request: &PlatformMakerStrandPrepareRequest,
) -> PlatformMakerControlAction {
    match request {
        PlatformMakerStrandPrepareRequest::Upsert { .. } => {
            PlatformMakerControlAction::StrandUpsert
        }
        PlatformMakerStrandPrepareRequest::Recenter { .. } => {
            PlatformMakerControlAction::StrandRecenter
        }
        PlatformMakerStrandPrepareRequest::SetEnabled { .. } => {
            PlatformMakerControlAction::StrandSetEnabled
        }
        PlatformMakerStrandPrepareRequest::Cancel { .. } => {
            PlatformMakerControlAction::StrandCancel
        }
    }
}

fn current_prepare_action(
    request: &PlatformMakerCurrentPrepareRequest,
) -> PlatformMakerControlAction {
    match request {
        PlatformMakerCurrentPrepareRequest::Upsert { .. } => {
            PlatformMakerControlAction::CurrentUpsert
        }
        PlatformMakerCurrentPrepareRequest::Cancel { .. } => {
            PlatformMakerControlAction::CurrentCancel
        }
    }
}

fn strand_prepare_wallet(request: &PlatformMakerStrandPrepareRequest) -> Result<String, SdkError> {
    let wallet = match request {
        PlatformMakerStrandPrepareRequest::Upsert { maker_wallet, .. }
        | PlatformMakerStrandPrepareRequest::Recenter { maker_wallet, .. }
        | PlatformMakerStrandPrepareRequest::SetEnabled { maker_wallet, .. }
        | PlatformMakerStrandPrepareRequest::Cancel { maker_wallet } => maker_wallet,
    };
    canonical_public_key(wallet, "maker_wallet")
}

fn current_prepare_wallet(
    request: &PlatformMakerCurrentPrepareRequest,
) -> Result<String, SdkError> {
    let wallet = match request {
        PlatformMakerCurrentPrepareRequest::Upsert { maker_wallet, .. }
        | PlatformMakerCurrentPrepareRequest::Cancel { maker_wallet } => maker_wallet,
    };
    canonical_public_key(wallet, "maker_wallet")
}

fn normalize_strand_prepare_request(
    request: PlatformMakerStrandPrepareRequest,
) -> Result<PlatformMakerStrandPrepareRequest, SdkError> {
    Ok(match request {
        PlatformMakerStrandPrepareRequest::Upsert {
            maker_wallet,
            enabled,
            async_only,
            sync_spread_ticks,
            mid_price_atoms,
            max_exposure_base_atoms,
            bid_offsets_ticks,
            ask_offsets_ticks,
            bid_sizes_base_atoms,
            ask_sizes_base_atoms,
            valid_until_slot,
        } => {
            if bid_offsets_ticks.len() != 16
                || ask_offsets_ticks.len() != 16
                || bid_sizes_base_atoms.len() != 16
                || ask_sizes_base_atoms.len() != 16
            {
                return Err(SdkError::InvalidRequest(
                    "Strand requires exactly 16 bid and 16 ask levels".to_owned(),
                ));
            }
            let bid_sizes_base_atoms =
                canonical_amounts(bid_sizes_base_atoms, "bid_sizes_base_atoms")?;
            let ask_sizes_base_atoms =
                canonical_amounts(ask_sizes_base_atoms, "ask_sizes_base_atoms")?;
            if !bid_sizes_base_atoms
                .iter()
                .chain(&ask_sizes_base_atoms)
                .any(|size| size != "0")
                || bid_offsets_ticks
                    .iter()
                    .zip(&bid_sizes_base_atoms)
                    .chain(ask_offsets_ticks.iter().zip(&ask_sizes_base_atoms))
                    .any(|(offset, size)| *offset == 0 && size != "0")
            {
                return Err(SdkError::InvalidRequest(
                    "active Strand levels require positive offsets".to_owned(),
                ));
            }
            PlatformMakerStrandPrepareRequest::Upsert {
                maker_wallet: canonical_public_key(&maker_wallet, "maker_wallet")?,
                enabled,
                async_only,
                sync_spread_ticks,
                mid_price_atoms: canonical_request_atoms(
                    &mid_price_atoms,
                    "mid_price_atoms",
                    false,
                )?,
                max_exposure_base_atoms: canonical_request_atoms(
                    &max_exposure_base_atoms,
                    "max_exposure_base_atoms",
                    false,
                )?,
                bid_offsets_ticks,
                ask_offsets_ticks,
                bid_sizes_base_atoms,
                ask_sizes_base_atoms,
                valid_until_slot: canonical_request_atoms(
                    &valid_until_slot,
                    "valid_until_slot",
                    true,
                )?,
            }
        }
        PlatformMakerStrandPrepareRequest::Recenter {
            maker_wallet,
            new_mid_price_atoms,
            valid_until_slot,
        } => PlatformMakerStrandPrepareRequest::Recenter {
            maker_wallet: canonical_public_key(&maker_wallet, "maker_wallet")?,
            new_mid_price_atoms: canonical_request_atoms(
                &new_mid_price_atoms,
                "new_mid_price_atoms",
                false,
            )?,
            valid_until_slot: canonical_request_atoms(&valid_until_slot, "valid_until_slot", true)?,
        },
        PlatformMakerStrandPrepareRequest::SetEnabled {
            maker_wallet,
            enabled,
        } => PlatformMakerStrandPrepareRequest::SetEnabled {
            maker_wallet: canonical_public_key(&maker_wallet, "maker_wallet")?,
            enabled,
        },
        PlatformMakerStrandPrepareRequest::Cancel { maker_wallet } => {
            PlatformMakerStrandPrepareRequest::Cancel {
                maker_wallet: canonical_public_key(&maker_wallet, "maker_wallet")?,
            }
        }
    })
}

fn normalize_current_prepare_request(
    request: PlatformMakerCurrentPrepareRequest,
) -> Result<PlatformMakerCurrentPrepareRequest, SdkError> {
    Ok(match request {
        PlatformMakerCurrentPrepareRequest::Upsert {
            maker_wallet,
            enabled,
            async_only,
            half_spread_bps,
            band_step_bps,
            max_conf_bps,
            max_oracle_dev_bps,
            max_oracle_age_secs,
            sync_spread_bps,
            max_exposure_base_atoms,
            bid_depth_base_atoms,
            ask_depth_base_atoms,
            valid_until_slot,
        } => {
            if bid_depth_base_atoms.len() != 8 || ask_depth_base_atoms.len() != 8 {
                return Err(SdkError::InvalidRequest(
                    "Current requires exactly 8 bid and 8 ask bands".to_owned(),
                ));
            }
            if half_spread_bps == 0
                || max_conf_bps == 0
                || max_conf_bps > 100
                || max_oracle_dev_bps == 0
                || max_oracle_dev_bps > 500
            {
                return Err(SdkError::InvalidRequest(
                    "Current mark-reference and spread bounds are invalid".to_owned(),
                ));
            }
            let bid_depth_base_atoms =
                canonical_amounts(bid_depth_base_atoms, "bid_depth_base_atoms")?;
            let ask_depth_base_atoms =
                canonical_amounts(ask_depth_base_atoms, "ask_depth_base_atoms")?;
            if !bid_depth_base_atoms
                .iter()
                .chain(&ask_depth_base_atoms)
                .any(|depth| depth != "0")
            {
                return Err(SdkError::InvalidRequest(
                    "Current requires at least one non-zero depth band".to_owned(),
                ));
            }
            PlatformMakerCurrentPrepareRequest::Upsert {
                maker_wallet: canonical_public_key(&maker_wallet, "maker_wallet")?,
                enabled,
                async_only,
                half_spread_bps,
                band_step_bps,
                max_conf_bps,
                max_oracle_dev_bps,
                max_oracle_age_secs,
                sync_spread_bps,
                max_exposure_base_atoms: canonical_request_atoms(
                    &max_exposure_base_atoms,
                    "max_exposure_base_atoms",
                    false,
                )?,
                bid_depth_base_atoms,
                ask_depth_base_atoms,
                valid_until_slot: canonical_request_atoms(
                    &valid_until_slot,
                    "valid_until_slot",
                    true,
                )?,
            }
        }
        PlatformMakerCurrentPrepareRequest::Cancel { maker_wallet } => {
            PlatformMakerCurrentPrepareRequest::Cancel {
                maker_wallet: canonical_public_key(&maker_wallet, "maker_wallet")?,
            }
        }
    })
}

fn canonical_amounts(values: Vec<String>, field: &str) -> Result<Vec<String>, SdkError> {
    values
        .into_iter()
        .map(|value| canonical_request_atoms(&value, field, true))
        .collect()
}

fn validate_maker_control_prepare(
    prepared: &PlatformMakerControlPrepareResponse,
    market_id: &str,
    maker_wallet: &str,
    product: PlatformMakerControlProduct,
    action: PlatformMakerControlAction,
) -> Result<(), SdkError> {
    validate_platform_version(prepared.schema_version, &prepared.contract_version)?;
    if prepared.market_id != market_id
        || prepared.maker_wallet != maker_wallet
        || prepared.product != product
        || prepared.action != action
        || !valid_handle(&prepared.maker_control_id, "mc_")
        || prepared.expires_at_ms == 0
    {
        return Err(SdkError::InvalidResponse(
            "prepared maker control is invalid".to_owned(),
        ));
    }
    canonical_base64(&prepared.transaction_base64, "transaction_base64")?;
    canonical_base58_32(&prepared.recent_blockhash, "recent_blockhash")?;
    Ok(())
}

fn validate_platform_market_id(value: &str) -> Result<String, SdkError> {
    let value = value.trim();
    if !valid_handle(value, "market_") {
        return Err(SdkError::InvalidRequest(
            "market_id must be an opaque Strata market ID".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_platform_asset_id(value: &str) -> Result<String, SdkError> {
    let value = value.trim();
    if !valid_handle(value, "asset_") {
        return Err(SdkError::InvalidRequest(
            "asset_id must be an opaque Strata asset ID".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_platform_authority(authority: &PlatformAuthority) -> Result<(), SdkError> {
    if authority.permission_source != PermissionSource::ExternalAgentOwner
        || authority.signing_location != SigningLocation::External
        || authority.accepts_private_keys
    {
        return Err(SdkError::InvalidResponse(
            "platform authority must remain with the external agent owner".to_owned(),
        ));
    }
    Ok(())
}

fn validate_platform_discovery(discovery: &PlatformDiscoveryResponse) -> Result<(), SdkError> {
    validate_platform_version(discovery.schema_version, &discovery.contract_version)?;
    validate_platform_authority(&discovery.authority)?;
    let mut ids = HashSet::new();
    if discovery.capabilities.iter().any(|capability| {
        capability.id.trim().is_empty()
            || capability.required_scope.trim().is_empty()
            || capability.transports.is_empty()
            || !ids.insert(capability.id.as_str())
    }) {
        return Err(SdkError::InvalidResponse(
            "platform capability discovery is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_platform_action_graph(graph: &PlatformActionGraphResponse) -> Result<(), SdkError> {
    validate_platform_version(graph.schema_version, &graph.contract_version)?;
    validate_platform_authority(&graph.authority)?;
    if graph.graph_version != "2.0" {
        return Err(SdkError::InvalidResponse(
            "unsupported platform action graph version".to_owned(),
        ));
    }

    let entities = graph
        .entities
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if entities.len() != graph.entities.len()
        || entities.contains("")
        || graph.relations.iter().any(|relation| {
            !entities.contains(relation.from.as_str())
                || !entities.contains(relation.to.as_str())
                || relation.kind.trim().is_empty()
        })
    {
        return Err(SdkError::InvalidResponse(
            "platform entity graph is invalid".to_owned(),
        ));
    }

    let mut operation_ids = HashSet::new();
    let mut operation_capabilities = HashMap::new();
    if graph.operations.iter().any(|operation| {
        operation.id.trim().is_empty()
            || operation.capability_id.trim().is_empty()
            || operation.summary.trim().is_empty()
            || operation.transports.is_empty()
            || !operation_ids.insert(operation.id.as_str())
            || operation_capabilities
                .insert(operation.id.as_str(), operation.capability_id.as_str())
                .is_some()
            || operation
                .transports
                .iter()
                .any(|transport| match transport.transport {
                    PlatformTransport::Http => {
                        transport.method.as_deref().is_none_or(str::is_empty)
                            || transport
                                .path
                                .as_deref()
                                .is_none_or(|path| !valid_platform_operation_path(path))
                            || transport.tool.is_some()
                    }
                    PlatformTransport::Websocket => {
                        transport
                            .path
                            .as_deref()
                            .is_none_or(|path| !valid_platform_operation_path(path))
                            || transport.method.is_some()
                            || transport.tool.is_some()
                    }
                    PlatformTransport::Mcp => {
                        transport.tool.as_deref().is_none_or(str::is_empty)
                            || transport.method.is_some()
                            || transport.path.is_some()
                    }
                })
    }) || !operation_ids.contains(graph.entry_operation_id.as_str())
    {
        return Err(SdkError::InvalidResponse(
            "platform operation graph is invalid".to_owned(),
        ));
    }

    let mut module_ids = HashSet::new();
    if graph.modules.iter().any(|module| {
        module.id.trim().is_empty()
            || module.client_property.trim().is_empty()
            || module.capability_ids.is_empty()
            || !module_ids.insert(module.id.as_str())
    }) {
        return Err(SdkError::InvalidResponse(
            "platform module graph is invalid".to_owned(),
        ));
    }

    let mut workflow_ids = HashSet::new();
    let mut covered_operation_ids = HashSet::new();
    if graph.workflows.iter().any(|workflow| {
        if workflow.id.trim().is_empty() || !workflow_ids.insert(workflow.id.as_str()) {
            return true;
        }
        let node_ids = workflow
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<HashSet<_>>();
        let mut outgoing = node_ids
            .iter()
            .copied()
            .map(|node_id| (node_id, Vec::new()))
            .collect::<HashMap<_, _>>();
        let nodes_are_invalid = node_ids.len() != workflow.nodes.len()
            || !node_ids.contains(workflow.entry_node.as_str())
            || workflow.nodes.iter().any(|node| {
                if node.id.trim().is_empty() {
                    return true;
                }
                match node.capability_id.as_deref() {
                    None => node.kind
                        != strata_public_contract::platform::PlatformActionKind::ExternalSignature
                        || !node.operation_ids.is_empty(),
                    Some(capability_id) => node.kind
                        == strata_public_contract::platform::PlatformActionKind::ExternalSignature
                        || node.operation_ids.is_empty()
                        || node.operation_ids.iter().any(|operation_id| {
                            let Some(operation_capability) =
                                operation_capabilities.get(operation_id.as_str())
                            else {
                                return true;
                            };
                            if *operation_capability != capability_id {
                                return true;
                            }
                            covered_operation_ids.insert(operation_id.as_str());
                            false
                        }),
                }
            });
        if nodes_are_invalid || workflow.edges.is_empty() {
            return true;
        }
        if workflow.edges.iter().any(|edge| {
            if !node_ids.contains(edge.from.as_str())
                || !node_ids.contains(edge.to.as_str())
                || edge.condition.trim().is_empty()
            {
                return true;
            }
            outgoing
                .get_mut(edge.from.as_str())
                .expect("validated workflow source node")
                .push(edge.to.as_str());
            false
        }) {
            return true;
        }
        let mut reached = HashSet::from([workflow.entry_node.as_str()]);
        let mut pending = vec![workflow.entry_node.as_str()];
        while let Some(node_id) = pending.pop() {
            for target in outgoing.get(node_id).into_iter().flatten() {
                if reached.insert(*target) {
                    pending.push(*target);
                }
            }
        }
        reached.len() != node_ids.len()
    }) {
        return Err(SdkError::InvalidResponse(
            "platform workflow graph is invalid".to_owned(),
        ));
    }
    if covered_operation_ids.len() != operation_ids.len() {
        return Err(SdkError::InvalidResponse(
            "platform action graph contains an orphaned operation".to_owned(),
        ));
    }
    Ok(())
}

fn valid_platform_operation_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains("..")
        && !path.to_ascii_lowercase().contains("/internal")
        && !path.to_ascii_lowercase().contains("/admin")
}

fn validate_platform_market_response(
    schema_version: u16,
    contract_version: &str,
    actual_market_id: &str,
    expected_market_id: &str,
) -> Result<(), SdkError> {
    validate_platform_version(schema_version, contract_version)?;
    if actual_market_id != expected_market_id {
        return Err(SdkError::InvalidResponse(
            "response market does not match request".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_page_request(request: PageRequest) -> Result<Vec<(String, String)>, SdkError> {
    let mut query = Vec::new();
    if let Some(limit) = request.limit {
        if !(1..=MAX_PLATFORM_PAGE_SIZE).contains(&limit) {
            return Err(SdkError::InvalidRequest(format!(
                "page limit must be between 1 and {MAX_PLATFORM_PAGE_SIZE}"
            )));
        }
        query.push(("limit".to_owned(), limit.to_string()));
    }
    if let Some(cursor) = request.cursor {
        let cursor = cursor.trim();
        if cursor.is_empty()
            || cursor.len() > 512
            || !cursor
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(SdkError::InvalidRequest(
                "cursor must be a non-empty opaque URL-safe value".to_owned(),
            ));
        }
        query.push(("cursor".to_owned(), cursor.to_owned()));
    }
    Ok(query)
}

fn validate_page_info(page: &PageInfo) -> Result<(), SdkError> {
    match (&page.next_cursor, page.has_more) {
        (Some(cursor), true)
            if !cursor.is_empty()
                && cursor.len() <= 512
                && cursor
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-') =>
        {
            Ok(())
        }
        (None, false) => Ok(()),
        _ => Err(SdkError::InvalidResponse(
            "pagination metadata is inconsistent".to_owned(),
        )),
    }
}

fn validate_response_atoms(value: &str, field: &str, allow_zero: bool) -> Result<u64, SdkError> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(SdkError::InvalidResponse(format!(
            "{field} must be a canonical unsigned atomic decimal string"
        )));
    }
    let parsed = value
        .parse::<u64>()
        .map_err(|_| SdkError::InvalidResponse(format!("{field} exceeds u64")))?;
    if !allow_zero && parsed == 0 {
        return Err(SdkError::InvalidResponse(format!(
            "{field} must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn validate_book_level(level: &PlatformBookLevel) -> Result<(u64, u64), SdkError> {
    Ok((
        validate_response_atoms(&level.price_atoms, "price_atoms", false)?,
        validate_response_atoms(&level.size_atoms, "size_atoms", false)?,
    ))
}

fn validate_book_levels(
    bids: &[PlatformBookLevel],
    asks: &[PlatformBookLevel],
) -> Result<(), SdkError> {
    let bid_prices = bids
        .iter()
        .map(validate_book_level)
        .collect::<Result<Vec<_>, _>>()?;
    let ask_prices = asks
        .iter()
        .map(validate_book_level)
        .collect::<Result<Vec<_>, _>>()?;
    if bid_prices
        .windows(2)
        .any(|levels| levels[0].0 <= levels[1].0)
        || ask_prices
            .windows(2)
            .any(|levels| levels[0].0 >= levels[1].0)
        || bid_prices
            .first()
            .zip(ask_prices.first())
            .is_some_and(|(bid, ask)| bid.0 >= ask.0)
    {
        return Err(SdkError::InvalidResponse(
            "book levels are not strictly ordered".to_owned(),
        ));
    }
    Ok(())
}

fn canonical_decimal(value: &str, field: &str) -> Result<(), SdkError> {
    let (mantissa, exponent) = value
        .split_once(['e', 'E'])
        .map_or((value, None), |(mantissa, exponent)| {
            (mantissa, Some(exponent))
        });
    let (whole, fraction) = mantissa
        .split_once('.')
        .map_or((mantissa, None), |(whole, fraction)| {
            (whole, Some(fraction))
        });
    let valid_whole = whole == "0"
        || (!whole.starts_with('0') && whole.bytes().all(|byte| byte.is_ascii_digit()));
    let valid_fraction = fraction.is_none_or(|fraction| {
        !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
    });
    let valid_exponent = exponent.is_none_or(|exponent| {
        let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
    });
    if !valid_whole
        || !valid_fraction
        || !valid_exponent
        || value.parse::<f64>().is_err()
        || !value.parse::<f64>().is_ok_and(f64::is_finite)
    {
        return Err(SdkError::InvalidResponse(format!(
            "{field} must be a canonical non-negative decimal string"
        )));
    }
    Ok(())
}

fn platform_history_range(range: PlatformPortfolioHistoryRange) -> &'static str {
    match range {
        PlatformPortfolioHistoryRange::Day => "24h",
        PlatformPortfolioHistoryRange::Week => "7d",
        PlatformPortfolioHistoryRange::Month => "30d",
    }
}

fn normalize_fill_limit(value: Option<u16>) -> Result<u16, SdkError> {
    match value {
        Some(limit @ 1..=200) => Ok(limit),
        Some(_) => Err(SdkError::InvalidRequest(
            "fill limit must be between 1 and 200".to_owned(),
        )),
        None => Ok(DEFAULT_ACCOUNT_FILL_LIMIT),
    }
}

fn normalize_market_ids(values: Vec<String>) -> Result<Vec<String>, SdkError> {
    let mut ids = Vec::with_capacity(values.len());
    let mut seen = HashSet::new();
    for value in values {
        let id = validate_platform_market_id(&value)?;
        if seen.insert(id.clone()) {
            ids.push(id);
        }
    }
    Ok(ids)
}

pub fn account_http_auth_message(
    market_id: &str,
    wallet_address: &str,
    timestamp_ms: u64,
    fill_limit: u16,
) -> Result<Vec<u8>, SdkError> {
    let market_id = validate_platform_market_id(market_id)?;
    let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
    let fill_limit = normalize_fill_limit(Some(fill_limit))?;
    Ok(format!(
        "strata:account-read:v2\n{market_id}\n{wallet_address}\n{timestamp_ms}\n{fill_limit}"
    )
    .into_bytes())
}

/// Optional signed-read headers (deprecated path); public reads send none.
fn maker_auth_headers(authorization: Option<(u64, &str)>) -> Result<HeaderMap, SdkError> {
    let mut headers = HeaderMap::new();
    if let Some((authorization_time_ms, authorization_signature)) = authorization {
        headers.insert(
            "x-strata-auth-time",
            HeaderValue::from_str(&authorization_time_ms.to_string()).map_err(|_| {
                SdkError::InvalidRequest("maker authorization time is invalid".to_owned())
            })?,
        );
        headers.insert(
            "x-strata-auth-signature",
            HeaderValue::from_str(authorization_signature).map_err(|_| {
                SdkError::InvalidRequest("maker authorization signature is invalid".to_owned())
            })?,
        );
    }
    Ok(headers)
}

pub fn maker_status_auth_message(
    market_id: &str,
    wallet_address: &str,
    timestamp_ms: u64,
) -> Result<Vec<u8>, SdkError> {
    let market_id = validate_platform_market_id(market_id)?;
    let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
    Ok(
        format!("strata:mm-status-read:v2\n{market_id}\n{wallet_address}\n{timestamp_ms}")
            .into_bytes(),
    )
}

fn validate_maker_status(response: &PlatformMakerStatusResponse) -> Result<(), SdkError> {
    let invalid = |detail: &str| SdkError::InvalidResponse(format!("maker status {detail}"));
    if !valid_handle(&response.maker_id, "maker_") {
        return Err(invalid("maker_id is invalid"));
    }
    let current_slot = validate_response_u128(&response.current_slot, "current_slot")?;
    let firm = &response.firm_orders;
    if u64::from(firm.bid_orders) + u64::from(firm.ask_orders) != u64::from(firm.resting_orders) {
        return Err(invalid("firm order counts are inconsistent"));
    }
    validate_response_u128(&firm.bid_size_atoms, "bid_size_atoms")?;
    validate_response_u128(&firm.ask_size_atoms, "ask_size_atoms")?;
    let mut expected_active: u32 = u32::from(firm.resting_orders > 0);
    if let Some(intent) = &response.intent {
        let minimum = validate_response_u128(&intent.minimum_price_atoms, "minimum_price_atoms")?;
        let maximum = validate_response_u128(&intent.maximum_price_atoms, "maximum_price_atoms")?;
        let maximum_fill =
            validate_response_u128(&intent.maximum_fill_size_atoms, "maximum_fill_size_atoms")?;
        let remaining = validate_response_u128(
            &intent.remaining_fill_size_atoms,
            "remaining_fill_size_atoms",
        )?;
        validate_response_u128(&intent.stake_atoms, "stake_atoms")?;
        if minimum > maximum || remaining > maximum_fill || intent.minimum_spread_bps > 10_000 {
            return Err(invalid("intent bounds are inconsistent"));
        }
        expected_active += u32::from(intent.active);
    }
    if response.signed_quotes.live_quotes.len() > 2 {
        return Err(invalid("cannot hold more than one live quote per side"));
    }
    for quote in &response.signed_quotes.live_quotes {
        validate_response_u128(&quote.price_atoms, "price_atoms")?;
        validate_response_u128(&quote.size_atoms, "size_atoms")?;
        validate_response_u128(&quote.nonce, "nonce")?;
        if quote.expires_at_ms < quote.issued_at_ms {
            return Err(invalid("signed quote expires before it was issued"));
        }
    }
    if response.strands.len() > 256 || response.currents.len() > 256 {
        return Err(invalid("maker product lists exceed the bounded size"));
    }
    for strand in &response.strands {
        validate_response_u128(&strand.mid_price_atoms, "mid_price_atoms")?;
        validate_response_u128(&strand.tick_size_atoms, "tick_size_atoms")?;
        let maximum =
            validate_response_u128(&strand.maximum_exposure_atoms, "maximum_exposure_atoms")?;
        let remaining =
            validate_response_u128(&strand.remaining_exposure_atoms, "remaining_exposure_atoms")?;
        if remaining > maximum || strand.bids.len() > 16 || strand.asks.len() > 16 {
            return Err(invalid("strand exposure or levels are inconsistent"));
        }
        for level in strand.bids.iter().chain(strand.asks.iter()) {
            if let Some(price) = &level.price_atoms {
                validate_response_u128(price, "price_atoms")?;
            }
            let size = validate_response_u128(&level.size_atoms, "size_atoms")?;
            let remaining =
                validate_response_u128(&level.remaining_size_atoms, "remaining_size_atoms")?;
            if remaining > size {
                return Err(invalid("strand level remaining exceeds size"));
            }
        }
        let expected_expired = match &strand.valid_until_slot {
            Some(slot) => current_slot > validate_response_u128(slot, "valid_until_slot")?,
            None => false,
        };
        if strand.expired != expected_expired {
            return Err(invalid("strand expiry disagrees with the current slot"));
        }
        expected_active += u32::from(strand.enabled && !strand.expired);
    }
    for current in &response.currents {
        let maximum =
            validate_response_u128(&current.maximum_exposure_atoms, "maximum_exposure_atoms")?;
        let remaining = validate_response_u128(
            &current.remaining_exposure_atoms,
            "remaining_exposure_atoms",
        )?;
        if remaining > maximum
            || current.bid_depth_atoms.len() > 8
            || current.ask_depth_atoms.len() > 8
            || current.half_spread_bps > 10_000
            || current.band_step_bps > 10_000
            || current.sync_spread_bps > 10_000
        {
            return Err(invalid("current exposure or bands are inconsistent"));
        }
        for depth in current
            .bid_depth_atoms
            .iter()
            .chain(current.ask_depth_atoms.iter())
        {
            validate_response_u128(depth, "depth_atoms")?;
        }
        let expected_expired = match &current.valid_until_slot {
            Some(slot) => current_slot > validate_response_u128(slot, "valid_until_slot")?,
            None => false,
        };
        if current.expired != expected_expired {
            return Err(invalid("current expiry disagrees with the current slot"));
        }
        expected_active += u32::from(current.enabled && !current.expired);
    }
    if response.dead_man_guards.len() > 32 {
        return Err(invalid("dead-man guard list exceeds the bounded size"));
    }
    for guard in &response.dead_man_guards {
        canonical_public_key(&guard.session_public_key, "session_public_key")?;
    }
    if u32::from(response.active_products) != expected_active {
        return Err(invalid(
            "active_products disagrees with the reported products",
        ));
    }
    Ok(())
}

pub fn maker_reputation_auth_message(
    market_id: &str,
    wallet_address: &str,
    timestamp_ms: u64,
) -> Result<Vec<u8>, SdkError> {
    let market_id = validate_platform_market_id(market_id)?;
    let wallet_address = canonical_public_key(wallet_address, "wallet_address")?;
    Ok(
        format!("strata:mm-reputation-read:v2\n{market_id}\n{wallet_address}\n{timestamp_ms}")
            .into_bytes(),
    )
}

fn validate_response_u128(value: &str, field: &str) -> Result<u128, SdkError> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(SdkError::InvalidResponse(format!(
            "{field} must be a canonical unsigned atomic decimal string"
        )));
    }
    value
        .parse::<u128>()
        .map_err(|_| SdkError::InvalidResponse(format!("{field} exceeds u128")))
}

fn validate_platform_portfolio(response: &PlatformPortfolioResponse) -> Result<(), SdkError> {
    let invalid = |detail: &str| SdkError::InvalidResponse(format!("portfolio {detail}"));
    validate_response_u128(&response.observed_slot, "observed_slot")?;
    if response.observed_at_ms > response.server_time_ms {
        return Err(invalid("cannot be observed after server time"));
    }
    if response.balances.len() > 10_000
        || response.positions.len() > 10_000
        || response.open_orders.len() > 10_000
        || response.recent_fills.len() > 10_000
        || response.unavailable_market_ids.len() > 10_000
        || response.unpriced_asset_ids.len() > 10_000
    {
        return Err(invalid("collections exceed the bounded size"));
    }
    let mut seen_orders = std::collections::BTreeSet::new();
    for order in &response.open_orders {
        validate_platform_market_id(&order.market_id)?;
        if !valid_handle(&order.order_id, "order_") || !seen_orders.insert(order.order_id.as_str())
        {
            return Err(invalid("open orders must carry unique opaque order IDs"));
        }
        let original = validate_response_u128(&order.original_size_atoms, "original_size_atoms")?;
        let remaining =
            validate_response_u128(&order.remaining_size_atoms, "remaining_size_atoms")?;
        if remaining > original || original == 0 {
            return Err(invalid("open order sizes are inconsistent"));
        }
    }
    let mut seen_fills = std::collections::BTreeSet::new();
    for fill in &response.recent_fills {
        validate_platform_market_id(&fill.market_id)?;
        if !valid_handle(&fill.fill_id, "fill_") || !seen_fills.insert(fill.fill_id.as_str()) {
            return Err(invalid("recent fills must carry unique opaque fill IDs"));
        }
        validate_response_u128(&fill.price_atoms, "price_atoms")?;
        validate_response_u128(&fill.size_atoms, "size_atoms")?;
    }
    for market_id in &response.unavailable_market_ids {
        validate_platform_market_id(market_id)?;
    }
    let mut seen_assets = std::collections::BTreeSet::new();
    let mut summed_value = 0u128;
    for balance in &response.balances {
        validate_platform_asset_id(&balance.asset_id)?;
        if !seen_assets.insert(balance.asset_id.as_str()) {
            return Err(invalid("balances must be unique per asset"));
        }
        let available = validate_response_u128(&balance.available_atoms, "available_atoms")?;
        let locked = validate_response_u128(&balance.locked_atoms, "locked_atoms")?;
        let total = validate_response_u128(&balance.total_atoms, "total_atoms")?;
        if total == 0 || available.checked_add(locked) != Some(total) {
            return Err(invalid("balance totals are inconsistent"));
        }
        let unpriced = response
            .unpriced_asset_ids
            .iter()
            .any(|asset_id| asset_id == &balance.asset_id);
        match &balance.value_usd_micros {
            Some(value) if !unpriced => {
                let value = validate_response_u128(value, "value_usd_micros")?;
                summed_value = summed_value
                    .checked_add(value)
                    .ok_or_else(|| invalid("value overflow"))?;
            }
            None if unpriced => {}
            _ => {
                return Err(invalid(
                    "balance valuation disagrees with unpriced_asset_ids",
                ))
            }
        }
    }
    let mut seen_unpriced = std::collections::BTreeSet::new();
    for asset_id in &response.unpriced_asset_ids {
        validate_platform_asset_id(asset_id)?;
        if !seen_unpriced.insert(asset_id.as_str()) || !seen_assets.contains(asset_id.as_str()) {
            return Err(invalid("unpriced assets must be unique held assets"));
        }
    }
    let mut seen_markets = std::collections::BTreeSet::new();
    for position in &response.positions {
        validate_platform_market_id(&position.market_id)?;
        validate_platform_asset_id(&position.base_asset_id)?;
        validate_platform_asset_id(&position.quote_asset_id)?;
        if position.base_asset_id == position.quote_asset_id
            || !seen_markets.insert(position.market_id.as_str())
        {
            return Err(invalid(
                "positions must be unique markets with distinct assets",
            ));
        }
        for (value, field) in [
            (&position.base_available_atoms, "base_available_atoms"),
            (&position.base_locked_atoms, "base_locked_atoms"),
            (&position.quote_available_atoms, "quote_available_atoms"),
            (&position.quote_locked_atoms, "quote_locked_atoms"),
        ] {
            validate_response_u128(value, field)?;
        }
    }
    match (
        response.valuation_complete,
        &response.equity_usd_micros,
        &response.available_usd_micros,
        &response.locked_usd_micros,
    ) {
        (true, Some(equity), Some(available), Some(locked)) => {
            if !response.unpriced_asset_ids.is_empty() {
                return Err(invalid("complete valuation cannot list unpriced assets"));
            }
            let equity = validate_response_u128(equity, "equity_usd_micros")?;
            let available = validate_response_u128(available, "available_usd_micros")?;
            let locked = validate_response_u128(locked, "locked_usd_micros")?;
            if available.checked_add(locked) != Some(equity) || summed_value != equity {
                return Err(invalid("USD totals are inconsistent"));
            }
        }
        (false, None, None, None) => {
            if response.unpriced_asset_ids.is_empty() {
                return Err(invalid("incomplete valuation must list unpriced assets"));
            }
        }
        _ => return Err(invalid("valuation flags disagree with USD totals")),
    }
    Ok(())
}

fn validate_maker_reputation(response: &PlatformMakerReputationResponse) -> Result<(), SdkError> {
    let expected_interval = if response.active {
        match response.tier {
            PlatformMakerReputationTier::Silver | PlatformMakerReputationTier::Gold => Some(100),
            PlatformMakerReputationTier::Platinum => Some(10),
            PlatformMakerReputationTier::Probation | PlatformMakerReputationTier::Bronze => None,
        }
    } else {
        None
    };
    let expected_next_tier = match response.tier {
        PlatformMakerReputationTier::Probation | PlatformMakerReputationTier::Bronze => {
            Some(PlatformMakerReputationTier::Silver)
        }
        PlatformMakerReputationTier::Silver => Some(PlatformMakerReputationTier::Gold),
        PlatformMakerReputationTier::Gold => Some(PlatformMakerReputationTier::Platinum),
        PlatformMakerReputationTier::Platinum => None,
    };
    if !valid_handle(&response.maker_id, "maker_")
        || response.reputation_score > 10_000
        || response.fill_rate_bps > 10_000
        || response.epoch_slashed_bps > 10_000
        || response.minimum_quote_interval_ms != expected_interval
        || response.signed_quote_stream_eligible != expected_interval.is_some()
        || response.tier_progress.next_tier != expected_next_tier
        || response
            .tier_progress
            .reputation_score_required
            .is_some_and(|score| score > 10_000)
    {
        return Err(SdkError::InvalidResponse(
            "maker reputation response violates its public contract".to_owned(),
        ));
    }
    let total_quote_requests =
        validate_response_atoms(&response.total_quote_requests, "total_quote_requests", true)?;
    let stake_atoms = validate_response_atoms(&response.stake_atoms, "stake_atoms", true)?;
    let tenure_slots = validate_response_atoms(&response.tenure_slots, "tenure_slots", true)?;
    for (value, field) in [
        (&response.successful_fills, "successful_fills"),
        (&response.missed_quote_requests, "missed_quote_requests"),
        (
            &response.lifetime_filled_quote_atoms,
            "lifetime_filled_quote_atoms",
        ),
        (&response.epoch_start_stake_atoms, "epoch_start_stake_atoms"),
        (&response.epoch_slashed_atoms, "epoch_slashed_atoms"),
        (
            &response.lifetime_auto_slashed_atoms,
            "lifetime_auto_slashed_atoms",
        ),
        (&response.registered_slot, "registered_slot"),
        (&response.last_active_slot, "last_active_slot"),
        (&response.last_settled_slot, "last_settled_slot"),
    ] {
        validate_response_atoms(value, field, true)?;
    }
    if let Some(value) = &response.revoked_at_slot {
        validate_response_atoms(value, "revoked_at_slot", true)?;
    }
    let progress = &response.tier_progress;
    let quote_requests_remaining = validate_response_atoms(
        &progress.quote_requests_remaining,
        "tier_progress.quote_requests_remaining",
        true,
    )?;
    let stake_atoms_remaining = validate_response_atoms(
        &progress.stake_atoms_remaining,
        "tier_progress.stake_atoms_remaining",
        true,
    )?;
    let tenure_slots_remaining = validate_response_atoms(
        &progress.tenure_slots_remaining,
        "tier_progress.tenure_slots_remaining",
        true,
    )?;
    let quote_requests_required = progress
        .quote_requests_required
        .as_deref()
        .map(|value| validate_response_atoms(value, "tier_progress.quote_requests_required", true))
        .transpose()?;
    let stake_atoms_required = progress
        .stake_atoms_required
        .as_deref()
        .map(|value| validate_response_atoms(value, "tier_progress.stake_atoms_required", true))
        .transpose()?;
    let tenure_slots_required = progress
        .tenure_slots_required
        .as_deref()
        .map(|value| validate_response_atoms(value, "tier_progress.tenure_slots_required", true))
        .transpose()?;
    let progress_shape_is_valid = match response.tier {
        PlatformMakerReputationTier::Probation => {
            progress.reputation_score_required == Some(5_000)
                && quote_requests_required == Some(50)
                && stake_atoms_required.is_none()
                && tenure_slots_required.is_none()
        }
        PlatformMakerReputationTier::Bronze => {
            progress.reputation_score_required == Some(5_000)
                && quote_requests_required.is_none()
                && stake_atoms_required.is_none()
                && tenure_slots_required.is_none()
        }
        PlatformMakerReputationTier::Silver => {
            progress.reputation_score_required == Some(7_500)
                && quote_requests_required.is_none()
                && stake_atoms_required.is_none()
                && tenure_slots_required.is_none()
        }
        PlatformMakerReputationTier::Gold => {
            progress.reputation_score_required == Some(9_000)
                && quote_requests_required.is_none()
                && stake_atoms_required.is_some()
                && tenure_slots_required == Some(6_480_000)
        }
        PlatformMakerReputationTier::Platinum => {
            progress.reputation_score_required.is_none()
                && quote_requests_required.is_none()
                && stake_atoms_required.is_none()
                && tenure_slots_required.is_none()
        }
    };
    let expected_reputation_remaining = progress
        .reputation_score_required
        .unwrap_or(response.reputation_score)
        .saturating_sub(response.reputation_score);
    if !progress_shape_is_valid
        || progress.reputation_score_remaining != expected_reputation_remaining
        || quote_requests_remaining
            != quote_requests_required
                .unwrap_or(total_quote_requests)
                .saturating_sub(total_quote_requests)
        || stake_atoms_remaining
            != stake_atoms_required
                .unwrap_or(stake_atoms)
                .saturating_sub(stake_atoms)
        || tenure_slots_remaining
            != tenure_slots_required
                .unwrap_or(tenure_slots)
                .saturating_sub(tenure_slots)
    {
        return Err(SdkError::InvalidResponse(
            "maker reputation tier progress is inconsistent".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_bug_message(value: &str) -> Result<String, SdkError> {
    let message = value.trim();
    if !(1..=2_000).contains(&message.chars().count()) {
        return Err(SdkError::InvalidRequest(
            "bug message must contain between 1 and 2,000 characters".to_owned(),
        ));
    }
    Ok(message.to_owned())
}

pub fn bug_authorization_payload(message: &str) -> Result<Vec<u8>, SdkError> {
    Ok(format!("strata-bug-report:v1:{}", normalize_bug_message(message)?).into_bytes())
}

fn normalize_referral_code(value: &str) -> Result<String, SdkError> {
    let code = value.trim();
    if code.is_empty()
        || code.len() > 64
        || !code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(SdkError::InvalidRequest(
            "referral_code must contain 1-64 letters, numbers, underscores, or dashes".to_owned(),
        ));
    }
    Ok(code.to_owned())
}

pub fn referral_link_authorization_payload(referral_code: &str) -> Result<Vec<u8>, SdkError> {
    Ok(format!(
        "strata-referral:v1:{}",
        normalize_referral_code(referral_code)?
    )
    .into_bytes())
}

pub fn referral_claim_authorization_payload(
    payout_wallet_address: &str,
) -> Result<Vec<u8>, SdkError> {
    Ok(format!(
        "strata-referral-claim:v1:{}",
        canonical_public_key(payout_wallet_address, "payout_wallet_address")?
    )
    .into_bytes())
}

fn canonical_hex_signature(value: &str, field: &str) -> Result<String, SdkError> {
    let signature = value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"))
        .unwrap_or(value.trim())
        .to_ascii_lowercase();
    if signature.len() != 128
        || !signature
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be a 64-byte hexadecimal Ed25519 signature"
        )));
    }
    Ok(signature)
}

/// Canonicalize an optional atomic value; `None` stays `None` so the server
/// resolves it (the account sequence is the one such field today).
fn canonical_optional_request_atoms(
    value: Option<&str>,
    field: &str,
) -> Result<Option<String>, SdkError> {
    value
        .map(|value| canonical_request_atoms(value, field, true))
        .transpose()
}

fn canonical_request_atoms(value: &str, field: &str, allow_zero: bool) -> Result<String, SdkError> {
    if value.is_empty()
        || !value.bytes().all(|byte| byte.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be a canonical unsigned atomic decimal string"
        )));
    }
    let parsed = value
        .parse::<u64>()
        .map_err(|_| SdkError::InvalidRequest(format!("{field} exceeds u64")))?;
    if !allow_zero && parsed == 0 {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be greater than zero"
        )));
    }
    Ok(parsed.to_string())
}

fn canonical_signature(value: &str, field: &str) -> Result<String, SdkError> {
    let value = value.trim();
    let decoded = bs58::decode(value)
        .into_vec()
        .map_err(|_| SdkError::InvalidRequest(format!("{field} must be base58")))?;
    if decoded.len() != 64 || bs58::encode(&decoded).into_string() != value {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be a canonical Ed25519 signature"
        )));
    }
    Ok(value.to_owned())
}

fn canonical_base58_32(value: &str, field: &str) -> Result<String, SdkError> {
    let value = value.trim();
    let decoded = bs58::decode(value)
        .into_vec()
        .map_err(|_| SdkError::InvalidRequest(format!("{field} must be base58")))?;
    if decoded.len() != 32 || bs58::encode(&decoded).into_string() != value {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be a canonical 32-byte base58 value"
        )));
    }
    Ok(value.to_owned())
}

fn canonical_base64(value: &str, field: &str) -> Result<String, SdkError> {
    let value = value.trim();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| SdkError::InvalidRequest(format!("{field} must be base64")))?;
    if decoded.is_empty() || base64::engine::general_purpose::STANDARD.encode(decoded) != value {
        return Err(SdkError::InvalidRequest(format!(
            "{field} must be canonical base64"
        )));
    }
    Ok(value.to_owned())
}

fn normalize_twap_challenge_request(
    request: PlatformTwapChallengeRequest,
) -> Result<PlatformTwapChallengeRequest, SdkError> {
    let request = match request {
        PlatformTwapChallengeRequest::Place {
            owner_wallet,
            session_public_key,
            side,
            total_size_atoms,
            slices_total,
            maximum_tolerance_bps,
            interval_slots,
            limit_price_atoms,
        } => {
            if !(2..=120).contains(&slices_total)
                || !(1..=1_000).contains(&maximum_tolerance_bps)
                || !(25..=4_500).contains(&interval_slots)
            {
                return Err(SdkError::InvalidRequest(
                    "TWAP schedule bounds are invalid".to_owned(),
                ));
            }
            PlatformTwapChallengeRequest::Place {
                owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
                session_public_key: canonical_public_key(
                    &session_public_key,
                    "session_public_key",
                )?,
                side,
                total_size_atoms: canonical_request_atoms(
                    &total_size_atoms,
                    "total_size_atoms",
                    false,
                )?,
                slices_total,
                maximum_tolerance_bps,
                interval_slots,
                limit_price_atoms: canonical_request_atoms(
                    &limit_price_atoms,
                    "limit_price_atoms",
                    false,
                )?,
            }
        }
        PlatformTwapChallengeRequest::Cancel {
            owner_wallet,
            session_public_key,
            twap_id,
        } => {
            if !valid_handle(twap_id.trim(), "twap_") {
                return Err(SdkError::InvalidRequest("twap_id is invalid".to_owned()));
            }
            PlatformTwapChallengeRequest::Cancel {
                owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
                session_public_key: canonical_public_key(
                    &session_public_key,
                    "session_public_key",
                )?,
                twap_id: twap_id.trim().to_owned(),
            }
        }
    };
    if twap_request_owner(&request) == twap_request_session(&request) {
        return Err(SdkError::InvalidRequest(
            "session_public_key must be distinct from owner_wallet".to_owned(),
        ));
    }
    Ok(request)
}

fn twap_request_action(request: &PlatformTwapChallengeRequest) -> PlatformTwapControlAction {
    match request {
        PlatformTwapChallengeRequest::Place { .. } => PlatformTwapControlAction::Place,
        PlatformTwapChallengeRequest::Cancel { .. } => PlatformTwapControlAction::Cancel,
    }
}

fn twap_request_owner(request: &PlatformTwapChallengeRequest) -> &str {
    match request {
        PlatformTwapChallengeRequest::Place { owner_wallet, .. }
        | PlatformTwapChallengeRequest::Cancel { owner_wallet, .. } => owner_wallet,
    }
}

fn twap_request_session(request: &PlatformTwapChallengeRequest) -> &str {
    match request {
        PlatformTwapChallengeRequest::Place {
            session_public_key, ..
        }
        | PlatformTwapChallengeRequest::Cancel {
            session_public_key, ..
        } => session_public_key,
    }
}

/// A parsed two-step TWAP authorization: the exact bytes to sign and the
/// blockhash lease they bind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwapAuthorization {
    pub bytes: Vec<u8>,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
}

fn opaque_twap_id(pda: &[u8]) -> String {
    opaque_product_id("twap", &bs58::encode(pda).into_string())
}

/// Two-step path helper: check a TWAP challenge's authorization payload
/// binds exactly this request before signing it. The one-call
/// [`StrataClient::execute_twap`] no longer needs it (one signature over the
/// transaction).
pub fn validate_twap_authorization(
    challenge: &PlatformTwapChallengeResponse,
    request: &PlatformTwapChallengeRequest,
) -> Result<TwapAuthorization, SdkError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(challenge.authorization_payload_base64.trim())
        .map_err(|_| SdkError::InvalidResponse("TWAP authorization is not base64".to_owned()))?;
    let owner = decode_public_key(twap_request_owner(request), "owner_wallet")?;
    let session = decode_public_key(twap_request_session(request), "session_public_key")?;
    let mut cursor = 0usize;
    take_expected(
        &bytes,
        &mut cursor,
        PUBLIC_TWAP_AUTH_DOMAIN,
        "TWAP authorization domain",
    )?;
    take_bytes(&bytes, &mut cursor, 64, "TWAP authorization product")?;
    take_expected(&bytes, &mut cursor, &owner, "TWAP authorization owner")?;
    take_expected(&bytes, &mut cursor, &session, "TWAP authorization session")?;
    let action = take_bytes(&bytes, &mut cursor, 1, "TWAP authorization action")?[0];
    let expected_action = twap_request_action(request);
    if action
        != match expected_action {
            PlatformTwapControlAction::Place => 0,
            PlatformTwapControlAction::Cancel => 1,
        }
        || challenge.action != expected_action
    {
        return Err(SdkError::InvalidResponse(
            "TWAP authorization action changed".to_owned(),
        ));
    }
    let pda = match request {
        PlatformTwapChallengeRequest::Place {
            side,
            total_size_atoms,
            slices_total,
            maximum_tolerance_bps,
            interval_slots,
            limit_price_atoms,
            ..
        } => {
            let encoded_side = take_bytes(&bytes, &mut cursor, 1, "TWAP side")?[0];
            let expected_side = match side {
                PlatformTradeSide::Buy => 0,
                PlatformTradeSide::Sell => 1,
            };
            if encoded_side != expected_side {
                return Err(SdkError::InvalidResponse("TWAP side changed".to_owned()));
            }
            take_u64_eq(
                &bytes,
                &mut cursor,
                parse_request_u64(total_size_atoms, "total_size_atoms")?,
                "TWAP total size",
            )?;
            if take_u16(&bytes, &mut cursor, "TWAP slices")? != *slices_total
                || take_u16(&bytes, &mut cursor, "TWAP tolerance")? != *maximum_tolerance_bps
            {
                return Err(SdkError::InvalidResponse(
                    "TWAP schedule bounds changed".to_owned(),
                ));
            }
            let interval_bytes: [u8; 4] = take_bytes(&bytes, &mut cursor, 4, "TWAP interval")?
                .try_into()
                .map_err(|_| SdkError::InvalidResponse("TWAP interval is invalid".to_owned()))?;
            if u32::from_le_bytes(interval_bytes) != *interval_slots {
                return Err(SdkError::InvalidResponse(
                    "TWAP interval changed".to_owned(),
                ));
            }
            take_u64_eq(
                &bytes,
                &mut cursor,
                parse_request_u64(limit_price_atoms, "limit_price_atoms")?,
                "TWAP limit price",
            )?;
            take_bytes(&bytes, &mut cursor, 8, "TWAP schedule nonce")?;
            take_bytes(&bytes, &mut cursor, 32, "TWAP identity")?.to_vec()
        }
        PlatformTwapChallengeRequest::Cancel { twap_id, .. } => {
            let pda = take_bytes(&bytes, &mut cursor, 32, "TWAP identity")?.to_vec();
            if opaque_twap_id(&pda) != *twap_id {
                return Err(SdkError::InvalidResponse(
                    "TWAP cancellation identity changed".to_owned(),
                ));
            }
            pda
        }
    };
    if opaque_twap_id(&pda) != challenge.twap_id {
        return Err(SdkError::InvalidResponse(
            "TWAP authorization identity changed".to_owned(),
        ));
    }
    let blockhash = take_bytes(&bytes, &mut cursor, 32, "TWAP recent blockhash")?;
    let recent_blockhash = bs58::encode(blockhash).into_string();
    let last_valid_block_height = take_u64(&bytes, &mut cursor, "TWAP block height")?;
    take_u64_eq(
        &bytes,
        &mut cursor,
        challenge.expires_at_ms,
        "TWAP authorization expiry",
    )?;
    let nonce = take_bytes(&bytes, &mut cursor, 16, "TWAP authorization nonce")?;
    if hex::encode(nonce) != challenge.challenge_id[4..] {
        return Err(SdkError::InvalidResponse(
            "TWAP challenge nonce changed".to_owned(),
        ));
    }
    if cursor != bytes.len() {
        return Err(SdkError::InvalidResponse(
            "TWAP authorization contains unrecognized fields".to_owned(),
        ));
    }
    Ok(TwapAuthorization {
        bytes,
        recent_blockhash,
        last_valid_block_height,
    })
}

/// Two-step path helper: check a prepared TWAP control preserved the signed
/// challenge bindings.
pub fn validate_twap_prepare_binding(
    prepared: &PlatformTwapPrepareResponse,
    challenge: &PlatformTwapChallengeResponse,
    authorization: &TwapAuthorization,
) -> Result<(), SdkError> {
    if prepared.market_id != challenge.market_id
        || prepared.action != challenge.action
        || prepared.twap_id != challenge.twap_id
        || prepared.recent_blockhash != authorization.recent_blockhash
        || prepared.last_valid_block_height != authorization.last_valid_block_height
        || prepared.expires_at_ms != challenge.expires_at_ms
    {
        return Err(SdkError::InvalidResponse(
            "prepared TWAP control changed the signed bindings".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_order_challenge_request(
    request: PlatformOrderChallengeRequest,
) -> Result<PlatformOrderChallengeRequest, SdkError> {
    let normalized = match request {
        PlatformOrderChallengeRequest::Place {
            owner_wallet,
            session_public_key,
            account_sequence,
            client_order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
        } => {
            let client_order_id = client_order_id.trim().to_owned();
            if client_order_id.is_empty()
                || client_order_id.len() > 64
                || !client_order_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
                || !matches!(
                    order_type,
                    PlatformOrderType::GoodUntilCancelled | PlatformOrderType::PostOnly
                )
            {
                return Err(SdkError::InvalidRequest(
                    "resting order client ID or type is invalid".to_owned(),
                ));
            }
            PlatformOrderChallengeRequest::Place {
                owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
                session_public_key: canonical_public_key(
                    &session_public_key,
                    "session_public_key",
                )?,
                account_sequence: canonical_optional_request_atoms(
                    account_sequence.as_deref(),
                    "account_sequence",
                )?,
                client_order_id,
                side,
                order_type,
                limit_price_atoms: canonical_request_atoms(
                    &limit_price_atoms,
                    "limit_price_atoms",
                    false,
                )?,
                size_atoms: canonical_request_atoms(&size_atoms, "size_atoms", false)?,
            }
        }
        PlatformOrderChallengeRequest::Cancel {
            owner_wallet,
            session_public_key,
            order_id,
        } => {
            if !valid_handle(order_id.trim(), "order_") {
                return Err(SdkError::InvalidRequest("order_id is invalid".to_owned()));
            }
            PlatformOrderChallengeRequest::Cancel {
                owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
                session_public_key: canonical_public_key(
                    &session_public_key,
                    "session_public_key",
                )?,
                order_id: order_id.trim().to_owned(),
            }
        }
        PlatformOrderChallengeRequest::CancelAll {
            owner_wallet,
            session_public_key,
        } => PlatformOrderChallengeRequest::CancelAll {
            owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
            session_public_key: canonical_public_key(&session_public_key, "session_public_key")?,
        },
        PlatformOrderChallengeRequest::Replace {
            owner_wallet,
            session_public_key,
            order_id,
            account_sequence,
            client_order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
        } => {
            let PlatformOrderBatchOperation::Replace {
                order_id,
                account_sequence,
                client_order_id,
                side,
                order_type,
                limit_price_atoms,
                size_atoms,
            } = normalize_order_batch_operation(PlatformOrderBatchOperation::Replace {
                order_id,
                account_sequence,
                client_order_id,
                side,
                order_type,
                limit_price_atoms,
                size_atoms,
            })?
            else {
                unreachable!()
            };
            PlatformOrderChallengeRequest::Replace {
                owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
                session_public_key: canonical_public_key(
                    &session_public_key,
                    "session_public_key",
                )?,
                order_id,
                account_sequence,
                client_order_id,
                side,
                order_type,
                limit_price_atoms,
                size_atoms,
            }
        }
        PlatformOrderChallengeRequest::Batch {
            owner_wallet,
            session_public_key,
            operations,
        } => {
            if operations.is_empty() || operations.len() > 6 {
                return Err(SdkError::InvalidRequest(
                    "order batch must contain between one and six operations".to_owned(),
                ));
            }
            PlatformOrderChallengeRequest::Batch {
                owner_wallet: canonical_public_key(&owner_wallet, "owner_wallet")?,
                session_public_key: canonical_public_key(
                    &session_public_key,
                    "session_public_key",
                )?,
                operations: operations
                    .into_iter()
                    .map(normalize_order_batch_operation)
                    .collect::<Result<_, _>>()?,
            }
        }
    };
    if order_request_owner(&normalized) == order_request_session(&normalized) {
        return Err(SdkError::InvalidRequest(
            "session_public_key must be distinct from owner_wallet".to_owned(),
        ));
    }
    Ok(normalized)
}

fn normalize_order_batch_operation(
    operation: PlatformOrderBatchOperation,
) -> Result<PlatformOrderBatchOperation, SdkError> {
    match operation {
        PlatformOrderBatchOperation::Place {
            account_sequence,
            client_order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
        } => {
            let client_order_id = normalize_order_client_id(client_order_id, order_type)?;
            Ok(PlatformOrderBatchOperation::Place {
                account_sequence: canonical_optional_request_atoms(
                    account_sequence.as_deref(),
                    "account_sequence",
                )?,
                client_order_id,
                side,
                order_type,
                limit_price_atoms: canonical_request_atoms(
                    &limit_price_atoms,
                    "limit_price_atoms",
                    false,
                )?,
                size_atoms: canonical_request_atoms(&size_atoms, "size_atoms", false)?,
            })
        }
        PlatformOrderBatchOperation::Cancel { order_id } => {
            if !valid_handle(order_id.trim(), "order_") {
                return Err(SdkError::InvalidRequest("order_id is invalid".to_owned()));
            }
            Ok(PlatformOrderBatchOperation::Cancel {
                order_id: order_id.trim().to_owned(),
            })
        }
        PlatformOrderBatchOperation::Replace {
            order_id,
            account_sequence,
            client_order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
        } => {
            if !valid_handle(order_id.trim(), "order_") {
                return Err(SdkError::InvalidRequest("order_id is invalid".to_owned()));
            }
            let client_order_id = normalize_order_client_id(client_order_id, order_type)?;
            Ok(PlatformOrderBatchOperation::Replace {
                order_id: order_id.trim().to_owned(),
                account_sequence: canonical_optional_request_atoms(
                    account_sequence.as_deref(),
                    "account_sequence",
                )?,
                client_order_id,
                side,
                order_type,
                limit_price_atoms: canonical_request_atoms(
                    &limit_price_atoms,
                    "limit_price_atoms",
                    false,
                )?,
                size_atoms: canonical_request_atoms(&size_atoms, "size_atoms", false)?,
            })
        }
    }
}

fn normalize_order_client_id(
    client_order_id: String,
    order_type: PlatformOrderType,
) -> Result<String, SdkError> {
    let client_order_id = client_order_id.trim().to_owned();
    if client_order_id.is_empty()
        || client_order_id.len() > 64
        || !client_order_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || !matches!(
            order_type,
            PlatformOrderType::GoodUntilCancelled | PlatformOrderType::PostOnly
        )
    {
        return Err(SdkError::InvalidRequest(
            "resting order client ID or type is invalid".to_owned(),
        ));
    }
    Ok(client_order_id)
}

fn order_request_action(request: &PlatformOrderChallengeRequest) -> PlatformOrderAction {
    match request {
        PlatformOrderChallengeRequest::Place { .. } => PlatformOrderAction::Place,
        PlatformOrderChallengeRequest::Cancel { .. } => PlatformOrderAction::Cancel,
        PlatformOrderChallengeRequest::CancelAll { .. } => PlatformOrderAction::CancelAll,
        PlatformOrderChallengeRequest::Replace { .. } => PlatformOrderAction::Replace,
        PlatformOrderChallengeRequest::Batch { .. } => PlatformOrderAction::Batch,
    }
}

fn order_request_owner(request: &PlatformOrderChallengeRequest) -> &str {
    match request {
        PlatformOrderChallengeRequest::Place { owner_wallet, .. }
        | PlatformOrderChallengeRequest::Cancel { owner_wallet, .. }
        | PlatformOrderChallengeRequest::CancelAll { owner_wallet, .. }
        | PlatformOrderChallengeRequest::Replace { owner_wallet, .. }
        | PlatformOrderChallengeRequest::Batch { owner_wallet, .. } => owner_wallet,
    }
}

fn order_request_session(request: &PlatformOrderChallengeRequest) -> &str {
    match request {
        PlatformOrderChallengeRequest::Place {
            session_public_key, ..
        }
        | PlatformOrderChallengeRequest::Cancel {
            session_public_key, ..
        }
        | PlatformOrderChallengeRequest::CancelAll {
            session_public_key, ..
        }
        | PlatformOrderChallengeRequest::Replace {
            session_public_key, ..
        }
        | PlatformOrderChallengeRequest::Batch {
            session_public_key, ..
        } => session_public_key,
    }
}

/// A parsed two-step order authorization: the exact bytes to sign and the
/// blockhash lease they bind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderAuthorization {
    pub bytes: Vec<u8>,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
}

/// A supplied account sequence must match the signed authorization exactly; a
/// sequence left to Strata is read from it (the server resolved it from the
/// Vault's confirmed market account) and every other binding is still checked.
fn take_order_account_sequence(
    bytes: &[u8],
    cursor: &mut usize,
    account_sequence: Option<&str>,
) -> Result<u64, SdkError> {
    match account_sequence {
        Some(expected) => {
            let expected = parse_request_u64(expected, "account_sequence")?;
            take_u64_eq(bytes, cursor, expected, "order account sequence")?;
            Ok(expected)
        }
        None => take_u64(bytes, cursor, "order account sequence"),
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_order_place_authorization(
    bytes: &[u8],
    cursor: &mut usize,
    challenge: &PlatformOrderChallengeResponse,
    account_sequence: Option<&str>,
    client_order_id: &str,
    side: PlatformTradeSide,
    order_type: PlatformOrderType,
    limit_price_atoms: &str,
    size_atoms: &str,
) -> Result<String, SdkError> {
    take_order_account_sequence(bytes, cursor, account_sequence)?;
    let client_length = take_u16(bytes, cursor, "client order ID length")? as usize;
    if client_length != client_order_id.len() {
        return Err(SdkError::InvalidResponse(
            "client order ID length changed".to_owned(),
        ));
    }
    take_expected(bytes, cursor, client_order_id.as_bytes(), "client order ID")?;
    let actual_side = take_bytes(bytes, cursor, 1, "order side")?[0];
    let expected_side = if side == PlatformTradeSide::Buy { 0 } else { 1 };
    if actual_side != expected_side {
        return Err(SdkError::InvalidResponse("order side changed".to_owned()));
    }
    let actual_type = take_bytes(bytes, cursor, 1, "order type")?[0];
    let expected_type = match order_type {
        PlatformOrderType::GoodUntilCancelled => 0,
        PlatformOrderType::PostOnly => 3,
        PlatformOrderType::ImmediateOrCancel | PlatformOrderType::FillOrKill => {
            return Err(SdkError::InvalidRequest(
                "order type is not a resting order".to_owned(),
            ));
        }
    };
    if actual_type != expected_type {
        return Err(SdkError::InvalidResponse("order type changed".to_owned()));
    }
    take_u64_eq(
        bytes,
        cursor,
        parse_request_u64(limit_price_atoms, "limit_price_atoms")?,
        "order limit price",
    )?;
    take_u64_eq(
        bytes,
        cursor,
        parse_request_u64(size_atoms, "size_atoms")?,
        "order size",
    )?;
    let order = take_bytes(bytes, cursor, 32, "order identity")?;
    Ok(opaque_order_id(&challenge.market_id, order))
}

fn validate_order_cancel_authorization(
    bytes: &[u8],
    cursor: &mut usize,
    challenge: &PlatformOrderChallengeResponse,
    expected_order_id: &str,
) -> Result<String, SdkError> {
    let order = take_bytes(bytes, cursor, 32, "cancel order identity")?;
    let rent_source = take_bytes(bytes, cursor, 1, "cancel rent source")?[0];
    if rent_source > 1 {
        return Err(SdkError::InvalidResponse(
            "cancel rent source is invalid".to_owned(),
        ));
    }
    let order_id = opaque_order_id(&challenge.market_id, order);
    if order_id != expected_order_id {
        return Err(SdkError::InvalidResponse(
            "cancel order identity changed".to_owned(),
        ));
    }
    Ok(order_id)
}

/// Two-step path helper: check an order challenge's authorization payload
/// binds exactly this operation (every field, opaque order identity, and
/// replay value) before signing it. The one-call
/// [`StrataClient::execute_order`] no longer needs it (one signature over the
/// transaction); the order command channel still uses it to bind the
/// challenge without a message signature.
pub fn validate_order_authorization(
    challenge: &PlatformOrderChallengeResponse,
    request: &PlatformOrderChallengeRequest,
) -> Result<OrderAuthorization, SdkError> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(challenge.authorization_payload_base64.trim())
        .map_err(|_| SdkError::InvalidResponse("order authorization is not base64".to_owned()))?;
    let owner = decode_public_key(order_request_owner(request), "owner_wallet")?;
    let session = decode_public_key(order_request_session(request), "session_public_key")?;
    let mut cursor = 0usize;
    take_expected(
        &bytes,
        &mut cursor,
        PUBLIC_ORDER_AUTH_DOMAIN,
        "order authorization domain",
    )?;
    let _market = take_bytes(&bytes, &mut cursor, 32, "order authorization market")?;
    take_expected(&bytes, &mut cursor, &owner, "order authorization owner")?;
    take_expected(&bytes, &mut cursor, &session, "order authorization session")?;
    let action = take_bytes(&bytes, &mut cursor, 1, "order authorization action")?[0];
    let expected_action = match order_request_action(request) {
        PlatformOrderAction::Place => 0,
        PlatformOrderAction::Cancel => 1,
        PlatformOrderAction::CancelAll => 2,
        PlatformOrderAction::Replace => 3,
        PlatformOrderAction::Batch => 4,
    };
    if action != expected_action || challenge.action != order_request_action(request) {
        return Err(SdkError::InvalidResponse(
            "order authorization action changed".to_owned(),
        ));
    }
    let mut derived_order_ids = Vec::new();
    match request {
        PlatformOrderChallengeRequest::Place {
            account_sequence,
            client_order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
            ..
        } => {
            take_order_account_sequence(&bytes, &mut cursor, account_sequence.as_deref())?;
            let client_length = take_u16(&bytes, &mut cursor, "client order ID length")? as usize;
            if client_length != client_order_id.len() {
                return Err(SdkError::InvalidResponse(
                    "client order ID length changed".to_owned(),
                ));
            }
            take_expected(
                &bytes,
                &mut cursor,
                client_order_id.as_bytes(),
                "client order ID",
            )?;
            let actual_side = take_bytes(&bytes, &mut cursor, 1, "order side")?[0];
            let expected_side = if *side == PlatformTradeSide::Buy {
                0
            } else {
                1
            };
            if actual_side != expected_side {
                return Err(SdkError::InvalidResponse("order side changed".to_owned()));
            }
            let actual_type = take_bytes(&bytes, &mut cursor, 1, "order type")?[0];
            let expected_type = match order_type {
                PlatformOrderType::GoodUntilCancelled => 0,
                PlatformOrderType::PostOnly => 3,
                PlatformOrderType::ImmediateOrCancel | PlatformOrderType::FillOrKill => {
                    return Err(SdkError::InvalidRequest(
                        "order type is not a resting order".to_owned(),
                    ));
                }
            };
            if actual_type != expected_type {
                return Err(SdkError::InvalidResponse("order type changed".to_owned()));
            }
            take_u64_eq(
                &bytes,
                &mut cursor,
                parse_request_u64(limit_price_atoms, "limit_price_atoms")?,
                "order limit price",
            )?;
            take_u64_eq(
                &bytes,
                &mut cursor,
                parse_request_u64(size_atoms, "size_atoms")?,
                "order size",
            )?;
            let order = take_bytes(&bytes, &mut cursor, 32, "order identity")?;
            derived_order_ids.push(opaque_order_id(&challenge.market_id, order));
        }
        PlatformOrderChallengeRequest::Cancel { .. }
        | PlatformOrderChallengeRequest::CancelAll { .. } => {
            let count = usize::from(take_bytes(&bytes, &mut cursor, 1, "cancel order count")?[0]);
            if count == 0
                || count > 6
                || (matches!(request, PlatformOrderChallengeRequest::Cancel { .. }) && count != 1)
            {
                return Err(SdkError::InvalidResponse(
                    "cancel order count changed".to_owned(),
                ));
            }
            for index in 0..count {
                let order = take_bytes(&bytes, &mut cursor, 32, &format!("cancel order {index}"))?;
                let rent_source = take_bytes(
                    &bytes,
                    &mut cursor,
                    1,
                    &format!("cancel rent source {index}"),
                )?[0];
                if rent_source > 1 {
                    return Err(SdkError::InvalidResponse(
                        "cancel rent source is invalid".to_owned(),
                    ));
                }
                derived_order_ids.push(opaque_order_id(&challenge.market_id, order));
            }
            if let PlatformOrderChallengeRequest::Cancel { order_id, .. } = request {
                if derived_order_ids.first() != Some(order_id) {
                    return Err(SdkError::InvalidResponse(
                        "cancel order identity changed".to_owned(),
                    ));
                }
            }
        }
        PlatformOrderChallengeRequest::Replace {
            order_id,
            account_sequence,
            client_order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
            ..
        } => {
            derived_order_ids.push(validate_order_cancel_authorization(
                &bytes,
                &mut cursor,
                challenge,
                order_id,
            )?);
            derived_order_ids.push(validate_order_place_authorization(
                &bytes,
                &mut cursor,
                challenge,
                account_sequence.as_deref(),
                client_order_id,
                *side,
                *order_type,
                limit_price_atoms,
                size_atoms,
            )?);
        }
        PlatformOrderChallengeRequest::Batch { operations, .. } => {
            let count = usize::from(take_bytes(&bytes, &mut cursor, 1, "batch count")?[0]);
            if count == 0 || count > 6 || count != operations.len() {
                return Err(SdkError::InvalidResponse(
                    "order batch count changed".to_owned(),
                ));
            }
            for operation in operations {
                let tag = take_bytes(&bytes, &mut cursor, 1, "batch action")?[0];
                match operation {
                    PlatformOrderBatchOperation::Place {
                        account_sequence,
                        client_order_id,
                        side,
                        order_type,
                        limit_price_atoms,
                        size_atoms,
                    } if tag == 0 => derived_order_ids.push(validate_order_place_authorization(
                        &bytes,
                        &mut cursor,
                        challenge,
                        account_sequence.as_deref(),
                        client_order_id,
                        *side,
                        *order_type,
                        limit_price_atoms,
                        size_atoms,
                    )?),
                    PlatformOrderBatchOperation::Cancel { order_id } if tag == 1 => {
                        derived_order_ids.push(validate_order_cancel_authorization(
                            &bytes,
                            &mut cursor,
                            challenge,
                            order_id,
                        )?)
                    }
                    PlatformOrderBatchOperation::Replace {
                        order_id,
                        account_sequence,
                        client_order_id,
                        side,
                        order_type,
                        limit_price_atoms,
                        size_atoms,
                    } if tag == 3 => {
                        derived_order_ids.push(validate_order_cancel_authorization(
                            &bytes,
                            &mut cursor,
                            challenge,
                            order_id,
                        )?);
                        derived_order_ids.push(validate_order_place_authorization(
                            &bytes,
                            &mut cursor,
                            challenge,
                            account_sequence.as_deref(),
                            client_order_id,
                            *side,
                            *order_type,
                            limit_price_atoms,
                            size_atoms,
                        )?);
                    }
                    _ => {
                        return Err(SdkError::InvalidResponse(
                            "order batch action changed".to_owned(),
                        ))
                    }
                }
            }
        }
    }
    if derived_order_ids != challenge.order_ids {
        return Err(SdkError::InvalidResponse(
            "order authorization opaque identities changed".to_owned(),
        ));
    }
    let recent_blockhash = bs58::encode(take_bytes(
        &bytes,
        &mut cursor,
        32,
        "order authorization blockhash",
    )?)
    .into_string();
    let last_valid_block_height = take_u64(
        &bytes,
        &mut cursor,
        "order authorization last valid block height",
    )?;
    take_u64_eq(
        &bytes,
        &mut cursor,
        challenge.expires_at_ms,
        "order authorization expiry",
    )?;
    let nonce = take_bytes(&bytes, &mut cursor, 16, "order authorization nonce")?;
    if hex::encode(nonce) != challenge.challenge_id[3..] {
        return Err(SdkError::InvalidResponse(
            "order challenge nonce changed".to_owned(),
        ));
    }
    let _epoch = take_bytes(&bytes, &mut cursor, 16, "order authorization epoch")?;
    if cursor != bytes.len() {
        return Err(SdkError::InvalidResponse(
            "order authorization contains unrecognized fields".to_owned(),
        ));
    }
    Ok(OrderAuthorization {
        bytes,
        recent_blockhash,
        last_valid_block_height,
    })
}

/// Direct-path binding: the prepared control must be for this market and
/// action, and its echoed order IDs must follow the request — every
/// requested cancel ID in request order (replace: old then new; batch
/// flattened in request order) with one fresh ID per place. `cancel_all`
/// only requires at least one order.
fn validate_order_direct_binding(
    prepared: &PlatformOrderPrepareResponse,
    request: &PlatformOrderChallengeRequest,
    market_id: &str,
) -> Result<(), SdkError> {
    let bound = prepared.market_id == market_id
        && prepared.action == order_request_action(request)
        && prepared
            .order_ids
            .iter()
            .all(|order_id| valid_handle(order_id, "order_"))
        && match request {
            PlatformOrderChallengeRequest::Place { .. } => prepared.order_ids.len() == 1,
            PlatformOrderChallengeRequest::Cancel { order_id, .. } => {
                prepared.order_ids.len() == 1 && prepared.order_ids[0] == *order_id
            }
            PlatformOrderChallengeRequest::CancelAll { .. } => !prepared.order_ids.is_empty(),
            PlatformOrderChallengeRequest::Replace { order_id, .. } => {
                prepared.order_ids.len() == 2 && prepared.order_ids[0] == *order_id
            }
            PlatformOrderChallengeRequest::Batch { operations, .. } => {
                let mut expected: Vec<Option<&str>> = Vec::new();
                for operation in operations {
                    match operation {
                        PlatformOrderBatchOperation::Place { .. } => expected.push(None),
                        PlatformOrderBatchOperation::Cancel { order_id } => {
                            expected.push(Some(order_id))
                        }
                        PlatformOrderBatchOperation::Replace { order_id, .. } => {
                            expected.push(Some(order_id));
                            expected.push(None);
                        }
                    }
                }
                expected.len() == prepared.order_ids.len()
                    && expected
                        .iter()
                        .zip(&prepared.order_ids)
                        .all(|(expected, actual)| expected.is_none_or(|id| id == actual))
            }
        };
    if !bound {
        return Err(SdkError::InvalidResponse(
            "prepared order control does not match the request".to_owned(),
        ));
    }
    Ok(())
}

/// Direct-path binding: the prepared TWAP control must be for this market and
/// action and, for a cancellation, the requested TWAP.
fn validate_twap_direct_binding(
    prepared: &PlatformTwapPrepareResponse,
    request: &PlatformTwapChallengeRequest,
    market_id: &str,
) -> Result<(), SdkError> {
    let bound = prepared.market_id == market_id
        && prepared.action == twap_request_action(request)
        && match request {
            PlatformTwapChallengeRequest::Place { .. } => valid_handle(&prepared.twap_id, "twap_"),
            PlatformTwapChallengeRequest::Cancel { twap_id, .. } => prepared.twap_id == *twap_id,
        };
    if !bound {
        return Err(SdkError::InvalidResponse(
            "prepared TWAP control does not match the request".to_owned(),
        ));
    }
    Ok(())
}

/// The order-control prepare authorization, checked: a valid challenge handle
/// and, when present, a canonical detached signature. `None` is sent as-is;
/// only the session-authenticated order command channel accepts it.
fn normalize_order_prepare_authorization(
    authorization: PlatformOrderPrepareAuthorization,
) -> Result<PlatformOrderPrepareAuthorization, SdkError> {
    if !valid_handle(&authorization.challenge_id, "oc_") {
        return Err(SdkError::InvalidRequest(
            "order challenge_id is invalid".to_owned(),
        ));
    }
    Ok(PlatformOrderPrepareAuthorization {
        challenge_id: authorization.challenge_id,
        authorization_signature: authorization
            .authorization_signature
            .as_deref()
            .map(|signature| canonical_signature(signature, "authorization_signature"))
            .transpose()?,
    })
}

/// Two-step path helper: check a prepared order control preserved the signed
/// challenge bindings.
pub fn validate_order_prepare_binding(
    prepared: &PlatformOrderPrepareResponse,
    challenge: &PlatformOrderChallengeResponse,
    authorization: &OrderAuthorization,
) -> Result<(), SdkError> {
    if prepared.market_id != challenge.market_id
        || prepared.action != challenge.action
        || prepared.order_ids != challenge.order_ids
        || prepared.recent_blockhash != authorization.recent_blockhash
        || prepared.last_valid_block_height != authorization.last_valid_block_height
        || prepared.expires_at_ms != challenge.expires_at_ms
    {
        return Err(SdkError::InvalidResponse(
            "prepared order control changed the signed bindings".to_owned(),
        ));
    }
    Ok(())
}

fn parse_request_u64(value: &str, field: &str) -> Result<u64, SdkError> {
    value
        .parse::<u64>()
        .map_err(|_| SdkError::InvalidRequest(format!("{field} exceeds u64")))
}

fn take_u16(source: &[u8], cursor: &mut usize, field: &str) -> Result<u16, SdkError> {
    let bytes: [u8; 2] = take_bytes(source, cursor, 2, field)?
        .try_into()
        .map_err(|_| SdkError::InvalidResponse(format!("{field} is invalid")))?;
    Ok(u16::from_le_bytes(bytes))
}

/// Opaque product identity: `{kind}_` + hex of the first 16 bytes of
/// `sha256("strata-sdk-product:v1\0{kind}\0{value}")`.
pub(crate) fn opaque_product_id(kind: &str, value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"strata-sdk-product:v1\0");
    digest.update(kind.as_bytes());
    digest.update([0]);
    digest.update(value.as_bytes());
    format!("{kind}_{}", hex::encode(&digest.finalize()[..16]))
}

/// The opaque market ID for a base58 market account key.
pub(crate) fn opaque_market_id(market_key: &str) -> String {
    opaque_product_id("market", market_key)
}

pub(crate) fn opaque_order_id(market_id: &str, order: &[u8]) -> String {
    opaque_product_id(
        "order",
        &format!("{market_id}:{}", bs58::encode(order).into_string()),
    )
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

/// Which amount a quote request fixes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuoteTarget {
    /// Spend exactly this input.
    ExactInput(u64),
    /// Receive at least this output; Strata resolves the input.
    ExactOutput(u64),
}

impl QuoteTarget {
    pub fn amount(self) -> u64 {
        match self {
            Self::ExactInput(amount) | Self::ExactOutput(amount) => amount,
        }
    }
}

/// The output floor an exact-output quote must carry: the requested amount
/// lowered by `maximum_tolerance_bps` (truncating), zero tolerance meaning the
/// requested amount itself.
pub fn exact_output_floor(amount_out: u64, maximum_tolerance_bps: u16) -> u64 {
    u64::try_from(
        u128::from(amount_out) * u128::from(10_000u16.saturating_sub(maximum_tolerance_bps))
            / 10_000,
    )
    .unwrap_or(0)
}

/// Exactly one of `amount_in_atoms` / `amount_out_atoms`, canonical and > 0.
pub fn quote_target(request: &QuoteRequest) -> Result<QuoteTarget, SdkError> {
    match (
        request.amount_in_atoms.as_deref(),
        request.amount_out_atoms.as_deref(),
    ) {
        (Some(amount_in), None) => {
            let amount = parse_atoms("amount_in_atoms", amount_in)?;
            if amount == 0 {
                return Err(SdkError::InvalidRequest(
                    "amount_in_atoms must be greater than zero".to_owned(),
                ));
            }
            Ok(QuoteTarget::ExactInput(amount))
        }
        (None, Some(amount_out)) => {
            let amount = parse_atoms("amount_out_atoms", amount_out)?;
            if amount == 0 {
                return Err(SdkError::InvalidRequest(
                    "amount_out_atoms must be greater than zero".to_owned(),
                ));
            }
            Ok(QuoteTarget::ExactOutput(amount))
        }
        _ => Err(SdkError::InvalidRequest(
            "provide exactly one of amount_in_atoms or amount_out_atoms".to_owned(),
        )),
    }
}

fn validate_quote(
    quote: &QuoteResponse,
    market_id: &str,
    request: &QuoteRequest,
    target: QuoteTarget,
) -> Result<(), SdkError> {
    validate_version(quote.schema_version, &quote.contract_version)?;
    let bound_to_request = match target {
        QuoteTarget::ExactInput(amount_in) => quote.amount_in_atoms == amount_in.to_string(),
        // The floor is the requested output with the caller's tolerance
        // applied the same way an exact-input quote applies it.
        QuoteTarget::ExactOutput(amount_out) => {
            quote.minimum_output_atoms
                == exact_output_floor(amount_out, request.maximum_tolerance_bps).to_string()
        }
    };
    if quote.provider != "Sonar"
        || quote.market_id != market_id
        || quote.side != request.side
        || quote.maximum_tolerance_bps != request.maximum_tolerance_bps
        || !bound_to_request
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

    let amount_in = parse_atoms("amount_in_atoms", &quote.amount_in_atoms)?;
    let consumed = parse_atoms("amount_in_consumed_atoms", &quote.amount_in_consumed_atoms)?;
    let output = parse_atoms("amount_out_atoms", &quote.amount_out_atoms)?;
    let minimum = parse_atoms("minimum_output_atoms", &quote.minimum_output_atoms)?;
    parse_atoms("input_fee_atoms", &quote.input_fee_atoms)?;
    parse_atoms("output_fee_atoms", &quote.output_fee_atoms)?;
    if consumed > amount_in || minimum > output {
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

/// A parsed two-step execution authorization: the exact bytes to sign and
/// the blockhash lease they bind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionAuthorization {
    pub bytes: Vec<u8>,
    pub recent_blockhash: String,
    pub last_valid_block_height: u64,
}

/// Two-step path helper: check an execution challenge is bound to this quote
/// and still inside its lifetime.
pub fn validate_execution_challenge(
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

/// Two-step path helper: check a prepared execution preserved the signed
/// challenge bindings.
pub fn validate_execution_prepare(
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

fn normalize_execution_challenge_request(
    request: ExecutionChallengeRequest,
) -> Result<ExecutionChallengeRequest, SdkError> {
    if !valid_handle(&request.quote_id, "sq_") {
        return Err(SdkError::InvalidRequest("quote_id is invalid".to_owned()));
    }
    Ok(ExecutionChallengeRequest {
        quote_id: request.quote_id,
        owner_wallet: canonical_public_key(&request.owner_wallet, "owner_wallet")?,
        session_public_key: canonical_public_key(
            &request.session_public_key,
            "session_public_key",
        )?,
        account_sequence: canonical_optional_request_atoms(
            request.account_sequence.as_deref(),
            "account_sequence",
        )?,
    })
}

/// Direct-path binding: a prepared execution must be bound to exactly this
/// quote and carry a well-formed transaction envelope.
fn validate_execution_direct_prepare(
    prepared: &ExecutionPrepareResponse,
    quote: &QuoteResponse,
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
    if !valid_handle(&prepared.execution_id, "se_") || prepared.expires_at_ms == 0 {
        return Err(SdkError::InvalidResponse(
            "prepared execution does not match the requested quote".to_owned(),
        ));
    }
    canonical_base64(&prepared.transaction_base64, "transaction_base64")?;
    canonical_base58_32(&prepared.recent_blockhash, "recent_blockhash")?;
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

/// Two-step path helper: check a challenge's authorization payload binds
/// exactly this quote, owner, and session before signing it. The one-call
/// [`StrataClient::execute_quote`] no longer needs it (one signature over the
/// transaction).
pub fn validate_execution_authorization(
    challenge: &ExecutionChallengeResponse,
    quote: &QuoteResponse,
    owner_wallet: &str,
    session_public_key: &str,
    account_sequence: Option<u64>,
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
    match account_sequence {
        Some(expected) => take_u64_eq(
            &bytes,
            &mut cursor,
            expected,
            "authorization account sequence",
        )?,
        // Left to Strata: the resolved sequence is whatever the signed
        // authorization carries; every other binding is still checked.
        None => {
            take_u64(&bytes, &mut cursor, "authorization account sequence")?;
        }
    }
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
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_platform_discovery() -> PlatformDiscoveryResponse {
        let mut discovery: PlatformDiscoveryResponse =
            serde_json::from_str(strata_public_contract::platform::PLATFORM_CAPABILITIES_FIXTURE)
                .unwrap();
        let http = vec![PlatformTransport::Http];
        let websocket = vec![PlatformTransport::Websocket];
        let http_and_websocket = vec![PlatformTransport::Http, PlatformTransport::Websocket];
        let capability = |id: &str, risk: CapabilityRisk, transports: Vec<PlatformTransport>| {
            LivePlatformCapability {
                id: id.to_owned(),
                risk,
                required_scope: "test".to_owned(),
                transports,
                mcp_exposure: McpExposure::None,
            }
        };
        discovery.capabilities = vec![
            capability("platform.discover", CapabilityRisk::Read, http.clone()),
            capability("platform.status.read", CapabilityRisk::Read, http.clone()),
            capability("assets.read", CapabilityRisk::Read, http.clone()),
            capability("markets.read", CapabilityRisk::Read, http.clone()),
            capability("books.read", CapabilityRisk::Read, http_and_websocket),
            capability("markets.status.read", CapabilityRisk::Read, http.clone()),
            capability("fees.read", CapabilityRisk::Read, http.clone()),
            capability(
                "market_data.book.snapshot",
                CapabilityRisk::Read,
                http.clone(),
            ),
            capability(
                "market_data.book.stream",
                CapabilityRisk::Read,
                websocket.clone(),
            ),
            capability(
                "market_data.bbo.stream",
                CapabilityRisk::Read,
                websocket.clone(),
            ),
            capability(
                "market_data.trades.read",
                CapabilityRisk::Read,
                http.clone(),
            ),
            capability(
                "market_data.trades.stream",
                CapabilityRisk::Read,
                websocket.clone(),
            ),
            capability(
                "market_data.candles.read",
                CapabilityRisk::Read,
                http.clone(),
            ),
            capability(
                "market_data.marks.read",
                CapabilityRisk::Read,
                vec![PlatformTransport::Http, PlatformTransport::Websocket],
            ),
            capability("quotes.swap.read", CapabilityRisk::Read, http.clone()),
            capability("execution.status.read", CapabilityRisk::Read, http.clone()),
            capability("execution.stream", CapabilityRisk::Read, websocket.clone()),
            capability(
                "orders.prepare",
                CapabilityRisk::Prepare,
                vec![PlatformTransport::Http, PlatformTransport::Websocket],
            ),
            capability(
                "orders.submit",
                CapabilityRisk::Submit,
                vec![PlatformTransport::Http, PlatformTransport::Websocket],
            ),
            capability("algos.twap.place", CapabilityRisk::Submit, http.clone()),
            capability(
                "algos.twap.cancel",
                CapabilityRisk::Destructive,
                http.clone(),
            ),
            capability("algos.twap.read", CapabilityRisk::Read, http.clone()),
            capability("algos.twap.stream", CapabilityRisk::Read, websocket.clone()),
            capability("account.read", CapabilityRisk::Read, http.clone()),
            capability("account.stream", CapabilityRisk::Read, websocket.clone()),
            capability("portfolio.read", CapabilityRisk::Read, http.clone()),
            capability("portfolio.history.read", CapabilityRisk::Read, http.clone()),
            capability("vault.status.read", CapabilityRisk::Read, http.clone()),
            capability("vault.setup", CapabilityRisk::Submit, http.clone()),
            capability("vault.deposit", CapabilityRisk::Submit, http.clone()),
            capability("vault.withdraw", CapabilityRisk::Destructive, http.clone()),
            capability(
                "vault.delegate.manage",
                CapabilityRisk::Destructive,
                http.clone(),
            ),
            capability(
                "vault.policy.manage",
                CapabilityRisk::Destructive,
                http.clone(),
            ),
            capability("vault.pause", CapabilityRisk::Destructive, http.clone()),
            capability("vault.relay", CapabilityRisk::Submit, http.clone()),
            capability("mm.status.read", CapabilityRisk::Read, http.clone()),
            capability("mm.reputation.read", CapabilityRisk::Read, http.clone()),
            capability("mm.fills.stream", CapabilityRisk::Read, websocket),
            capability("mm.strand.manage", CapabilityRisk::Submit, http.clone()),
            capability("mm.current.manage", CapabilityRisk::Submit, http.clone()),
            capability("rewards.read", CapabilityRisk::Read, http.clone()),
            capability("referrals.read", CapabilityRisk::Read, http.clone()),
            capability("referrals.link", CapabilityRisk::Submit, http.clone()),
            capability("referrals.claim", CapabilityRisk::Submit, http.clone()),
            capability("bugs.read", CapabilityRisk::Read, http.clone()),
            capability("bugs.submit", CapabilityRisk::Submit, http),
        ];
        discovery
    }

    fn seed_platform_capabilities(client: &StrataClient) {
        client
            .store_platform_capabilities(test_platform_discovery())
            .unwrap();
    }

    fn fixture(path: &str) -> serde_json::Value {
        if path == "platform-capabilities" {
            return serde_json::to_value(test_platform_discovery()).unwrap();
        }
        let raw = match path {
            "action-graph" => strata_public_contract::contract_fixtures::ACTION_GRAPH,
            "markets" => strata_public_contract::contract_fixtures::MARKETS,
            "quote" => strata_public_contract::contract_fixtures::QUOTE,
            "capabilities" => strata_public_contract::contract_fixtures::CAPABILITIES,
            "execution-prepare" => strata_public_contract::contract_fixtures::EXECUTION_PREPARE,
            "execution-submit" => strata_public_contract::contract_fixtures::EXECUTION_SUBMIT,
            "order-challenge" => strata_public_contract::platform::PLATFORM_ORDER_CHALLENGE_FIXTURE,
            "order-prepare" => strata_public_contract::platform::PLATFORM_ORDER_PREPARE_FIXTURE,
            "order-submit" => strata_public_contract::platform::PLATFORM_ORDER_SUBMIT_FIXTURE,
            "order-status" => strata_public_contract::platform::PLATFORM_ORDER_STATUS_FIXTURE,
            "twap-challenge" => strata_public_contract::platform::PLATFORM_TWAP_CHALLENGE_FIXTURE,
            "twap-prepare" => strata_public_contract::platform::PLATFORM_TWAP_PREPARE_FIXTURE,
            "twap-submit" => strata_public_contract::platform::PLATFORM_TWAP_SUBMIT_FIXTURE,
            "platform-action-graph" => strata_public_contract::platform::PLATFORM_ACTION_GRAPH,
            "platform-status" => strata_public_contract::platform::PLATFORM_SERVICE_STATUS_FIXTURE,
            "assets" => strata_public_contract::platform::PLATFORM_ASSETS_FIXTURE,
            "swap-quote" => strata_public_contract::platform::PLATFORM_SWAP_QUOTE_FIXTURE,
            "platform-markets" => strata_public_contract::platform::PLATFORM_MARKETS_FIXTURE,
            "book" => strata_public_contract::platform::PLATFORM_BOOK_FIXTURE,
            "bbo" => strata_public_contract::platform::PLATFORM_BBO_FIXTURE,
            "fees" => strata_public_contract::platform::PLATFORM_FEES_FIXTURE,
            "market-status" => strata_public_contract::platform::PLATFORM_STATUS_FIXTURE,
            "trades" => strata_public_contract::platform::PLATFORM_TRADES_FIXTURE,
            "candles" => strata_public_contract::platform::PLATFORM_CANDLES_FIXTURE,
            "mark" => strata_public_contract::platform::PLATFORM_MARK_FIXTURE,
            "execution-status" => {
                strata_public_contract::platform::PLATFORM_EXECUTION_STATUS_FIXTURE
            }
            "twaps" => strata_public_contract::platform::PLATFORM_TWAPS_FIXTURE,
            "portfolio" => strata_public_contract::platform::PLATFORM_PORTFOLIO_FIXTURE,
            "maker-status" => strata_public_contract::platform::PLATFORM_MAKER_STATUS_FIXTURE,
            "maker-stream" => strata_public_contract::platform::PLATFORM_MAKER_STREAM_FIXTURE,
            "twap-stream" => strata_public_contract::platform::PLATFORM_TWAP_STREAM_FIXTURE,
            "execution-stream" => {
                strata_public_contract::platform::PLATFORM_EXECUTION_STREAM_FIXTURE
            }
            "portfolio-history" => {
                strata_public_contract::platform::PLATFORM_PORTFOLIO_HISTORY_FIXTURE
            }
            "vault-status" => strata_public_contract::platform::PLATFORM_VAULT_STATUS_FIXTURE,
            "vault-pause-prepare" => {
                strata_public_contract::platform::PLATFORM_VAULT_PAUSE_PREPARE_FIXTURE
            }
            "vault-setup-prepare" => {
                strata_public_contract::platform::PLATFORM_VAULT_SETUP_PREPARE_FIXTURE
            }
            "vault-delegate-prepare" => {
                strata_public_contract::platform::PLATFORM_VAULT_DELEGATE_PREPARE_FIXTURE
            }
            "vault-policy-prepare" => {
                strata_public_contract::platform::PLATFORM_VAULT_POLICY_PREPARE_FIXTURE
            }
            "vault-deposit-prepare" => {
                strata_public_contract::platform::PLATFORM_VAULT_DEPOSIT_PREPARE_FIXTURE
            }
            "vault-withdraw-prepare" => {
                strata_public_contract::platform::PLATFORM_VAULT_WITHDRAW_PREPARE_FIXTURE
            }
            "vault-submit" => strata_public_contract::platform::PLATFORM_VAULT_SUBMIT_FIXTURE,
            "rewards" => strata_public_contract::platform::PLATFORM_REWARDS_FIXTURE,
            "referrals" => strata_public_contract::platform::PLATFORM_REFERRALS_FIXTURE,
            "referral-link" => strata_public_contract::platform::PLATFORM_REFERRAL_LINK_FIXTURE,
            "referral-claim" => strata_public_contract::platform::PLATFORM_REFERRAL_CLAIM_FIXTURE,
            "bugs" => strata_public_contract::platform::PLATFORM_BUGS_FIXTURE,
            "bug-submit" => strata_public_contract::platform::PLATFORM_BUG_SUBMIT_FIXTURE,
            "account" => strata_public_contract::platform::PLATFORM_ACCOUNT_FIXTURE,
            _ => unreachable!(),
        };
        serde_json::from_str(raw).unwrap()
    }

    async fn mount_get(server: &MockServer, operation_path: &str, fixture_name: &str) {
        Mock::given(method("GET"))
            .and(path(operation_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture(fixture_name)))
            .expect(1)
            .mount(server)
            .await;
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
        Mock::given(method("GET"))
            .and(path("/sonar/action-graph"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("action-graph")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/sonar/markets/sol-usdc/quote"))
            .and(body_json(serde_json::json!({
                "market_id": "11111111111111111111111111111111",
                "side": "sell",
                "amount_in_atoms": "10000000",
                "maximum_tolerance_bps": 50
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

        let graph = client.action_graph().await.unwrap();
        assert_eq!(graph.entry_node, "discover_capabilities");
        assert_eq!(graph.authority.permission_source, "external_agent_owner");

        let quote = client
            .quote(QuoteRequest {
                market_id: "SOL/USDC".to_owned(),
                side: QuoteSide::Sell,
                amount_in_atoms: Some("10000000".to_owned()),
                amount_out_atoms: None,
                maximum_tolerance_bps: 50,
            })
            .await
            .unwrap();
        let public = serde_json::to_value(quote).unwrap();
        assert!(public.get("quote_id").is_some());
        assert!(public.get("unexpected_field").is_none());

        // The request must fix exactly one amount.
        for (amount_in, amount_out) in [(None, None), (Some("1"), Some("1")), (Some("0"), None)] {
            let request = QuoteRequest {
                market_id: "SOL/USDC".to_owned(),
                side: QuoteSide::Sell,
                amount_in_atoms: amount_in.map(str::to_owned),
                amount_out_atoms: amount_out.map(str::to_owned),
                maximum_tolerance_bps: 50,
            };
            assert!(matches!(
                quote_target(&request),
                Err(SdkError::InvalidRequest(_))
            ));
        }
    }

    #[test]
    fn exact_output_quotes_bind_to_the_requested_minimum_output() {
        let raw: QuoteResponse = serde_json::from_str(include_str!(
            "../../strata-public-contract/fixtures/v1/quote.json"
        ))
        .unwrap();
        let market_id = raw.market_id.clone();
        let mut quote = raw.clone();
        // Zero tolerance: the floor is the requested amount itself and the
        // best route delivers it (within a basis point) at quote time. The
        // response echoes the tolerance next to the measured impact.
        quote.minimum_output_atoms = "1000000000".to_owned();
        quote.amount_out_atoms = "1000000004".to_owned();
        quote.maximum_tolerance_bps = 0;
        let request = QuoteRequest {
            market_id: market_id.clone(),
            side: quote.side,
            amount_in_atoms: None,
            amount_out_atoms: Some("1000000000".to_owned()),
            maximum_tolerance_bps: 0,
        };
        let target = quote_target(&request).unwrap();
        assert_eq!(target, QuoteTarget::ExactOutput(1_000_000_000));
        // Serialization leaves the unused amount out so older servers reject
        // rather than misread the request.
        let wire = serde_json::to_string(&request).unwrap();
        assert!(wire.contains("amount_out_atoms") && !wire.contains("amount_in_atoms"));
        validate_quote(&quote, &market_id, &request, target).unwrap();
        // A response whose floor is not the requested output is refused.
        quote.minimum_output_atoms = "999999999".to_owned();
        assert!(validate_quote(&quote, &market_id, &request, target).is_err());
        // With a tolerance the floor is the requested amount lowered by it,
        // exactly as an exact-input quote lowers its own floor.
        assert_eq!(exact_output_floor(1_000_000_000, 25), 997_500_000);
        let tolerant = QuoteRequest {
            maximum_tolerance_bps: 25,
            ..request.clone()
        };
        quote.minimum_output_atoms = "997500000".to_owned();
        quote.maximum_tolerance_bps = 25;
        validate_quote(
            &quote,
            &market_id,
            &tolerant,
            quote_target(&tolerant).unwrap(),
        )
        .unwrap();
        // A quote that echoes a different tolerance than requested is foreign.
        quote.maximum_tolerance_bps = 10;
        assert!(validate_quote(
            &quote,
            &market_id,
            &tolerant,
            quote_target(&tolerant).unwrap()
        )
        .is_err());
        // An exact-input request still binds on the input amount (the fixture
        // carries a 50 bps tolerance).
        let exact_input = QuoteRequest {
            market_id: market_id.clone(),
            side: raw.side,
            amount_in_atoms: Some(raw.amount_in_atoms.clone()),
            amount_out_atoms: None,
            maximum_tolerance_bps: 50,
        };
        let input_target = quote_target(&exact_input).unwrap();
        validate_quote(&raw, &market_id, &exact_input, input_target).unwrap();
    }

    #[test]
    fn platform_graph_rejects_orphaned_operations() {
        let mut graph = PlatformActionGraphResponse::foundation();
        let mut orphan = graph.operations[0].clone();
        orphan.id = "platform.unmapped.read".to_owned();
        orphan.summary =
            "This test operation is deliberately absent from every workflow.".to_owned();
        graph.operations.push(orphan);

        assert!(matches!(
            validate_platform_action_graph(&graph),
            Err(SdkError::InvalidResponse(message))
                if message.contains("orphaned operation")
        ));
    }

    #[tokio::test]
    async fn platform_reads_map_the_complete_live_product_surface() {
        let server = MockServer::start().await;
        let market_id = "market_33333333333333333333333333333333";
        let wallet = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
        mount_get(&server, "/v2/capabilities", "platform-capabilities").await;
        mount_get(&server, "/v2/action-graph", "platform-action-graph").await;
        mount_get(&server, "/v2/status", "platform-status").await;
        mount_get(&server, "/v2/assets", "assets").await;
        mount_get(&server, "/v2/markets", "platform-markets").await;
        Mock::given(method("POST"))
            .and(path("/v2/quotes"))
            .and(body_json(serde_json::json!({
                "input_asset_id": "asset_11111111111111111111111111111111",
                "output_asset_id": "asset_22222222222222222222222222222222",
                "amount_in_atoms": "10000000",
                "maximum_tolerance_bps": 50
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("swap-quote")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v2/markets/{market_id}/book")))
            .and(query_param("depth", "50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("book")))
            .expect(1)
            .mount(&server)
            .await;
        mount_get(&server, &format!("/v2/markets/{market_id}/bbo"), "bbo").await;
        mount_get(&server, &format!("/v2/markets/{market_id}/fees"), "fees").await;
        mount_get(
            &server,
            &format!("/v2/markets/{market_id}/status"),
            "market-status",
        )
        .await;
        Mock::given(method("GET"))
            .and(path(format!("/v2/markets/{market_id}/trades")))
            .and(query_param("limit", "25"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("trades")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v2/markets/{market_id}/candles")))
            .and(query_param("from_ms", "1786549800000"))
            .and(query_param("to_ms", "1786550400001"))
            .and(query_param("resolution_seconds", "300"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("candles")))
            .expect(1)
            .mount(&server)
            .await;
        mount_get(&server, &format!("/v2/markets/{market_id}/marks"), "mark").await;
        mount_get(
            &server,
            &format!("/v2/markets/{market_id}/executions/se_0123456789abcdef0123456789abcdef"),
            "execution-status",
        )
        .await;
        mount_get(
            &server,
            &format!("/v2/markets/{market_id}/account/{wallet}/twaps"),
            "twaps",
        )
        .await;

        let client = StrataClient::new(server.uri()).unwrap();
        assert!(!client
            .platform_capabilities()
            .await
            .unwrap()
            .capabilities
            .is_empty());
        assert_eq!(
            client
                .platform_action_graph()
                .await
                .unwrap()
                .entry_operation_id,
            "platform.capabilities.read"
        );
        assert_eq!(
            client.platform_status().await.unwrap().available_operations,
            59
        );
        assert!(!client
            .platform_assets(PageRequest::default())
            .await
            .unwrap()
            .assets
            .is_empty());
        assert!(!client
            .platform_markets(PageRequest::default())
            .await
            .unwrap()
            .markets
            .is_empty());
        assert_eq!(
            client
                .platform_swap_quote(PlatformSwapQuoteRequest {
                    input_asset_id: "asset_11111111111111111111111111111111".to_owned(),
                    output_asset_id: "asset_22222222222222222222222222222222".to_owned(),
                    amount_in_atoms: "10000000".to_owned(),
                    maximum_tolerance_bps: 50,
                })
                .await
                .unwrap()
                .amount_out_atoms,
            "1990000"
        );
        assert_eq!(
            client
                .platform_book(market_id, PlatformBookRequest { depth: Some(50) },)
                .await
                .unwrap()
                .bids
                .len(),
            2
        );
        assert!(client
            .platform_best_bid_ask(market_id)
            .await
            .unwrap()
            .best_bid
            .is_some());
        assert!(
            client
                .platform_fees(market_id)
                .await
                .unwrap()
                .exact_fee_returned_by_quote
        );
        assert_eq!(
            client
                .platform_market_status(market_id)
                .await
                .unwrap()
                .market_id,
            market_id
        );
        assert!(!client
            .platform_trades(market_id, PlatformTradesRequest { limit: Some(25) },)
            .await
            .unwrap()
            .trades
            .is_empty());
        assert_eq!(
            client
                .platform_candles(
                    market_id,
                    PlatformCandlesRequest {
                        from_ms: 1_786_549_800_000,
                        to_ms: 1_786_550_400_001,
                        resolution_seconds: Some(300),
                    },
                )
                .await
                .unwrap()
                .resolution_seconds,
            300
        );
        assert!(!client.platform_mark(market_id).await.unwrap().stale);
        assert_eq!(
            client
                .platform_execution_status(market_id, "se_0123456789abcdef0123456789abcdef",)
                .await
                .unwrap()
                .status,
            PlatformExecutionState::Confirmed
        );
        assert!(!client
            .platform_twaps(market_id, wallet)
            .await
            .unwrap()
            .twaps
            .is_empty());
    }

    #[tokio::test]
    async fn platform_capability_preflight_fails_closed_and_caches_discovery() {
        let server = MockServer::start().await;
        let mut discovery = test_platform_discovery();
        discovery
            .capabilities
            .retain(|capability| capability.id != "mm.current.manage");
        Mock::given(method("GET"))
            .and(path("/v2/capabilities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(discovery))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        for _ in 0..2 {
            let error = client
                .platform_maker_current_prepare(
                    "market_33333333333333333333333333333333",
                    PlatformMakerCurrentPrepareRequest::Cancel {
                        maker_wallet: "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL".to_owned(),
                    },
                )
                .await
                .unwrap_err();
            assert!(matches!(
                error,
                SdkError::OperationUnavailable(message)
                    if message.contains("mm.current.manage")
            ));
        }

        seed_platform_capabilities(&client);
        client
            .require_platform_capability(
                "algos.twap.cancel",
                CapabilityRisk::Destructive,
                PlatformTransport::Http,
            )
            .await
            .unwrap();
        assert!(client
            .require_platform_capability(
                "algos.twap.cancel",
                CapabilityRisk::Submit,
                PlatformTransport::Http,
            )
            .await
            .is_err());
    }

    #[tokio::test]
    async fn maker_controls_use_exact_product_paths_and_external_transaction_bytes() {
        let server = MockServer::start().await;
        let market_id = "market_33333333333333333333333333333333";
        let wallet = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
        let control_id = "mc_0123456789abcdef0123456789abcdef";
        Mock::given(method("POST"))
            .and(path(format!(
                "/v2/markets/{market_id}/makers/strands/prepare"
            )))
            .and(query_param("transaction_version", "0"))
            .and(body_json(serde_json::json!({
                "action": "cancel",
                "maker_wallet": wallet,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "schema_version": 2,
                "contract_version": "2.0",
                "maker_control_id": control_id,
                "market_id": market_id,
                "maker_wallet": wallet,
                "product": "strand",
                "action": "strand_cancel",
                "transaction_base64": "AQ==",
                "recent_blockhash": "11111111111111111111111111111111",
                "last_valid_block_height": 123,
                "expires_at_ms": 1786550460000u64,
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v2/markets/{market_id}/makers/currents/prepare"
            )))
            .and(query_param("transaction_version", "0"))
            .and(body_json(serde_json::json!({
                "action": "cancel",
                "maker_wallet": wallet,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "schema_version": 2,
                "contract_version": "2.0",
                "maker_control_id": control_id,
                "market_id": market_id,
                "maker_wallet": wallet,
                "product": "current",
                "action": "current_cancel",
                "transaction_base64": "AQ==",
                "recent_blockhash": "11111111111111111111111111111111",
                "last_valid_block_height": 123,
                "expires_at_ms": 1786550460000u64,
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!(
                "/v2/markets/{market_id}/makers/strands/submit"
            )))
            .and(body_json(serde_json::json!({
                "maker_control_id": control_id,
                "signed_transaction_base64": "AQ==",
                "idempotency_key": "strand-cancel-1",
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "schema_version": 2,
                "contract_version": "2.0",
                "maker_control_id": control_id,
                "market_id": market_id,
                "maker_wallet": wallet,
                "product": "strand",
                "action": "strand_cancel",
                "signature": "1".repeat(64),
                "status": "submitted",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        seed_platform_capabilities(&client);
        let strand = client
            .platform_maker_strand_prepare(
                market_id,
                PlatformMakerStrandPrepareRequest::Cancel {
                    maker_wallet: wallet.to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(strand.action, PlatformMakerControlAction::StrandCancel);
        let current = client
            .platform_maker_current_prepare(
                market_id,
                PlatformMakerCurrentPrepareRequest::Cancel {
                    maker_wallet: wallet.to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(current.action, PlatformMakerControlAction::CurrentCancel);
        let submitted = client
            .platform_maker_strand_submit(
                market_id,
                PlatformMakerControlSubmitRequest {
                    maker_control_id: control_id.to_owned(),
                    signed_transaction_base64: "AQ==".to_owned(),
                    idempotency_key: "strand-cancel-1".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            submitted.status,
            PlatformMakerControlSubmissionStatus::Submitted
        );
    }

    struct TestAccountSigner {
        wallet: String,
        expected_message: Vec<u8>,
        signature_byte: u8,
    }

    #[async_trait]
    impl AccountSigner for TestAccountSigner {
        fn public_key(&self) -> &str {
            &self.wallet
        }

        async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, String> {
            assert_eq!(message, self.expected_message);
            Ok(vec![self.signature_byte; 64])
        }
    }

    #[tokio::test]
    async fn platform_account_and_community_reads_preserve_external_authority() {
        let server = MockServer::start().await;
        let market_id = "market_33333333333333333333333333333333";
        let wallet = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
        mount_get(&server, "/v2/capabilities", "platform-capabilities").await;
        Mock::given(method("GET"))
            .and(path(format!("/v2/markets/{market_id}/account/{wallet}")))
            .and(query_param("fill_limit", "25"))
            .and(header("x-strata-auth-time", "1786550400000"))
            .and(header("x-strata-auth-signature", "07".repeat(64)))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("account")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v2/account/{wallet}/portfolio")))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("portfolio")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!(
                "/v2/markets/market_33333333333333333333333333333333/makers/{wallet}"
            )))
            .and(header("x-strata-auth-time", "1786550400000"))
            .and(header("x-strata-auth-signature", "0a".repeat(64)))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("maker-status")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/v2/account/{wallet}/portfolio/history")))
            .and(query_param("range", "24h"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("portfolio-history")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v2/vault/status"))
            .and(query_param("wallet_address", wallet))
            .and(query_param(
                "session_public_key",
                "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("vault-status")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/pause/prepare"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "paused": true,
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("vault-pause-prepare")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/setup/prepare"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "session_public_key": "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
                "market_id": "market_33333333333333333333333333333333",
                "expires_at_ms": null,
                "minimum_interval_seconds": 1,
                "maximum_tolerance_bps": 100,
                "spending_limits": [
                    {
                        "asset_id": "asset_0123456789abcdef0123456789abcdef",
                        "maximum_per_execution_atoms": null,
                    },
                    {
                        "asset_id": "asset_fedcba9876543210fedcba9876543210",
                        "maximum_per_execution_atoms": "100000000",
                    },
                ],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("vault-setup-prepare")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/deposits/prepare"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "market_id": "market_33333333333333333333333333333333",
                "asset_id": "asset_0123456789abcdef0123456789abcdef",
                "amount_atoms": "10000000",
                "session_public_key": "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
            })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(fixture("vault-deposit-prepare")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/withdrawals/prepare"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "market_id": "market_33333333333333333333333333333333",
                "asset_id": "asset_fedcba9876543210fedcba9876543210",
                "destination_wallet_address": wallet,
                "amount_atoms": "5000000",
            })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(fixture("vault-withdraw-prepare")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/submit"))
            .and(body_json(serde_json::json!({
                "preparation_id": "vp_4d5e6f708192a3b4c5d6e7f8091a2b3c",
                "signed_transaction_base64": "AQIDBA==",
                "idempotency_key": "deposit-1",
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("vault-submit")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/v2/vault/submissions/vp_4d5e6f708192a3b4c5d6e7f8091a2b3c",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("vault-submit")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/delegates/prepare"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "session_public_key": "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2",
                "action": "revoke",
            })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(fixture("vault-delegate-prepare")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/vault/policies/prepare"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "withdrawal_access": {
                    "mode": "restricted",
                    "allowed_wallet_addresses": [wallet],
                },
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("vault-policy-prepare")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v2/rewards"))
            .and(query_param("wallet_address", wallet))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("rewards")))
            .expect(1)
            .mount(&server)
            .await;
        mount_get(&server, &format!("/v2/referrals/{wallet}"), "referrals").await;
        Mock::given(method("POST"))
            .and(path("/v2/referrals/link"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "referral_code": "STRATA1",
                "authorization_signature": "22".repeat(64),
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("referral-link")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/referrals/claim"))
            .and(body_json(serde_json::json!({
                "wallet_address": wallet,
                "payout_wallet_address": wallet,
                "authorization_signature": "33".repeat(64),
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("referral-claim")))
            .expect(1)
            .mount(&server)
            .await;
        mount_get(&server, &format!("/v2/bugs/{wallet}"), "bugs").await;
        Mock::given(method("POST"))
            .and(path("/v2/bugs"))
            .and(body_json(serde_json::json!({
                "owner_wallet": wallet,
                "message": "public report",
                "authorization_signature": "07".repeat(64),
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("bug-submit")))
            .expect(1)
            .mount(&server)
            .await;

        let signer = TestAccountSigner {
            wallet: wallet.to_owned(),
            expected_message: format!(
                "strata:account-read:v2\n{market_id}\n{wallet}\n1786550400000\n25"
            )
            .into_bytes(),
            signature_byte: 7,
        };
        let client = StrataClient::new(server.uri()).unwrap();
        let account = client
            .platform_account_market(
                market_id,
                &signer,
                PlatformAccountMarketRequest {
                    fill_limit: Some(25),
                },
            )
            .await
            .unwrap();
        assert_eq!(account.wallet_address, wallet);
        assert!(!account.orders.is_empty());
        let maker_status = client
            .platform_maker_status_authorized(PlatformMakerStatusAuthorizedRequest {
                market_id: "market_33333333333333333333333333333333".to_owned(),
                wallet_address: wallet.to_owned(),
                authorization_time_ms: 1_786_550_400_000,
                authorization_signature: "0a".repeat(64),
            })
            .await
            .unwrap();
        assert_eq!(maker_status.active_products, 3);
        assert_eq!(maker_status.strands.len(), 1);
        assert!(maker_status
            .intent
            .as_ref()
            .is_some_and(|intent| intent.active));
        assert_eq!(
            maker_status_auth_message(
                "market_33333333333333333333333333333333",
                wallet,
                1_786_550_400_000
            )
            .unwrap(),
            format!(
                "strata:mm-status-read:v2\nmarket_33333333333333333333333333333333\n{wallet}\n1786550400000"
            )
            .into_bytes()
        );
        let portfolio = client.platform_portfolio(wallet).await.unwrap();
        assert_eq!(portfolio.wallet_address, wallet);
        assert_eq!(portfolio.balances.len(), 2);
        assert_eq!(portfolio.positions.len(), 1);
        assert_eq!(portfolio.equity_usd_micros.as_deref(), Some("439989500"));
        assert!(portfolio.valuation_complete);
        assert_eq!(
            client
                .platform_portfolio_history(wallet, PlatformPortfolioHistoryRange::Day)
                .await
                .unwrap()
                .range,
            PlatformPortfolioHistoryRange::Day
        );
        assert!(client
            .platform_vault_status(
                wallet,
                PlatformVaultStatusRequest {
                    session_public_key: Some(
                        "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2".to_owned(),
                    ),
                },
            )
            .await
            .unwrap()
            .session
            .is_some_and(|session| session.market_execution_ready));
        assert!(
            client
                .platform_vault_pause_prepare(PlatformVaultPausePrepareRequest {
                    wallet_address: wallet.to_owned(),
                    paused: true,
                })
                .await
                .unwrap()
                .owner_signature_required
        );
        let setup = client
            .platform_vault_setup_prepare(PlatformVaultSetupPrepareRequest {
                wallet_address: wallet.to_owned(),
                session_public_key: "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2".to_owned(),
                market_id: Some("market_33333333333333333333333333333333".to_owned()),
                expires_at_ms: None,
                minimum_interval_seconds: None,
                maximum_tolerance_bps: None,
                spending_limits: vec![
                    PlatformVaultSpendingLimit {
                        asset_id: "asset_0123456789abcdef0123456789abcdef".to_owned(),
                        maximum_per_execution_atoms: None,
                    },
                    PlatformVaultSpendingLimit {
                        asset_id: "asset_fedcba9876543210fedcba9876543210".to_owned(),
                        maximum_per_execution_atoms: Some("100000000".to_owned()),
                    },
                ],
            })
            .await
            .unwrap();
        assert_eq!(setup.mode, PlatformVaultSetupMode::Create);
        assert!(setup.owner_signature_required);
        assert_eq!(
            setup.minimum_interval_seconds,
            PLATFORM_SESSION_DEFAULT_MINIMUM_INTERVAL_SECONDS
        );
        assert_eq!(
            setup.maximum_tolerance_bps,
            PLATFORM_SESSION_DEFAULT_MAXIMUM_TOLERANCE_BPS
        );
        // A first deposit that names the session key onboards in the same
        // owner signature.
        let deposit = client
            .platform_vault_deposit_prepare(PlatformVaultDepositPrepareRequest {
                wallet_address: wallet.to_owned(),
                market_id: "market_33333333333333333333333333333333".to_owned(),
                asset_id: "asset_0123456789abcdef0123456789abcdef".to_owned(),
                amount_atoms: "10000000".to_owned(),
                session_public_key: Some("9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(deposit.amount_atoms, "10000000");
        assert!(deposit.owner_signature_required);
        assert!(deposit.sponsored);
        assert!(deposit.registers_session);
        assert_eq!(
            deposit.preparation_id,
            "vp_4d5e6f708192a3b4c5d6e7f8091a2b3c"
        );
        // Owner signs, hands it back: Strata pays and broadcasts, then reports.
        let receipt = client
            .platform_vault_submit(PlatformVaultSubmitRequest {
                preparation_id: deposit.preparation_id.clone(),
                signed_transaction_base64: "AQIDBA==".to_owned(),
                idempotency_key: "deposit-1".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(receipt.action, PlatformVaultAction::Deposit);
        assert_eq!(receipt.status, PlatformVaultSubmissionStatus::Submitted);
        assert!(receipt.sponsored);
        let outcome = client
            .platform_vault_submission(&deposit.preparation_id)
            .await
            .unwrap();
        assert_eq!(outcome.preparation_id, deposit.preparation_id);
        assert!(client
            .platform_vault_submission("or_4d5e6f708192a3b4c5d6e7f8091a2b3c")
            .await
            .is_err());
        let withdrawal = client
            .platform_vault_withdraw_prepare(PlatformVaultWithdrawPrepareRequest {
                wallet_address: wallet.to_owned(),
                market_id: "market_33333333333333333333333333333333".to_owned(),
                asset_id: "asset_fedcba9876543210fedcba9876543210".to_owned(),
                destination_wallet_address: wallet.to_owned(),
                amount_atoms: "5000000".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(withdrawal.amount_atoms, "5000000");
        assert!(withdrawal.owner_signature_required);
        let delegate = client
            .platform_vault_delegate_prepare(PlatformVaultDelegatePrepareRequest {
                wallet_address: wallet.to_owned(),
                session_public_key: "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2".to_owned(),
                action: PlatformVaultDelegateAction::Revoke,
            })
            .await
            .unwrap();
        assert_eq!(delegate.action, PlatformVaultDelegateAction::Revoke);
        assert!(delegate.owner_signature_required);
        let policy = client
            .platform_vault_policy_prepare(PlatformVaultPolicyPrepareRequest {
                wallet_address: wallet.to_owned(),
                withdrawal_access: PlatformVaultWithdrawalAccess {
                    mode: PlatformVaultWithdrawalMode::Restricted,
                    allowed_wallet_addresses: vec![wallet.to_owned()],
                },
            })
            .await
            .unwrap();
        assert_eq!(
            policy.withdrawal_access.mode,
            PlatformVaultWithdrawalMode::Restricted
        );
        assert!(policy.owner_signature_required);
        assert!(client
            .platform_rewards(PlatformRewardsRequest {
                wallet_address: Some(wallet.to_owned()),
                limit: Some(20),
            })
            .await
            .unwrap()
            .owner
            .is_some());
        assert_eq!(
            client
                .platform_referrals(wallet)
                .await
                .unwrap()
                .wallet_address,
            wallet
        );
        assert_eq!(
            client
                .platform_referral_link(PlatformReferralLinkRequest {
                    wallet_address: wallet.to_owned(),
                    referral_code: "STRATA1".to_owned(),
                    authorization_signature: "22".repeat(64),
                })
                .await
                .unwrap()
                .status,
            "pending_first_fill"
        );
        assert_eq!(
            client
                .platform_referral_claim(PlatformReferralClaimRequest {
                    wallet_address: wallet.to_owned(),
                    payout_wallet_address: None,
                    authorization_signature: "33".repeat(64),
                })
                .await
                .unwrap()
                .status,
            "requested"
        );
        assert_eq!(
            client.platform_bugs(wallet).await.unwrap().wallet_address,
            wallet
        );
        assert_eq!(
            client
                .platform_bug_submit(PlatformBugSubmitRequest {
                    owner_wallet: wallet.to_owned(),
                    message: " public report ".to_owned(),
                    authorization_signature: format!("0x{}", "07".repeat(64)),
                })
                .await
                .unwrap()
                .status,
            PlatformBugStatus::Pending
        );
        assert_eq!(
            bug_authorization_payload(" public report ").unwrap(),
            b"strata-bug-report:v1:public report"
        );
        assert_eq!(
            referral_link_authorization_payload(" STRATA1 ").unwrap(),
            b"strata-referral:v1:STRATA1"
        );
        assert_eq!(
            referral_claim_authorization_payload(wallet).unwrap(),
            format!("strata-referral-claim:v1:{wallet}").as_bytes()
        );
    }

    #[tokio::test]
    async fn market_data_stream_fails_closed_on_a_book_sequence_gap() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let market_id = "market_33333333333333333333333333333333";
        let mut snapshot = fixture("book");
        snapshot
            .as_object_mut()
            .unwrap()
            .insert("type".to_owned(), serde_json::json!("book_snapshot"));
        let gap = serde_json::json!({
            "type": "book_delta",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "stream_id": "book:market_33333333333333333333333333333333",
            "sequence": "44",
            "previous_sequence": "42",
            "server_time_ms": 1786550400100u64,
            "changes": [{
                "side": "bid",
                "price_atoms": "149990000",
                "size_atoms": "0"
            }]
        });
        let server = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(connection).await.unwrap();
            socket
                .send(Message::Text(snapshot.to_string().into()))
                .await
                .unwrap();
            socket
                .send(Message::Text(gap.to_string().into()))
                .await
                .unwrap();
            let _ = socket.next().await;
        });

        let client = StrataClient::new(format!("http://{address}")).unwrap();
        seed_platform_capabilities(&client);
        let mut stream = client.connect_market_data(market_id).await.unwrap();
        assert!(matches!(
            stream.next_event().await.unwrap(),
            Some(PlatformMarketDataEvent::BookSnapshot { .. })
        ));
        assert!(matches!(
            stream.next_event().await,
            Err(SdkError::InvalidResponse(message))
                if message == "market stream sequence gap detected"
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn account_stream_signs_the_exact_challenge_and_sequences_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let market_id = "market_33333333333333333333333333333333";
        let wallet = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
        let challenge = "ab".repeat(32);
        let challenge_for_server = challenge.clone();
        let mut snapshot = fixture("account");
        snapshot.as_object_mut().unwrap().extend([
            ("type".to_owned(), serde_json::json!("account_snapshot")),
            (
                "stream_id".to_owned(),
                serde_json::json!("account_stream_66666666666666666666666666666666"),
            ),
            ("sequence".to_owned(), serde_json::json!("1")),
        ]);
        let orders = serde_json::json!({
            "type": "orders_snapshot",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "wallet_address": wallet,
            "stream_id": "account_stream_66666666666666666666666666666666",
            "sequence": "2",
            "previous_sequence": "1",
            "server_time_ms": 1786550400100u64,
            "orders": []
        });
        let server = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(connection).await.unwrap();
            socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "auth_challenge",
                        "schema_version": 2,
                        "contract_version": "2.0",
                        "market_id": market_id,
                        "wallet_address": wallet,
                        "challenge": challenge_for_server,
                        "server_time_ms": 1786550400000u64,
                        "expires_at_ms": 1786550405000u64
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let Message::Text(authentication) = socket.next().await.unwrap().unwrap() else {
                panic!("expected text authentication");
            };
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&authentication).unwrap(),
                serde_json::json!({
                    "type": "authenticate",
                    "signature": "09".repeat(64),
                })
            );
            socket
                .send(Message::Text(snapshot.to_string().into()))
                .await
                .unwrap();
            socket
                .send(Message::Text(orders.to_string().into()))
                .await
                .unwrap();
            let _ = socket.next().await;
        });

        let signer = TestAccountSigner {
            wallet: wallet.to_owned(),
            expected_message: format!(
                "strata:account-stream:v2\n{market_id}\n{wallet}\n{challenge}"
            )
            .into_bytes(),
            signature_byte: 9,
        };
        let client = StrataClient::new(format!("http://{address}")).unwrap();
        seed_platform_capabilities(&client);
        let mut stream = client.connect_account(market_id, &signer).await.unwrap();
        assert!(matches!(
            stream.next_event().await.unwrap(),
            Some(PlatformAccountEvent::AccountSnapshot { .. })
        ));
        assert!(matches!(
            stream.next_event().await.unwrap(),
            Some(PlatformAccountEvent::OrdersSnapshot { .. })
        ));
        stream.close().await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn execution_stream_watches_handles_and_sequences_updates() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let snapshot = fixture("execution-stream");
        let market_id = snapshot["market_id"].as_str().unwrap().to_owned();
        let watched: Vec<String> = vec![
            snapshot["executions"][0]["execution_id"]
                .as_str()
                .unwrap()
                .to_owned(),
            snapshot["executions"][1]["execution_id"]
                .as_str()
                .unwrap()
                .to_owned(),
            snapshot["unknown_execution_ids"][0]
                .as_str()
                .unwrap()
                .to_owned(),
        ];
        let expected_watch = serde_json::json!({"type": "watch", "execution_ids": watched});
        let mut confirmed = snapshot["executions"][1].clone();
        confirmed["status"] = serde_json::json!("confirmed");
        confirmed["signature"] = serde_json::json!("2".repeat(64));
        confirmed["settlement"] = serde_json::json!("confirmed");
        let update = serde_json::json!({
            "type": "execution_update",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "stream_id": snapshot["stream_id"],
            "sequence": "2",
            "previous_sequence": "1",
            "server_time_ms": 1786550400100u64,
            "execution": confirmed,
        });
        let unknown = serde_json::json!({
            "type": "execution_unknown",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "stream_id": snapshot["stream_id"],
            "sequence": "3",
            "previous_sequence": "2",
            "server_time_ms": 1786550400200u64,
            "execution_id": "se_abcdefabcdefabcdefabcdefabcdefab",
        });
        let gap = serde_json::json!({
            "type": "heartbeat",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "stream_id": snapshot["stream_id"],
            "sequence": "5",
            "previous_sequence": "4",
            "server_time_ms": 1786550400300u64,
        });
        let server = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(connection).await.unwrap();
            let Message::Text(watch) = socket.next().await.unwrap().unwrap() else {
                panic!("expected a watch frame");
            };
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&watch).unwrap(),
                expected_watch
            );
            socket
                .send(Message::Text(snapshot.to_string().into()))
                .await
                .unwrap();
            socket
                .send(Message::Text(update.to_string().into()))
                .await
                .unwrap();
            let Message::Text(more) = socket.next().await.unwrap().unwrap() else {
                panic!("expected a second watch frame");
            };
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&more).unwrap(),
                serde_json::json!({"type": "watch", "execution_ids": ["se_abcdefabcdefabcdefabcdefabcdefab"]})
            );
            for frame in [unknown, gap] {
                socket
                    .send(Message::Text(frame.to_string().into()))
                    .await
                    .unwrap();
            }
            let _ = socket.next().await;
        });
        let client = StrataClient::new(format!("http://{address}")).unwrap();
        seed_platform_capabilities(&client);
        let ids: Vec<String> = vec![
            "se_0123456789abcdef0123456789abcdef".to_owned(),
            "se_fedcba9876543210fedcba9876543210".to_owned(),
            "se_00000000000000000000000000000000".to_owned(),
        ];
        let mut stream = client
            .connect_executions("market_33333333333333333333333333333333", &ids)
            .await
            .unwrap();
        match stream.next_event().await.unwrap() {
            Some(PlatformExecutionEvent::ExecutionsSnapshot {
                executions,
                unknown_execution_ids,
                ..
            }) => {
                assert_eq!(executions.len(), 2);
                assert_eq!(unknown_execution_ids.len(), 1);
            }
            other => panic!("expected execution snapshot, got {other:?}"),
        }
        match stream.next_event().await.unwrap() {
            Some(PlatformExecutionEvent::ExecutionUpdate { execution, .. }) => {
                assert_eq!(execution.status, PlatformExecutionState::Confirmed);
            }
            other => panic!("expected execution update, got {other:?}"),
        }
        stream
            .watch(&["se_abcdefabcdefabcdefabcdefabcdefab".to_owned()])
            .await
            .unwrap();
        match stream.next_event().await.unwrap() {
            Some(PlatformExecutionEvent::ExecutionUnknown { execution_id, .. }) => {
                assert_eq!(execution_id, "se_abcdefabcdefabcdefabcdefabcdefab");
            }
            other => panic!("expected execution unknown, got {other:?}"),
        }
        assert!(
            stream.next_event().await.is_err(),
            "a sequence gap must fail closed"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn twap_stream_sequences_progress_and_fails_closed_on_gaps() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let snapshot = fixture("twap-stream");
        let market_id = snapshot["market_id"].as_str().unwrap().to_owned();
        let wallet = snapshot["wallet_address"].as_str().unwrap().to_owned();
        let mut update = serde_json::json!({
            "type": "twap_update",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "wallet_address": wallet,
            "stream_id": snapshot["stream_id"],
            "sequence": "2",
            "previous_sequence": "1",
            "server_time_ms": 1786550400100u64,
        });
        let mut twap = snapshot["twaps"][0].clone();
        let executed = twap["slices_executed"].as_u64().unwrap() + 1;
        twap["slices_executed"] = serde_json::json!(executed);
        update["twap"] = twap;
        let gap = serde_json::json!({
            "type": "heartbeat",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "wallet_address": wallet,
            "stream_id": snapshot["stream_id"],
            "sequence": "4",
            "previous_sequence": "3",
            "server_time_ms": 1786550400200u64,
        });
        let expected_path = format!("/v2/markets/{market_id}/account/{wallet}/twaps/stream");
        let server = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut requested_path = String::new();
            let mut socket = tokio_tungstenite::accept_hdr_async(
                connection,
                |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                 response: tokio_tungstenite::tungstenite::handshake::server::Response| {
                    requested_path = request.uri().path().to_owned();
                    Ok(response)
                },
            )
            .await
            .unwrap();
            assert_eq!(requested_path, expected_path);
            for frame in [snapshot, update, gap] {
                socket
                    .send(Message::Text(frame.to_string().into()))
                    .await
                    .unwrap();
            }
            let _ = socket.next().await;
        });
        let client = StrataClient::new(format!("http://{address}")).unwrap();
        seed_platform_capabilities(&client);
        let mut stream = client.connect_twaps(&market_id, &wallet).await.unwrap();
        match stream.next_event().await.unwrap() {
            Some(PlatformTwapEvent::TwapsSnapshot { twaps, .. }) => assert_eq!(twaps.len(), 1),
            other => panic!("expected TWAP snapshot, got {other:?}"),
        }
        match stream.next_event().await.unwrap() {
            Some(PlatformTwapEvent::TwapUpdate { twap, .. }) => {
                assert_eq!(u64::from(twap.slices_executed), executed);
            }
            other => panic!("expected TWAP update, got {other:?}"),
        }
        assert!(
            stream.next_event().await.is_err(),
            "a sequence gap must fail closed"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn maker_stream_signs_the_exact_challenge_and_sequences_maker_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let market_id = "market_33333333333333333333333333333333";
        let wallet = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
        let challenge = "cd".repeat(32);
        let challenge_for_server = challenge.clone();
        let snapshot = fixture("maker-stream");
        let mut fill_event = serde_json::json!({
            "type": "maker_fill",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "wallet_address": wallet,
            "stream_id": snapshot["stream_id"],
            "sequence": "2",
            "previous_sequence": "1",
            "server_time_ms": 1786896000100u64,
        });
        let mut fill = snapshot["fills"][0].clone();
        fill["fill_id"] = serde_json::json!("fill_99999999999999999999999999999999");
        fill["product"] = serde_json::json!("intent");
        fill_event["fill"] = fill;
        let mut status = snapshot["status"].clone();
        status["intent"] = serde_json::Value::Null;
        status["active_products"] = serde_json::json!(2);
        let status_event = serde_json::json!({
            "type": "maker_status",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "wallet_address": wallet,
            "stream_id": snapshot["stream_id"],
            "sequence": "3",
            "previous_sequence": "2",
            "server_time_ms": 1786896000200u64,
            "status": status,
        });
        let gap = serde_json::json!({
            "type": "heartbeat",
            "schema_version": 2,
            "contract_version": "2.0",
            "market_id": market_id,
            "wallet_address": wallet,
            "stream_id": snapshot["stream_id"],
            "sequence": "5",
            "previous_sequence": "4",
            "server_time_ms": 1786896000300u64,
        });
        let server = tokio::spawn(async move {
            let (connection, _) = listener.accept().await.unwrap();
            let mut socket = tokio_tungstenite::accept_async(connection).await.unwrap();
            socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "auth_challenge",
                        "schema_version": 2,
                        "contract_version": "2.0",
                        "market_id": market_id,
                        "wallet_address": wallet,
                        "challenge": challenge_for_server,
                        "server_time_ms": 1786896000000u64,
                        "expires_at_ms": 1786896005000u64
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let Message::Text(authentication) = socket.next().await.unwrap().unwrap() else {
                panic!("expected text authentication");
            };
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&authentication).unwrap(),
                serde_json::json!({
                    "type": "authenticate",
                    "signature": "07".repeat(64),
                })
            );
            for frame in [snapshot, fill_event, status_event, gap] {
                socket
                    .send(Message::Text(frame.to_string().into()))
                    .await
                    .unwrap();
            }
            let _ = socket.next().await;
        });

        let signer = TestAccountSigner {
            wallet: wallet.to_owned(),
            expected_message: format!(
                "strata:mm-fills-stream:v2\n{market_id}\n{wallet}\n{challenge}"
            )
            .into_bytes(),
            signature_byte: 7,
        };
        let client = StrataClient::new(format!("http://{address}")).unwrap();
        seed_platform_capabilities(&client);
        let mut stream = client.connect_maker(market_id, &signer).await.unwrap();
        match stream.next_event().await.unwrap() {
            Some(PlatformMakerEvent::MakerSnapshot { status, fills, .. }) => {
                assert_eq!(status.active_products, 3);
                assert_eq!(fills.len(), 1);
            }
            other => panic!("expected maker snapshot, got {other:?}"),
        }
        match stream.next_event().await.unwrap() {
            Some(PlatformMakerEvent::MakerFill { fill, .. }) => {
                assert_eq!(fill.product, PlatformMakerProduct::Intent);
            }
            other => panic!("expected maker fill, got {other:?}"),
        }
        match stream.next_event().await.unwrap() {
            Some(PlatformMakerEvent::MakerStatus { status, .. }) => {
                assert!(status.intent.is_none());
                assert_eq!(status.active_products, 2);
            }
            other => panic!("expected maker status, got {other:?}"),
        }
        assert!(
            stream.next_event().await.is_err(),
            "a sequence gap must fail closed"
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn resting_order_calls_use_only_product_paths_and_external_signatures() {
        let server = MockServer::start().await;
        let market_id = "market_22222222222222222222222222222222";
        let owner_wallet = bs58::encode([1u8; 32]).into_string();
        let session_public_key = bs58::encode([2u8; 32]).into_string();
        let authorization_signature = bs58::encode([3u8; 64]).into_string();
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/orders/challenge")))
            .and(body_json(serde_json::json!({
                "action": "place",
                "owner_wallet": owner_wallet,
                "session_public_key": session_public_key,
                "account_sequence": "7",
                "client_order_id": "agent-order-7",
                "side": "buy",
                "order_type": "post_only",
                "limit_price_atoms": "150000000",
                "size_atoms": "1000000"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("order-challenge")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/orders/prepare")))
            .and(body_json(serde_json::json!({
                "challenge_id": "oc_11111111111111111111111111111111",
                "authorization_signature": authorization_signature
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("order-prepare")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/orders/submit")))
            .and(body_json(serde_json::json!({
                "order_control_id": "or_44444444444444444444444444444444",
                "signed_transaction_base64": "AQIDBA==",
                "idempotency_key": "order-attempt-7"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("order-submit")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/orders/status")))
            .and(body_json(serde_json::json!({
                "order_control_id": "or_44444444444444444444444444444444",
                "idempotency_key": "order-attempt-7"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("order-status")))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        seed_platform_capabilities(&client);
        let challenge = client
            .order_challenge(
                market_id,
                PlatformOrderChallengeRequest::Place {
                    owner_wallet,
                    session_public_key,
                    account_sequence: Some("7".to_owned()),
                    client_order_id: "agent-order-7".to_owned(),
                    side: PlatformTradeSide::Buy,
                    order_type: PlatformOrderType::PostOnly,
                    limit_price_atoms: "150000000".to_owned(),
                    size_atoms: "1000000".to_owned(),
                },
            )
            .await
            .unwrap();
        let prepared = client
            .order_prepare(
                market_id,
                PlatformOrderPrepareRequest::Authorized(PlatformOrderPrepareAuthorization {
                    challenge_id: challenge.challenge_id,
                    authorization_signature: Some(authorization_signature),
                }),
            )
            .await
            .unwrap();
        let receipt = client
            .order_submit(
                market_id,
                PlatformOrderSubmitRequest {
                    order_control_id: prepared.order_control_id,
                    signed_transaction_base64: "AQIDBA==".to_owned(),
                    idempotency_key: "order-attempt-7".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, PlatformOrderSubmissionStatus::Submitted);
        let status = client
            .order_status(
                market_id,
                PlatformOrderStatusRequest {
                    order_control_id: receipt.order_control_id,
                    idempotency_key: "order-attempt-7".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(status.status, PlatformOrderControlStatus::Submitting);
    }

    #[tokio::test]
    async fn twap_calls_use_only_product_paths_and_external_signatures() {
        let server = MockServer::start().await;
        let market_id = "market_22222222222222222222222222222222";
        let owner_wallet = bs58::encode([1u8; 32]).into_string();
        let session_public_key = bs58::encode([2u8; 32]).into_string();
        let authorization_signature = bs58::encode([3u8; 64]).into_string();
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/twaps/challenge")))
            .and(body_json(serde_json::json!({
                "action": "place",
                "owner_wallet": owner_wallet,
                "session_public_key": session_public_key,
                "side": "buy",
                "total_size_atoms": "10000000",
                "slices_total": 10,
                "maximum_tolerance_bps": 100,
                "interval_slots": 100,
                "limit_price_atoms": "150000000"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("twap-challenge")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/twaps/prepare")))
            .and(body_json(serde_json::json!({
                "challenge_id": "twc_0123456789abcdef0123456789abcdef",
                "authorization_signature": authorization_signature
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("twap-prepare")))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/twaps/submit")))
            .and(body_json(serde_json::json!({
                "twap_control_id": "twctl_44444444444444444444444444444444",
                "signed_transaction_base64": "AQIDBA==",
                "idempotency_key": "twap-attempt-7"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("twap-submit")))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        seed_platform_capabilities(&client);
        let challenge = client
            .twap_challenge(
                market_id,
                PlatformTwapChallengeRequest::Place {
                    owner_wallet,
                    session_public_key,
                    side: PlatformTradeSide::Buy,
                    total_size_atoms: "10000000".to_owned(),
                    slices_total: 10,
                    maximum_tolerance_bps: 100,
                    interval_slots: 100,
                    limit_price_atoms: "150000000".to_owned(),
                },
            )
            .await
            .unwrap();
        let prepared = client
            .twap_prepare(
                market_id,
                PlatformTwapPrepareRequest::Authorized(PlatformTwapPrepareAuthorization {
                    challenge_id: challenge.challenge_id,
                    authorization_signature,
                }),
            )
            .await
            .unwrap();
        let receipt = client
            .twap_submit(
                market_id,
                PlatformTwapSubmitRequest {
                    twap_control_id: prepared.twap_control_id,
                    signed_transaction_base64: "AQIDBA==".to_owned(),
                    idempotency_key: "twap-attempt-7".to_owned(),
                },
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, PlatformOrderSubmissionStatus::Submitted);
        assert_eq!(receipt.action, PlatformTwapControlAction::Place);
    }

    /// A session signer for the one-signature helpers: it signs transactions
    /// only and fails the test if a message signature is ever requested.
    struct OneSignatureSigner {
        expected_transaction: String,
    }

    #[async_trait]
    impl SessionSigner for OneSignatureSigner {
        fn public_key(&self) -> &str {
            transaction_verifier::test_support::SESSION_PUBLIC_KEY
        }

        async fn sign_message(&self, _message: &[u8]) -> Result<Vec<u8>, String> {
            panic!("one-signature path must not sign a message");
        }

        async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, String> {
            assert_eq!(transaction_base64, self.expected_transaction);
            Ok(transaction_base64.to_owned())
        }
    }

    struct MessageMutatingSigner {
        expected_transaction: String,
    }

    #[async_trait]
    impl SessionSigner for MessageMutatingSigner {
        fn public_key(&self) -> &str {
            transaction_verifier::test_support::SESSION_PUBLIC_KEY
        }

        async fn sign_message(&self, _message: &[u8]) -> Result<Vec<u8>, String> {
            panic!("one-signature path must not sign a message");
        }

        async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, String> {
            assert_eq!(transaction_base64, self.expected_transaction);
            let mut transaction = base64::engine::general_purpose::STANDARD
                .decode(transaction_base64)
                .unwrap();
            let last = transaction.last_mut().unwrap();
            *last ^= 1;
            Ok(base64::engine::general_purpose::STANDARD.encode(transaction))
        }
    }

    /// Records what a custom verifier is handed on the direct path.
    struct RecordingVerifier {
        market_id: String,
        seen: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait]
    impl OrderVerifier for RecordingVerifier {
        async fn verify(&self, context: &OrderVerificationContext<'_>) -> Result<(), String> {
            assert!(context.challenge.is_none());
            assert_eq!(context.market_id, self.market_id);
            assert_eq!(context.prepared.market_id, self.market_id);
            assert_eq!(
                context.owner_wallet,
                transaction_verifier::test_support::OWNER_WALLET
            );
            assert_eq!(
                context.session_public_key,
                transaction_verifier::test_support::SESSION_PUBLIC_KEY
            );
            assert_eq!(
                order_request_action(context.operation),
                context.prepared.action
            );
            self.seen.lock().unwrap().push("order".to_owned());
            Ok(())
        }
    }

    #[async_trait]
    impl TwapVerifier for RecordingVerifier {
        async fn verify(&self, context: &TwapVerificationContext<'_>) -> Result<(), String> {
            assert!(context.challenge.is_none());
            assert_eq!(context.market_id, self.market_id);
            assert_eq!(context.prepared.market_id, self.market_id);
            assert_eq!(
                context.owner_wallet,
                transaction_verifier::test_support::OWNER_WALLET
            );
            assert_eq!(
                twap_request_action(context.operation),
                context.prepared.action
            );
            self.seen.lock().unwrap().push("twap".to_owned());
            Ok(())
        }
    }

    #[async_trait]
    impl ExecutionVerifier for RecordingVerifier {
        async fn verify(&self, context: &ExecutionVerificationContext<'_>) -> Result<(), String> {
            assert!(context.challenge.is_none());
            assert_eq!(context.prepared.quote_id, context.quote.quote_id);
            assert_eq!(context.prepared.market_id, self.market_id);
            assert_eq!(
                context.owner_wallet,
                transaction_verifier::test_support::OWNER_WALLET
            );
            self.seen.lock().unwrap().push("execution".to_owned());
            Ok(())
        }
    }

    #[tokio::test]
    async fn execute_order_uses_one_signature_over_a_verified_direct_prepare() {
        use transaction_verifier::test_support::{
            market_id, order_id, place_transaction, recent_blockhash, PlaceTransactionOptions,
            OWNER_WALLET, PLACE_PRICE, PLACE_SIZE, SESSION_PUBLIC_KEY,
        };
        let server = MockServer::start().await;
        let market_id = market_id();
        let transaction = place_transaction(PlaceTransactionOptions::default());
        let mut prepared = fixture("order-prepare");
        prepared["market_id"] = serde_json::json!(market_id);
        prepared["order_ids"] = serde_json::json!([order_id()]);
        prepared["transaction_base64"] = serde_json::json!(transaction);
        prepared["recent_blockhash"] = serde_json::json!(recent_blockhash());
        let mut submitted = fixture("order-submit");
        submitted["market_id"] = serde_json::json!(market_id);
        submitted["order_ids"] = serde_json::json!([order_id()]);
        // Direct prepare: the operation itself is the body — no challenge, no
        // challenge_id, no message signature.
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/orders/prepare")))
            .and(body_json(serde_json::json!({
                "action": "place",
                "owner_wallet": OWNER_WALLET,
                "session_public_key": SESSION_PUBLIC_KEY,
                "client_order_id": "agent-42",
                "side": "buy",
                "order_type": "post_only",
                "limit_price_atoms": PLACE_PRICE.to_string(),
                "size_atoms": PLACE_SIZE.to_string()
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(prepared))
            .expect(3)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/orders/submit")))
            .and(body_json(serde_json::json!({
                "order_control_id": "or_44444444444444444444444444444444",
                "signed_transaction_base64": transaction,
                "idempotency_key": "or_44444444444444444444444444444444"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(submitted))
            .expect(2)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        seed_platform_capabilities(&client);
        let signer = OneSignatureSigner {
            expected_transaction: transaction.clone(),
        };
        let operation = OrderExecuteOperation::Place {
            owner_wallet: OWNER_WALLET.to_owned(),
            account_sequence: None,
            client_order_id: "agent-42".to_owned(),
            side: PlatformTradeSide::Buy,
            order_type: PlatformOrderType::PostOnly,
            limit_price_atoms: PLACE_PRICE.to_string(),
            size_atoms: PLACE_SIZE.to_string(),
        };
        // Built-in verifier: the SDK decodes the transaction and requires it
        // to be exactly this operation before the one signature.
        let receipt = client
            .execute_order(
                &market_id,
                &operation,
                &signer,
                &DefaultTransactionVerifier,
                None,
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, PlatformOrderSubmissionStatus::Submitted);
        assert_eq!(receipt.order_ids, vec![order_id()]);

        // A custom verifier still receives the operation and prepared
        // transaction, with no challenge.
        let recording = RecordingVerifier {
            market_id: market_id.clone(),
            seen: std::sync::Mutex::new(Vec::new()),
        };
        client
            .execute_order(&market_id, &operation, &signer, &recording, None)
            .await
            .unwrap();
        assert_eq!(*recording.seen.lock().unwrap(), vec!["order".to_owned()]);

        let error = client
            .execute_order(
                &market_id,
                &operation,
                &MessageMutatingSigner {
                    expected_transaction: transaction,
                },
                &DefaultTransactionVerifier,
                None,
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            SdkError::Verification(message)
                if message.contains("signed transaction message changed after verification")
        ));
    }

    #[tokio::test]
    async fn execute_order_refuses_a_transaction_that_is_not_the_operation() {
        use transaction_verifier::test_support::{
            market_id, order_id, place_transaction, recent_blockhash, PlaceTransactionOptions,
            OWNER_WALLET, PLACE_PRICE, PLACE_SIZE,
        };
        let market_id = market_id();
        let operation = OrderExecuteOperation::Place {
            owner_wallet: OWNER_WALLET.to_owned(),
            account_sequence: None,
            client_order_id: "agent-42".to_owned(),
            side: PlatformTradeSide::Buy,
            order_type: PlatformOrderType::PostOnly,
            limit_price_atoms: PLACE_PRICE.to_string(),
            size_atoms: PLACE_SIZE.to_string(),
        };
        // Built-in verifier refusals: a different side, the session as fee
        // payer, a session-signed system transfer, another market.
        let cases = [
            (
                PlaceTransactionOptions {
                    side: 1,
                    ..PlaceTransactionOptions::default()
                },
                "exactly the requested orders",
            ),
            (
                PlaceTransactionOptions {
                    session_pays: true,
                    ..PlaceTransactionOptions::default()
                },
                "fee payer",
            ),
            (
                PlaceTransactionOptions {
                    extra_system_transfer: true,
                    ..PlaceTransactionOptions::default()
                },
                "system or token instruction",
            ),
            (
                PlaceTransactionOptions {
                    market: Some([7; 32]),
                    ..PlaceTransactionOptions::default()
                },
                "another market",
            ),
        ];
        for (options, expected) in cases {
            let server = MockServer::start().await;
            let transaction = place_transaction(options);
            let mut prepared = fixture("order-prepare");
            prepared["market_id"] = serde_json::json!(market_id);
            prepared["order_ids"] = serde_json::json!([order_id()]);
            prepared["transaction_base64"] = serde_json::json!(transaction);
            prepared["recent_blockhash"] = serde_json::json!(recent_blockhash());
            Mock::given(method("POST"))
                .and(path(format!("/v2/markets/{market_id}/orders/prepare")))
                .respond_with(ResponseTemplate::new(200).set_body_json(prepared))
                .expect(1)
                .mount(&server)
                .await;
            // No submit mount: a refusal must stop before signing.
            let client = StrataClient::new(server.uri()).unwrap();
            seed_platform_capabilities(&client);
            let signer = OneSignatureSigner {
                expected_transaction: "never signed".to_owned(),
            };
            let error = client
                .execute_order(
                    &market_id,
                    &operation,
                    &signer,
                    &DefaultTransactionVerifier,
                    None,
                )
                .await
                .unwrap_err();
            match error {
                SdkError::Verification(message) => {
                    assert!(message.contains(expected), "{message}")
                }
                other => panic!("expected a verification refusal, got {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn execute_twap_uses_the_direct_prepare_body_and_one_signature() {
        use transaction_verifier::test_support::{
            place_transaction, PlaceTransactionOptions, OWNER_WALLET, SESSION_PUBLIC_KEY,
        };
        let server = MockServer::start().await;
        let market_id = "market_22222222222222222222222222222222";
        let transaction = place_transaction(PlaceTransactionOptions::default());
        let mut prepared = fixture("twap-prepare");
        prepared["transaction_base64"] = serde_json::json!(transaction);
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/twaps/prepare")))
            .and(body_json(serde_json::json!({
                "action": "place",
                "owner_wallet": OWNER_WALLET,
                "session_public_key": SESSION_PUBLIC_KEY,
                "side": "buy",
                "total_size_atoms": "10000000",
                "slices_total": 10,
                "maximum_tolerance_bps": 100,
                "interval_slots": 100,
                "limit_price_atoms": "150000000"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(prepared))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/v2/markets/{market_id}/twaps/submit")))
            .and(body_json(serde_json::json!({
                "twap_control_id": "twctl_44444444444444444444444444444444",
                "signed_transaction_base64": transaction,
                "idempotency_key": "twap-attempt-7"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("twap-submit")))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        seed_platform_capabilities(&client);
        let signer = OneSignatureSigner {
            expected_transaction: transaction,
        };
        let recording = RecordingVerifier {
            market_id: market_id.to_owned(),
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let receipt = client
            .execute_twap(
                market_id,
                &TwapExecuteOperation::Place {
                    owner_wallet: OWNER_WALLET.to_owned(),
                    side: PlatformTradeSide::Buy,
                    total_size_atoms: "10000000".to_owned(),
                    slices_total: 10,
                    maximum_tolerance_bps: 100,
                    interval_slots: 100,
                    limit_price_atoms: "150000000".to_owned(),
                },
                &signer,
                &recording,
                Some("twap-attempt-7"),
            )
            .await
            .unwrap();
        assert_eq!(receipt.status, PlatformOrderSubmissionStatus::Submitted);
        assert_eq!(*recording.seen.lock().unwrap(), vec!["twap".to_owned()]);
    }

    #[tokio::test]
    async fn execute_quote_uses_the_direct_prepare_body_and_one_signature() {
        use transaction_verifier::test_support::{OWNER_WALLET, SESSION_PUBLIC_KEY};
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sonar/markets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("markets")))
            .expect(1)
            .mount(&server)
            .await;
        let mut quote: QuoteResponse = serde_json::from_value(fixture("quote")).unwrap();
        quote.expires_at_ms = unix_ms().unwrap() + 60_000;
        let prepared = fixture("execution-prepare");
        Mock::given(method("POST"))
            .and(path("/sonar/markets/sol-usdc/execution/prepare"))
            .and(body_json(serde_json::json!({
                "quote_id": quote.quote_id,
                "owner_wallet": OWNER_WALLET,
                "session_public_key": SESSION_PUBLIC_KEY
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(prepared.clone()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/sonar/markets/sol-usdc/execution/submit"))
            .and(body_json(serde_json::json!({
                "execution_id": prepared["execution_id"],
                "signed_transaction_base64": prepared["transaction_base64"],
                "idempotency_key": prepared["execution_id"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("execution-submit")))
            .expect(1)
            .mount(&server)
            .await;

        let client = StrataClient::new(server.uri()).unwrap();
        let signer = OneSignatureSigner {
            expected_transaction: prepared["transaction_base64"].as_str().unwrap().to_owned(),
        };
        let recording = RecordingVerifier {
            market_id: quote.market_id.clone(),
            seen: std::sync::Mutex::new(Vec::new()),
        };
        let receipt = client
            .execute_quote(&quote, OWNER_WALLET, None, &signer, &recording, None)
            .await
            .unwrap();
        assert_eq!(receipt.status, ExecutionStatus::Submitted);
        assert_eq!(
            *recording.seen.lock().unwrap(),
            vec!["execution".to_owned()]
        );
    }

    #[test]
    fn twap_authorization_parser_binds_every_public_place_field() {
        let owner = [1u8; 32];
        let session = [2u8; 32];
        let pda = [3u8; 32];
        let blockhash = [4u8; 32];
        let nonce = [5u8; 16];
        let expires_at_ms = 1_786_550_460_000u64;
        let request = PlatformTwapChallengeRequest::Place {
            owner_wallet: bs58::encode(owner).into_string(),
            session_public_key: bs58::encode(session).into_string(),
            side: PlatformTradeSide::Buy,
            total_size_atoms: "10000000".to_owned(),
            slices_total: 10,
            maximum_tolerance_bps: 100,
            interval_slots: 100,
            limit_price_atoms: "150000000".to_owned(),
        };
        let mut payload = Vec::new();
        payload.extend_from_slice(PUBLIC_TWAP_AUTH_DOMAIN);
        payload.extend_from_slice(&[9u8; 32]);
        payload.extend_from_slice(&[8u8; 32]);
        payload.extend_from_slice(&owner);
        payload.extend_from_slice(&session);
        payload.push(0);
        payload.push(0);
        payload.extend_from_slice(&10_000_000u64.to_le_bytes());
        payload.extend_from_slice(&10u16.to_le_bytes());
        payload.extend_from_slice(&100u16.to_le_bytes());
        payload.extend_from_slice(&100u32.to_le_bytes());
        payload.extend_from_slice(&150_000_000u64.to_le_bytes());
        payload.extend_from_slice(&7u64.to_le_bytes());
        payload.extend_from_slice(&pda);
        payload.extend_from_slice(&blockhash);
        payload.extend_from_slice(&123u64.to_le_bytes());
        payload.extend_from_slice(&expires_at_ms.to_le_bytes());
        payload.extend_from_slice(&nonce);
        let challenge = PlatformTwapChallengeResponse {
            schema_version: 2,
            contract_version: "2.0".to_owned(),
            challenge_id: format!("twc_{}", hex::encode(nonce)),
            market_id: "market_22222222222222222222222222222222".to_owned(),
            action: PlatformTwapControlAction::Place,
            twap_id: opaque_twap_id(&pda),
            authorization_payload_base64: base64::engine::general_purpose::STANDARD
                .encode(&payload),
            server_time_ms: expires_at_ms - 60_000,
            expires_at_ms,
        };
        let authorization = validate_twap_authorization(&challenge, &request).unwrap();
        assert_eq!(authorization.bytes, payload);
        assert_eq!(authorization.last_valid_block_height, 123);
        assert_eq!(
            authorization.recent_blockhash,
            bs58::encode(blockhash).into_string()
        );

        let mut changed = request.clone();
        if let PlatformTwapChallengeRequest::Place {
            total_size_atoms, ..
        } = &mut changed
        {
            *total_size_atoms = "10000001".to_owned();
        }
        assert!(validate_twap_authorization(&challenge, &changed).is_err());
    }

    #[test]
    fn order_authorization_parser_binds_every_public_place_field() {
        let owner = [1u8; 32];
        let session = [2u8; 32];
        let order = [3u8; 32];
        let nonce = [4u8; 16];
        let blockhash = [5u8; 32];
        let epoch = [6u8; 16];
        let market_id = "market_22222222222222222222222222222222";
        let expires_at_ms = 1_786_550_460_000u64;
        let request = PlatformOrderChallengeRequest::Place {
            owner_wallet: bs58::encode(owner).into_string(),
            session_public_key: bs58::encode(session).into_string(),
            account_sequence: Some("7".to_owned()),
            client_order_id: "agent-order-7".to_owned(),
            side: PlatformTradeSide::Buy,
            order_type: PlatformOrderType::PostOnly,
            limit_price_atoms: "150000000".to_owned(),
            size_atoms: "1000000".to_owned(),
        };
        let mut payload = Vec::new();
        payload.extend_from_slice(PUBLIC_ORDER_AUTH_DOMAIN);
        payload.extend_from_slice(&[9u8; 32]);
        payload.extend_from_slice(&owner);
        payload.extend_from_slice(&session);
        payload.push(0);
        payload.extend_from_slice(&7u64.to_le_bytes());
        payload.extend_from_slice(&("agent-order-7".len() as u16).to_le_bytes());
        payload.extend_from_slice(b"agent-order-7");
        payload.push(0);
        payload.push(3);
        payload.extend_from_slice(&150_000_000u64.to_le_bytes());
        payload.extend_from_slice(&1_000_000u64.to_le_bytes());
        payload.extend_from_slice(&order);
        payload.extend_from_slice(&blockhash);
        payload.extend_from_slice(&400_000_000u64.to_le_bytes());
        payload.extend_from_slice(&expires_at_ms.to_le_bytes());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&epoch);
        let challenge = PlatformOrderChallengeResponse {
            schema_version: 2,
            contract_version: "2.0".to_owned(),
            challenge_id: format!("oc_{}", hex::encode(nonce)),
            market_id: market_id.to_owned(),
            action: PlatformOrderAction::Place,
            order_ids: vec![opaque_order_id(market_id, &order)],
            authorization_payload_base64: base64::engine::general_purpose::STANDARD.encode(payload),
            server_time_ms: expires_at_ms - 60_000,
            expires_at_ms,
        };
        let authorization = validate_order_authorization(&challenge, &request).unwrap();
        assert_eq!(
            authorization.recent_blockhash,
            bs58::encode(blockhash).into_string()
        );
        assert_eq!(authorization.last_valid_block_height, 400_000_000);

        // A sequence left to Strata is accepted from the signed authorization
        // while every other binding is still enforced; a supplied sequence
        // that differs from it is rejected.
        let mut resolved = request.clone();
        if let PlatformOrderChallengeRequest::Place {
            account_sequence, ..
        } = &mut resolved
        {
            *account_sequence = None;
        }
        assert!(validate_order_authorization(&challenge, &resolved).is_ok());
        let mut pinned_elsewhere = request.clone();
        if let PlatformOrderChallengeRequest::Place {
            account_sequence, ..
        } = &mut pinned_elsewhere
        {
            *account_sequence = Some("8".to_owned());
        }
        assert!(validate_order_authorization(&challenge, &pinned_elsewhere).is_err());

        let mut changed = request;
        if let PlatformOrderChallengeRequest::Place { size_atoms, .. } = &mut changed {
            *size_atoms = "1000001".to_owned();
        }
        assert!(validate_order_authorization(&challenge, &changed).is_err());
    }

    #[test]
    fn order_authorization_parser_binds_atomic_batch_order_and_replacement_fields() {
        let owner = [1u8; 32];
        let session = [2u8; 32];
        let cancelled = [3u8; 32];
        let replaced = [4u8; 32];
        let replacement = [5u8; 32];
        let nonce = [6u8; 16];
        let blockhash = [7u8; 32];
        let market_id = "market_22222222222222222222222222222222";
        let expires_at_ms = 1_786_550_460_000u64;
        let request = PlatformOrderChallengeRequest::Batch {
            owner_wallet: bs58::encode(owner).into_string(),
            session_public_key: bs58::encode(session).into_string(),
            operations: vec![
                PlatformOrderBatchOperation::Cancel {
                    order_id: opaque_order_id(market_id, &cancelled),
                },
                PlatformOrderBatchOperation::Replace {
                    order_id: opaque_order_id(market_id, &replaced),
                    account_sequence: Some("8".to_owned()),
                    client_order_id: "replacement-8".to_owned(),
                    side: PlatformTradeSide::Sell,
                    order_type: PlatformOrderType::PostOnly,
                    limit_price_atoms: "151000000".to_owned(),
                    size_atoms: "2000000".to_owned(),
                },
            ],
        };
        let mut payload = Vec::new();
        payload.extend_from_slice(PUBLIC_ORDER_AUTH_DOMAIN);
        payload.extend_from_slice(&[9u8; 32]);
        payload.extend_from_slice(&owner);
        payload.extend_from_slice(&session);
        payload.push(4);
        payload.push(2);
        payload.push(1);
        payload.extend_from_slice(&cancelled);
        payload.push(1);
        payload.push(3);
        payload.extend_from_slice(&replaced);
        payload.push(0);
        payload.extend_from_slice(&8u64.to_le_bytes());
        payload.extend_from_slice(&("replacement-8".len() as u16).to_le_bytes());
        payload.extend_from_slice(b"replacement-8");
        payload.push(1);
        payload.push(3);
        payload.extend_from_slice(&151_000_000u64.to_le_bytes());
        payload.extend_from_slice(&2_000_000u64.to_le_bytes());
        payload.extend_from_slice(&replacement);
        payload.extend_from_slice(&blockhash);
        payload.extend_from_slice(&400_000_000u64.to_le_bytes());
        payload.extend_from_slice(&expires_at_ms.to_le_bytes());
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(&[8u8; 16]);
        let challenge = PlatformOrderChallengeResponse {
            schema_version: 2,
            contract_version: "2.0".to_owned(),
            challenge_id: format!("oc_{}", hex::encode(nonce)),
            market_id: market_id.to_owned(),
            action: PlatformOrderAction::Batch,
            order_ids: vec![
                opaque_order_id(market_id, &cancelled),
                opaque_order_id(market_id, &replaced),
                opaque_order_id(market_id, &replacement),
            ],
            authorization_payload_base64: base64::engine::general_purpose::STANDARD.encode(payload),
            server_time_ms: expires_at_ms - 60_000,
            expires_at_ms,
        };
        validate_order_authorization(&challenge, &request).unwrap();

        let mut changed = request;
        if let PlatformOrderChallengeRequest::Batch { operations, .. } = &mut changed {
            if let PlatformOrderBatchOperation::Replace { size_atoms, .. } = &mut operations[1] {
                *size_atoms = "2000001".to_owned();
            }
        }
        assert!(validate_order_authorization(&challenge, &changed).is_err());
    }

    #[test]
    fn maker_quickstart_derives_current_atoms_levels_and_expiry() {
        let assets: PlatformAssetsResponse = serde_json::from_value(fixture("assets")).unwrap();
        let mut base_asset = assets
            .assets
            .into_iter()
            .find(|asset| asset.symbol == "SOL")
            .unwrap();
        base_asset.symbol = "WSOL".to_owned();
        base_asset.name = "Wrapped SOL".to_owned();
        let request = PlatformMakerQuickstartRequest {
            market: "SOL/USDC".to_owned(),
            product: PlatformMakerControlProduct::Current,
            spread_bps: 5,
            size: "0.01 SOL".to_owned(),
            duration: Some("10m".to_owned()),
            levels: None,
            level_step_bps: None,
            side: PlatformMakerQuickstartSide::Both,
            async_only: false,
        };
        let operation = maker_quickstart_operation(
            "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL",
            &request,
            &base_asset,
            &request.market,
            1_000,
            150_000_000,
            10_000,
        )
        .unwrap();
        assert_eq!(
            human_base_atoms("0.01 WSOL", &base_asset, &request.market).unwrap(),
            10_000_000
        );
        assert!(human_base_atoms("0.01 BTC", &base_asset, &request.market).is_err());
        let PlatformMakerQuickstartOperation::Current(PlatformMakerCurrentPrepareRequest::Upsert {
            max_exposure_base_atoms,
            bid_depth_base_atoms,
            ask_depth_base_atoms,
            valid_until_slot,
            ..
        }) = operation
        else {
            panic!("expected Current upsert");
        };
        assert_eq!(max_exposure_base_atoms, "10000000");
        assert_eq!(valid_until_slot, "2500");
        assert_eq!(
            &bid_depth_base_atoms[..4],
            &["3333334", "3333333", "3333333", "0"]
        );
        assert_eq!(bid_depth_base_atoms, ask_depth_base_atoms);
        assert!(same_maker_depth(
            ["3333334", "3333333", "3333333"].into_iter(),
            &bid_depth_base_atoms,
        ));
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
