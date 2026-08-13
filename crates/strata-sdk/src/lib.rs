//! Official Rust client for Strata markets and Sonar quotes.
//!
//! It provides typed requests and responses and validates compatibility, quote
//! binding, and economic fields before returning data to the application.

use async_trait::async_trait;
use base64::Engine as _;
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use strata_public_contract::{ErrorResponse, CONTRACT_MAJOR, CONTRACT_VERSION};
use thiserror::Error;

pub use strata_public_contract::platform::{
    PlatformOrderAction, PlatformOrderBatchOperation, PlatformOrderChallengeRequest,
    PlatformOrderChallengeResponse, PlatformOrderControlStatus, PlatformOrderPrepareRequest,
    PlatformOrderPrepareResponse, PlatformOrderStatusRequest, PlatformOrderStatusResponse,
    PlatformOrderSubmissionStatus, PlatformOrderSubmitRequest, PlatformOrderSubmitResponse,
    PlatformOrderType, PlatformTradeSide,
};
pub use strata_public_contract::{
    ActionAuthorityModel, ActionEdge, ActionGraph, ActionNode, ActionNodeKind, ActionOperation,
    CapabilityCatalog, CapabilityDescriptor, CapabilityRisk, CapabilityStability,
    ExecutionChallengeRequest, ExecutionChallengeResponse, ExecutionPrepareRequest,
    ExecutionPrepareResponse, ExecutionStatus, ExecutionSubmitRequest, ExecutionSubmitResponse,
    Market, MarketsResponse, McpExposure, QuoteRequest, QuoteResponse, QuoteSide,
    DEFAULT_SLIPPAGE_BPS,
};

pub const DEFAULT_API_BASE: &str = "https://api.stratabook.app";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const PUBLIC_EXECUTION_AUTH_DOMAIN: &[u8] = b"strata-sonar-execution:v1\0";
const PUBLIC_ORDER_AUTH_DOMAIN: &[u8] = b"strata-platform-order-control:v1\0";

#[async_trait]
pub trait SessionSigner: Send + Sync {
    /// Canonical base58 Ed25519 public key registered as the Vault delegate.
    fn public_key(&self) -> &str;

    /// Sign the exact SDK-validated public operation authorization.
    async fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, String>;

    /// Add only the session signature to an already-verified transaction.
    async fn sign_transaction(&self, transaction_base64: &str) -> Result<String, String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrderExecuteOperation {
    Place {
        owner_wallet: String,
        account_sequence: String,
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
        account_sequence: String,
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
    fn challenge_request(&self, session_public_key: String) -> PlatformOrderChallengeRequest {
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

#[derive(Debug)]
pub struct OrderVerificationContext<'a> {
    pub challenge: &'a PlatformOrderChallengeResponse,
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

    /// Request canonical authorization bytes for an external signer. This
    /// operation accepts public identity only; signing material stays external.
    pub async fn execution_challenge(
        &self,
        market: &str,
        request: ExecutionChallengeRequest,
    ) -> Result<ExecutionChallengeResponse, SdkError> {
        if !valid_handle(&request.quote_id, "sq_") {
            return Err(SdkError::InvalidRequest("quote_id is invalid".to_owned()));
        }
        let request = ExecutionChallengeRequest {
            quote_id: request.quote_id,
            owner_wallet: canonical_public_key(&request.owner_wallet, "owner_wallet")?,
            session_public_key: canonical_public_key(
                &request.session_public_key,
                "session_public_key",
            )?,
            account_sequence: parse_atoms("account_sequence", &request.account_sequence)?
                .to_string(),
        };
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

    /// Exchange an external authorization signature for a quote-bound,
    /// partially signed transaction.
    pub async fn execution_prepare(
        &self,
        market: &str,
        request: ExecutionPrepareRequest,
    ) -> Result<ExecutionPrepareResponse, SdkError> {
        if !valid_handle(&request.challenge_id, "sc_") {
            return Err(SdkError::InvalidRequest(
                "challenge_id is invalid".to_owned(),
            ));
        }
        let signature = bs58::decode(request.authorization_signature.trim())
            .into_vec()
            .map_err(|_| {
                SdkError::InvalidRequest("authorization_signature must be base58".to_owned())
            })?;
        if signature.len() != 64
            || bs58::encode(&signature).into_string() != request.authorization_signature.trim()
        {
            return Err(SdkError::InvalidRequest(
                "authorization_signature must be a canonical Ed25519 signature".to_owned(),
            ));
        }
        let request = ExecutionPrepareRequest {
            challenge_id: request.challenge_id,
            authorization_signature: bs58::encode(signature).into_string(),
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

    /// Exchange a detached external authorization signature for a backend-
    /// partially-signed v0 transaction.
    pub async fn order_prepare(
        &self,
        market_id: &str,
        request: PlatformOrderPrepareRequest,
    ) -> Result<PlatformOrderPrepareResponse, SdkError> {
        let market_id = validate_platform_market_id(market_id)?;
        if !valid_handle(&request.challenge_id, "oc_") {
            return Err(SdkError::InvalidRequest(
                "order challenge_id is invalid".to_owned(),
            ));
        }
        let signature =
            canonical_signature(&request.authorization_signature, "authorization_signature")?;
        let prepared: PlatformOrderPrepareResponse = self
            .post(
                &format!("v2/markets/{market_id}/orders/prepare"),
                &PlatformOrderPrepareRequest {
                    challenge_id: request.challenge_id,
                    authorization_signature: signature,
                },
            )
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
        Ok(prepared)
    }

    /// Submit an externally signed order-control transaction. The same
    /// control ID and idempotency key return the same receipt.
    pub async fn order_submit(
        &self,
        market_id: &str,
        request: PlatformOrderSubmitRequest,
    ) -> Result<PlatformOrderSubmitResponse, SdkError> {
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

    /// Execute one resting-order operation while all private keys and signing
    /// policy remain in the caller's signer adapter. Authorization bytes are
    /// parsed before message signing, and the mandatory verifier runs before
    /// the transaction signature is requested.
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
        let challenge = self.order_challenge(&market_id, request.clone()).await?;
        if challenge.action != order_request_action(&request) {
            return Err(SdkError::InvalidResponse(
                "order challenge action changed".to_owned(),
            ));
        }
        let authorization = validate_order_authorization(&challenge, &request)?;
        let signature = signer
            .sign_message(&authorization.bytes)
            .await
            .map_err(SdkError::Signer)?;
        if signature.len() != 64 {
            return Err(SdkError::InvalidResponse(
                "order authorization signature must contain 64 bytes".to_owned(),
            ));
        }
        let prepared = self
            .order_prepare(
                &market_id,
                PlatformOrderPrepareRequest {
                    challenge_id: challenge.challenge_id.clone(),
                    authorization_signature: bs58::encode(signature).into_string(),
                },
            )
            .await?;
        validate_order_prepare_binding(&prepared, &challenge, &authorization)?;
        verifier
            .verify(&OrderVerificationContext {
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
        let signed_transaction =
            canonical_base64(&signed_transaction, "signed_transaction_base64")?;
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

fn validate_platform_market_id(value: &str) -> Result<String, SdkError> {
    let value = value.trim();
    if !valid_handle(value, "market_") {
        return Err(SdkError::InvalidRequest(
            "market_id must be an opaque Strata market ID".to_owned(),
        ));
    }
    Ok(value.to_owned())
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
                account_sequence: canonical_request_atoms(
                    &account_sequence,
                    "account_sequence",
                    true,
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
                account_sequence: canonical_request_atoms(
                    &account_sequence,
                    "account_sequence",
                    true,
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
                account_sequence: canonical_request_atoms(
                    &account_sequence,
                    "account_sequence",
                    true,
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

struct OrderAuthorization {
    bytes: Vec<u8>,
    recent_blockhash: String,
    last_valid_block_height: u64,
}

#[allow(clippy::too_many_arguments)]
fn validate_order_place_authorization(
    bytes: &[u8],
    cursor: &mut usize,
    challenge: &PlatformOrderChallengeResponse,
    account_sequence: &str,
    client_order_id: &str,
    side: PlatformTradeSide,
    order_type: PlatformOrderType,
    limit_price_atoms: &str,
    size_atoms: &str,
) -> Result<String, SdkError> {
    take_u64_eq(
        bytes,
        cursor,
        parse_request_u64(account_sequence, "account_sequence")?,
        "order account sequence",
    )?;
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

fn validate_order_authorization(
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
            take_u64_eq(
                &bytes,
                &mut cursor,
                parse_request_u64(account_sequence, "account_sequence")?,
                "order account sequence",
            )?;
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
                account_sequence,
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
                        account_sequence,
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
                            account_sequence,
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

fn validate_order_prepare_binding(
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

fn opaque_order_id(market_id: &str, order: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"strata-sdk-product:v1\0");
    digest.update(b"order");
    digest.update([0]);
    digest.update(market_id.as_bytes());
    digest.update(b":");
    digest.update(bs58::encode(order).into_string().as_bytes());
    format!("order_{}", hex::encode(&digest.finalize()[..16]))
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
            "action-graph" => strata_public_contract::contract_fixtures::ACTION_GRAPH,
            "markets" => strata_public_contract::contract_fixtures::MARKETS,
            "quote" => strata_public_contract::contract_fixtures::QUOTE,
            "capabilities" => strata_public_contract::contract_fixtures::CAPABILITIES,
            "order-challenge" => strata_public_contract::platform::PLATFORM_ORDER_CHALLENGE_FIXTURE,
            "order-prepare" => strata_public_contract::platform::PLATFORM_ORDER_PREPARE_FIXTURE,
            "order-submit" => strata_public_contract::platform::PLATFORM_ORDER_SUBMIT_FIXTURE,
            "order-status" => strata_public_contract::platform::PLATFORM_ORDER_STATUS_FIXTURE,
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

        let graph = client.action_graph().await.unwrap();
        assert_eq!(graph.entry_node, "discover_capabilities");
        assert_eq!(graph.authority.permission_source, "external_agent_owner");

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
        let challenge = client
            .order_challenge(
                market_id,
                PlatformOrderChallengeRequest::Place {
                    owner_wallet,
                    session_public_key,
                    account_sequence: "7".to_owned(),
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
                PlatformOrderPrepareRequest {
                    challenge_id: challenge.challenge_id,
                    authorization_signature,
                },
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
            account_sequence: "7".to_owned(),
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
                    account_sequence: "8".to_owned(),
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
