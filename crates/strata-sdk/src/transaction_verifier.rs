//! Built-in transaction verification for one-signature order control.
//!
//! With the direct prepare path the session's signature over the returned
//! transaction is the whole authorization, so before signing the SDK checks
//! that the transaction is exactly the requested operation and nothing more:
//!
//! - the session key co-signs and never pays (it is not the fee payer);
//! - the owner wallet is not asked to sign;
//! - the session key signs only delegated instructions of one program (never a
//!   system, token, or other well-known program instruction);
//! - for resting orders, every delegated place/cancel is decoded and matched
//!   against the requested sides, prices, sizes, order types, order IDs, and the
//!   market — nothing added, nothing changed, nothing missing.
//!
//! TWAP and immediate execution get the same structural checks; their inner
//! economics are bound server-side by the echoed prepare fields the client
//! already checks. Applications with stricter policies keep supplying their
//! own [`OrderVerifier`], [`TwapVerifier`], or [`ExecutionVerifier`].
//!
//! The decoder is hand-written for base64 legacy and v0 Solana transactions
//! (no RPC, no Solana crates): compact-u16 lengths, message header, static
//! keys, blockhash, instructions, and the v0 address-table-lookup section.

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use base64::Engine as _;

use crate::{
    opaque_market_id, opaque_order_id, ExecutionVerificationContext, ExecutionVerifier,
    OrderVerificationContext, OrderVerifier, PlatformOrderBatchOperation,
    PlatformOrderChallengeRequest, PlatformOrderType, PlatformTradeSide, TwapVerificationContext,
    TwapVerifier,
};

/// Programs a Vault session key must never sign for directly.
const WELL_KNOWN_PROGRAMS: [&str; 10] = [
    "11111111111111111111111111111111",             // system
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // token
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // token-2022
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", // associated token
    "ComputeBudget111111111111111111111111111111",  // compute budget
    "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",  // memo
    "Stake11111111111111111111111111111111111111",  // stake
    "Vote111111111111111111111111111111111111111",  // vote
    "AddressLookupTab1e1111111111111111111111111",  // address lookup tables
    "BPFLoaderUpgradeab1e11111111111111111111111",  // upgradeable loader
];

/// Delegated-instruction envelope tag (`execute_with_delegate`).
const DELEGATED_ENVELOPE_TAG: u8 = 3;
/// Inner instruction tags a delegated order-control transaction may carry.
const INNER_TAG_BALANCE: u8 = 1;
const INNER_TAG_CANCEL_ORDER: u8 = 4;
const INNER_TAG_PLACE_ORDER: u8 = 33;
const INNER_TAG_MARKET_ACCOUNT: u8 = 34;
const INNER_TAG_TWAP_CANCEL: u8 = 36;
const INNER_TAG_TWAP_POST: u8 = 38;

/// Message version of a decoded transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransactionVersion {
    Legacy,
    V0,
}

/// One compiled instruction exactly as it appears in the message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedInstruction {
    pub program_id_index: u8,
    pub account_indexes: Vec<u8>,
    pub data: Vec<u8>,
}

/// A decoded legacy or v0 transaction envelope. Address-table lookups are
/// counted but never resolved: verification only reasons about static keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedTransaction {
    pub version: TransactionVersion,
    pub signature_count: usize,
    pub num_required_signatures: u8,
    pub num_readonly_signed: u8,
    pub num_readonly_unsigned: u8,
    /// Base58 static account keys, in message order.
    pub static_account_keys: Vec<String>,
    pub recent_blockhash: String,
    pub instructions: Vec<DecodedInstruction>,
    pub address_table_lookup_count: usize,
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn u8(&mut self) -> Result<u8, String> {
        let byte = *self
            .bytes
            .get(self.offset)
            .ok_or_else(|| "transaction is truncated".to_owned())?;
        self.offset += 1;
        Ok(byte)
    }

    fn compact_u16(&mut self) -> Result<usize, String> {
        let mut value = 0usize;
        let mut shift = 0u32;
        for _ in 0..3 {
            let byte = self.u8()?;
            value |= usize::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
        Err("transaction length prefix is invalid".to_owned())
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let end = self
            .offset
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| "transaction is truncated".to_owned())?;
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }

    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

/// Decode a base64 legacy or v0 transaction without any RPC.
pub fn decode_transaction(transaction_base64: &str) -> Result<DecodedTransaction, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(transaction_base64.trim())
        .map_err(|_| "invalid base64 payload".to_owned())?;
    let mut reader = Reader::new(&bytes);
    let signature_count = reader.compact_u16()?;
    reader.take(signature_count.saturating_mul(64))?;
    let mut first = reader.u8()?;
    let mut version = TransactionVersion::Legacy;
    if first & 0x80 != 0 {
        if first & 0x7f != 0 {
            return Err("unsupported transaction version".to_owned());
        }
        version = TransactionVersion::V0;
        first = reader.u8()?;
    }
    let num_required_signatures = first;
    let num_readonly_signed = reader.u8()?;
    let num_readonly_unsigned = reader.u8()?;
    let key_count = reader.compact_u16()?;
    let mut static_account_keys = Vec::with_capacity(key_count);
    for _ in 0..key_count {
        static_account_keys.push(bs58::encode(reader.take(32)?).into_string());
    }
    let recent_blockhash = bs58::encode(reader.take(32)?).into_string();
    let instruction_count = reader.compact_u16()?;
    let mut instructions = Vec::with_capacity(instruction_count);
    for _ in 0..instruction_count {
        let program_id_index = reader.u8()?;
        let account_count = reader.compact_u16()?;
        let account_indexes = reader.take(account_count)?.to_vec();
        let data_length = reader.compact_u16()?;
        let data = reader.take(data_length)?.to_vec();
        instructions.push(DecodedInstruction {
            program_id_index,
            account_indexes,
            data,
        });
    }
    let mut address_table_lookup_count = 0;
    if version == TransactionVersion::V0 {
        address_table_lookup_count = reader.compact_u16()?;
        for _ in 0..address_table_lookup_count {
            reader.take(32)?;
            let writable = reader.compact_u16()?;
            reader.take(writable)?;
            let readonly = reader.compact_u16()?;
            reader.take(readonly)?;
        }
    }
    if !reader.done() {
        return Err("transaction carries trailing bytes".to_owned());
    }
    if signature_count != usize::from(num_required_signatures) || num_required_signatures == 0 {
        return Err("transaction signature layout is invalid".to_owned());
    }
    if static_account_keys.len() < usize::from(num_required_signatures) {
        return Err("transaction signer layout is invalid".to_owned());
    }
    Ok(DecodedTransaction {
        version,
        signature_count,
        num_required_signatures,
        num_readonly_signed,
        num_readonly_unsigned,
        static_account_keys,
        recent_blockhash,
        instructions,
        address_table_lookup_count,
    })
}

struct DelegatedInstruction {
    inner_tag: u8,
    inner: Vec<u8>,
    /// Base58 keys of the inner instruction's accounts, in order (`None` when
    /// an index points outside the static key table).
    inner_accounts: Vec<Option<String>>,
}

struct StructuralOptions {
    allow_address_tables: bool,
    require_envelope: bool,
}

/// The structural invariants every session-signed Strata transaction must
/// hold. Returns the delegated instructions for the caller's own decoding.
fn structural_checks(
    tx: &DecodedTransaction,
    session_public_key: &str,
    owner_wallet: &str,
    recent_blockhash: &str,
    options: StructuralOptions,
) -> Result<Vec<DelegatedInstruction>, String> {
    if tx.recent_blockhash != recent_blockhash {
        return Err("prepared transaction blockhash does not match".to_owned());
    }
    let keys = &tx.static_account_keys;
    let required = usize::from(tx.num_required_signatures);
    let session_index = keys
        .iter()
        .position(|key| key == session_public_key)
        .filter(|index| *index < required)
        .ok_or_else(|| "the session key is not a required signer".to_owned())?;
    if session_index == 0 {
        return Err("the session key must never be the fee payer".to_owned());
    }
    if let Some(owner_index) = keys.iter().position(|key| key == owner_wallet) {
        if owner_index < required {
            return Err("the owner wallet must not be asked to sign".to_owned());
        }
    }
    if !options.allow_address_tables && tx.address_table_lookup_count != 0 {
        return Err("order-control transactions carry no lookup tables".to_owned());
    }
    let session_index_u8 = u8::try_from(session_index)
        .map_err(|_| "transaction signer layout is invalid".to_owned())?;
    let mut envelope_program: Option<&str> = None;
    let mut inner_program: Option<&str> = None;
    let mut delegated = Vec::new();
    for instruction in &tx.instructions {
        if !instruction.account_indexes.contains(&session_index_u8) {
            continue;
        }
        let program = keys
            .get(usize::from(instruction.program_id_index))
            .ok_or_else(|| "instruction program is not static".to_owned())?;
        if WELL_KNOWN_PROGRAMS.contains(&program.as_str()) {
            return Err("the session key must not sign a system or token instruction".to_owned());
        }
        match envelope_program {
            None => envelope_program = Some(program),
            Some(existing) if existing != program => {
                return Err("the session key signs for more than one program".to_owned());
            }
            Some(_) => {}
        }
        if instruction.account_indexes.first() != Some(&session_index_u8) {
            return Err("the session key is not the delegate signer".to_owned());
        }
        if !options.require_envelope {
            continue;
        }
        let data = &instruction.data;
        if data.len() < 14 || data[0] != DELEGATED_ENVELOPE_TAG {
            return Err("the session key signs a non-delegated instruction".to_owned());
        }
        let inner_length = usize::from(data[11]) | (usize::from(data[12]) << 8);
        let inner_end = 14 + inner_length;
        if inner_length == 0 || inner_end > data.len() {
            return Err("delegated instruction is malformed".to_owned());
        }
        let inner = data[14..inner_end].to_vec();
        let inner_program_key = instruction
            .account_indexes
            .get(3)
            .and_then(|index| keys.get(usize::from(*index)))
            .ok_or_else(|| "delegated instruction target is not static".to_owned())?;
        match inner_program {
            None => inner_program = Some(inner_program_key),
            Some(existing) if existing != inner_program_key => {
                return Err("delegated instructions target more than one program".to_owned());
            }
            Some(_) => {}
        }
        delegated.push(DelegatedInstruction {
            inner_tag: inner[0],
            inner,
            inner_accounts: instruction
                .account_indexes
                .iter()
                .skip(6)
                .map(|index| keys.get(usize::from(*index)).cloned())
                .collect(),
        });
    }
    if delegated.is_empty() && options.require_envelope {
        return Err("the transaction carries no delegated instruction".to_owned());
    }
    Ok(delegated)
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, String> {
    let slice = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| "delegated instruction is truncated".to_owned())?;
    let mut array = [0u8; 8];
    array.copy_from_slice(slice);
    Ok(u64::from_le_bytes(array))
}

const fn side_wire(side: PlatformTradeSide) -> u8 {
    match side {
        PlatformTradeSide::Buy => 0,
        PlatformTradeSide::Sell => 1,
    }
}

fn order_type_wire(order_type: PlatformOrderType) -> Result<u8, String> {
    match order_type {
        PlatformOrderType::GoodUntilCancelled => Ok(0),
        PlatformOrderType::PostOnly => Ok(3),
        PlatformOrderType::ImmediateOrCancel | PlatformOrderType::FillOrKill => {
            Err("order type is not a resting order".to_owned())
        }
    }
}

fn atoms(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{field} must be an unsigned atomic decimal string"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field} exceeds u64"))
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ExpectedPlace {
    side: u8,
    order_type: u8,
    price: u64,
    size: u64,
}

impl ExpectedPlace {
    fn from_request(
        side: PlatformTradeSide,
        order_type: PlatformOrderType,
        limit_price_atoms: &str,
        size_atoms: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            side: side_wire(side),
            order_type: order_type_wire(order_type)?,
            price: atoms(limit_price_atoms, "limit_price_atoms")?,
            size: atoms(size_atoms, "size_atoms")?,
        })
    }
}

enum ExpectedCancels {
    /// Order IDs that must be cancelled.
    Ids(Vec<String>),
    /// `cancel_all`: at least one cancellation, identities chosen server-side.
    All,
}

struct ExpectedOrderIntent {
    places: Vec<ExpectedPlace>,
    cancels: ExpectedCancels,
}

fn expected_order_intent(
    operation: &PlatformOrderChallengeRequest,
) -> Result<ExpectedOrderIntent, String> {
    Ok(match operation {
        PlatformOrderChallengeRequest::Place {
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
            ..
        } => ExpectedOrderIntent {
            places: vec![ExpectedPlace::from_request(
                *side,
                *order_type,
                limit_price_atoms,
                size_atoms,
            )?],
            cancels: ExpectedCancels::Ids(Vec::new()),
        },
        PlatformOrderChallengeRequest::Cancel { order_id, .. } => ExpectedOrderIntent {
            places: Vec::new(),
            cancels: ExpectedCancels::Ids(vec![order_id.clone()]),
        },
        PlatformOrderChallengeRequest::CancelAll { .. } => ExpectedOrderIntent {
            places: Vec::new(),
            cancels: ExpectedCancels::All,
        },
        PlatformOrderChallengeRequest::Replace {
            order_id,
            side,
            order_type,
            limit_price_atoms,
            size_atoms,
            ..
        } => ExpectedOrderIntent {
            places: vec![ExpectedPlace::from_request(
                *side,
                *order_type,
                limit_price_atoms,
                size_atoms,
            )?],
            cancels: ExpectedCancels::Ids(vec![order_id.clone()]),
        },
        PlatformOrderChallengeRequest::Batch { operations, .. } => {
            let mut places = Vec::new();
            let mut cancels = Vec::new();
            for item in operations {
                match item {
                    PlatformOrderBatchOperation::Place {
                        side,
                        order_type,
                        limit_price_atoms,
                        size_atoms,
                        ..
                    } => places.push(ExpectedPlace::from_request(
                        *side,
                        *order_type,
                        limit_price_atoms,
                        size_atoms,
                    )?),
                    PlatformOrderBatchOperation::Cancel { order_id } => {
                        cancels.push(order_id.clone());
                    }
                    PlatformOrderBatchOperation::Replace {
                        order_id,
                        side,
                        order_type,
                        limit_price_atoms,
                        size_atoms,
                        ..
                    } => {
                        cancels.push(order_id.clone());
                        places.push(ExpectedPlace::from_request(
                            *side,
                            *order_type,
                            limit_price_atoms,
                            size_atoms,
                        )?);
                    }
                }
            }
            ExpectedOrderIntent {
                places,
                cancels: ExpectedCancels::Ids(cancels),
            }
        }
    })
}

fn same_multiset<T, K, F>(left: &[T], right: &[T], key: F) -> bool
where
    K: std::hash::Hash + Eq,
    F: Fn(&T) -> K,
{
    if left.len() != right.len() {
        return false;
    }
    let mut counts: HashMap<K, usize> = HashMap::new();
    for value in left {
        *counts.entry(key(value)).or_insert(0) += 1;
    }
    for value in right {
        match counts.get_mut(&key(value)) {
            Some(remaining) if *remaining > 0 => *remaining -= 1,
            _ => return false,
        }
    }
    true
}

fn decode_order_key(order: &str) -> Result<Vec<u8>, String> {
    bs58::decode(order)
        .into_vec()
        .map_err(|_| "order account key is not base58".to_owned())
}

/// Deny-by-default verification of a prepared resting-order transaction: it
/// must be exactly the requested operation for this market and session.
pub fn verify_order_transaction(context: &OrderVerificationContext<'_>) -> Result<(), String> {
    let tx = decode_transaction(&context.prepared.transaction_base64)?;
    let delegated = structural_checks(
        &tx,
        context.session_public_key,
        context.owner_wallet,
        &context.prepared.recent_blockhash,
        StructuralOptions {
            allow_address_tables: false,
            require_envelope: true,
        },
    )?;
    struct DecodedPlace {
        place: ExpectedPlace,
        market: String,
        order: String,
    }
    struct DecodedCancel {
        market: String,
        order: String,
    }
    let mut places: Vec<DecodedPlace> = Vec::new();
    let mut cancels: Vec<DecodedCancel> = Vec::new();
    for instruction in &delegated {
        match instruction.inner_tag {
            INNER_TAG_BALANCE | INNER_TAG_MARKET_ACCOUNT => {}
            INNER_TAG_PLACE_ORDER => {
                let inner = &instruction.inner;
                // [tag][side][order_type][0,0][price u64][size u64][expiry u64][bump]
                if inner.len() < 30 {
                    return Err("place instruction is truncated".to_owned());
                }
                let market = instruction.inner_accounts.get(1).cloned().flatten();
                let order = instruction.inner_accounts.get(3).cloned().flatten();
                let (Some(market), Some(order)) = (market, order) else {
                    return Err("place instruction accounts are not static".to_owned());
                };
                places.push(DecodedPlace {
                    place: ExpectedPlace {
                        side: inner[1],
                        order_type: inner[2],
                        price: read_u64(inner, 5)?,
                        size: read_u64(inner, 13)?,
                    },
                    market,
                    order,
                });
            }
            INNER_TAG_CANCEL_ORDER => {
                let market = instruction.inner_accounts.get(1).cloned().flatten();
                let order = instruction.inner_accounts.get(3).cloned().flatten();
                let (Some(market), Some(order)) = (market, order) else {
                    return Err("cancel instruction accounts are not static".to_owned());
                };
                cancels.push(DecodedCancel { market, order });
            }
            other => {
                return Err(format!(
                    "the transaction delegates an unexpected instruction ({other})"
                ));
            }
        }
    }
    // Every touched market must be the requested one.
    let markets: HashSet<&str> = places
        .iter()
        .map(|entry| entry.market.as_str())
        .chain(cancels.iter().map(|entry| entry.market.as_str()))
        .collect();
    for market in markets {
        if opaque_market_id(market) != context.market_id {
            return Err("the transaction touches another market".to_owned());
        }
    }
    let expected = expected_order_intent(context.operation)?;
    let decoded_places: Vec<ExpectedPlace> =
        places.iter().map(|entry| entry.place.clone()).collect();
    if !same_multiset(&decoded_places, &expected.places, |place| place.clone()) {
        return Err("the transaction does not place exactly the requested orders".to_owned());
    }
    let cancelled_ids = cancels
        .iter()
        .map(|entry| {
            Ok(opaque_order_id(
                context.market_id,
                &decode_order_key(&entry.order)?,
            ))
        })
        .collect::<Result<Vec<String>, String>>()?;
    match &expected.cancels {
        ExpectedCancels::All => {
            if cancelled_ids.is_empty() {
                return Err("cancel_all prepared no cancellation".to_owned());
            }
        }
        ExpectedCancels::Ids(ids) => {
            if !same_multiset(&cancelled_ids, ids, |id| id.clone()) {
                return Err(
                    "the transaction does not cancel exactly the requested orders".to_owned(),
                );
            }
        }
    }
    // The echoed order IDs must be exactly the orders this transaction touches.
    let placed_ids = places
        .iter()
        .map(|entry| {
            Ok(opaque_order_id(
                context.market_id,
                &decode_order_key(&entry.order)?,
            ))
        })
        .collect::<Result<Vec<String>, String>>()?;
    let touched: Vec<String> = cancelled_ids.into_iter().chain(placed_ids).collect();
    if !same_multiset(&touched, &context.prepared.order_ids, |id| id.clone()) {
        return Err("prepared order IDs do not match the transaction".to_owned());
    }
    Ok(())
}

/// Structural verification of a prepared TWAP-control transaction: session
/// co-signs only delegated TWAP instructions and never pays; the bound TWAP
/// economics are checked against the echoed prepare fields by the client.
pub fn verify_twap_transaction(context: &TwapVerificationContext<'_>) -> Result<(), String> {
    let tx = decode_transaction(&context.prepared.transaction_base64)?;
    let delegated = structural_checks(
        &tx,
        context.session_public_key,
        context.owner_wallet,
        &context.prepared.recent_blockhash,
        StructuralOptions {
            allow_address_tables: false,
            require_envelope: true,
        },
    )?;
    for instruction in &delegated {
        if !matches!(
            instruction.inner_tag,
            INNER_TAG_BALANCE
                | INNER_TAG_MARKET_ACCOUNT
                | INNER_TAG_TWAP_POST
                | INNER_TAG_TWAP_CANCEL
        ) {
            return Err(format!(
                "the transaction delegates an unexpected instruction ({})",
                instruction.inner_tag
            ));
        }
    }
    Ok(())
}

/// Structural verification of a prepared immediate execution: the session
/// co-signs only Vault-delegated instructions of one program and never pays.
/// The bound quote economics are checked against the echoed prepare fields.
pub fn verify_execution_transaction(
    context: &ExecutionVerificationContext<'_>,
) -> Result<(), String> {
    let tx = decode_transaction(&context.prepared.transaction_base64)?;
    structural_checks(
        &tx,
        context.session_public_key,
        context.owner_wallet,
        &context.prepared.recent_blockhash,
        StructuralOptions {
            allow_address_tables: true,
            require_envelope: false,
        },
    )?;
    Ok(())
}

/// The SDK's built-in verifier: [`verify_order_transaction`],
/// [`verify_twap_transaction`], and [`verify_execution_transaction`] behind
/// the [`OrderVerifier`], [`TwapVerifier`], and [`ExecutionVerifier`] traits.
/// Pass `&DefaultTransactionVerifier` to the one-call `execute_*` helpers
/// unless the application enforces a stricter policy of its own.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DefaultTransactionVerifier;

#[async_trait]
impl OrderVerifier for DefaultTransactionVerifier {
    async fn verify(&self, context: &OrderVerificationContext<'_>) -> Result<(), String> {
        verify_order_transaction(context)
    }
}

#[async_trait]
impl TwapVerifier for DefaultTransactionVerifier {
    async fn verify(&self, context: &TwapVerificationContext<'_>) -> Result<(), String> {
        verify_twap_transaction(context)
    }
}

#[async_trait]
impl ExecutionVerifier for DefaultTransactionVerifier {
    async fn verify(&self, context: &ExecutionVerificationContext<'_>) -> Result<(), String> {
        verify_execution_transaction(context)
    }
}

/// Synthetic delegated-place transactions shared by the verifier unit tests
/// and the one-signature client tests.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    pub(crate) const OWNER_WALLET: &str = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
    pub(crate) const SESSION_PUBLIC_KEY: &str = "9Uu7cLBgfMk233BAjMvTS8XJy6KbZK7oQ7NXuCTi3Fg2";
    pub(crate) const FEE_PAYER: [u8; 32] = [1; 32];
    pub(crate) const MARKET_PDA: [u8; 32] = [2; 32];
    pub(crate) const ORDER_PDA: [u8; 32] = [3; 32];
    pub(crate) const RECENT_BLOCKHASH: [u8; 32] = [5; 32];
    const VAULT_PROGRAM: [u8; 32] = [11; 32];
    const STRATA_PROGRAM: [u8; 32] = [12; 32];
    const VAULT_PDA: [u8; 32] = [13; 32];
    const DELEGATE_PDA: [u8; 32] = [14; 32];
    const USER_ACCOUNT: [u8; 32] = [15; 32];
    const RENT_BANK: [u8; 32] = [16; 32];
    pub(crate) const PLACE_PRICE: u64 = 150_000_000;
    pub(crate) const PLACE_SIZE: u64 = 1_000_000_000;

    #[derive(Clone, Copy, Debug, Default)]
    pub(crate) struct PlaceTransactionOptions {
        /// Wire side (`0` buy, `1` sell).
        pub(crate) side: u8,
        /// Put the session key in the fee-payer slot.
        pub(crate) session_pays: bool,
        /// Append a session-signed system transfer.
        pub(crate) extra_system_transfer: bool,
        /// Place on another market key.
        pub(crate) market: Option<[u8; 32]>,
    }

    pub(crate) fn key(value: &str) -> [u8; 32] {
        bs58::decode(value).into_vec().unwrap().try_into().unwrap()
    }

    pub(crate) fn market_id() -> String {
        opaque_market_id(&bs58::encode(MARKET_PDA).into_string())
    }

    pub(crate) fn order_id() -> String {
        opaque_order_id(&market_id(), &ORDER_PDA)
    }

    pub(crate) fn recent_blockhash() -> String {
        bs58::encode(RECENT_BLOCKHASH).into_string()
    }

    fn compact(mut value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    fn envelope(inner: &[u8]) -> Vec<u8> {
        let mut data = vec![DELEGATED_ENVELOPE_TAG];
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&(inner.len() as u16).to_le_bytes());
        data.push(6);
        data.extend_from_slice(inner);
        data.extend_from_slice(&[2; 6]);
        data
    }

    fn place_inner(side: u8, order_type: u8, price: u64, size: u64) -> Vec<u8> {
        let mut inner = vec![INNER_TAG_PLACE_ORDER, side, order_type, 0, 0];
        inner.extend_from_slice(&price.to_le_bytes());
        inner.extend_from_slice(&size.to_le_bytes());
        inner.extend_from_slice(&0u64.to_le_bytes());
        inner.push(255);
        inner
    }

    fn instruction(program: u8, accounts: &[u8], data: &[u8]) -> Vec<u8> {
        let mut out = vec![program];
        out.extend(compact(accounts.len()));
        out.extend_from_slice(accounts);
        out.extend(compact(data.len()));
        out.extend_from_slice(data);
        out
    }

    /// A v0 transaction: fee payer + session sign; one compute-budget
    /// instruction (fee payer only) and one delegated place through the Vault
    /// program targeting the Strata program.
    pub(crate) fn place_transaction(options: PlaceTransactionOptions) -> String {
        let session = key(SESSION_PUBLIC_KEY);
        let owner = key(OWNER_WALLET);
        let system = key("11111111111111111111111111111111");
        let compute_budget = key("ComputeBudget111111111111111111111111111111");
        let mut keys: Vec<[u8; 32]> = if options.session_pays {
            vec![session, FEE_PAYER]
        } else {
            vec![FEE_PAYER, session]
        };
        keys.extend([
            VAULT_PDA,
            DELEGATE_PDA,
            USER_ACCOUNT,
            ORDER_PDA,
            RENT_BANK,
            MARKET_PDA,
            owner,
            VAULT_PROGRAM,
            STRATA_PROGRAM,
            system,
            compute_budget,
        ]);
        if let Some(market) = options.market {
            keys.push(market);
        }
        let at = |wanted: [u8; 32]| -> u8 {
            keys.iter()
                .position(|candidate| *candidate == wanted)
                .unwrap() as u8
        };
        let mut instructions = Vec::new();
        instructions.push(instruction(at(compute_budget), &[], &[2, 0, 0, 0, 0]));
        let market = options.market.unwrap_or(MARKET_PDA);
        // Delegated place: [session, vault, delegate, strataProgram, owner,
        // feePayer, inner: vault, market, userAccount, order, system, rentBank]
        let place_accounts = [
            at(session),
            at(VAULT_PDA),
            at(DELEGATE_PDA),
            at(STRATA_PROGRAM),
            at(owner),
            at(FEE_PAYER),
            at(VAULT_PDA),
            at(market),
            at(USER_ACCOUNT),
            at(ORDER_PDA),
            at(system),
            at(RENT_BANK),
        ];
        let place_data = envelope(&place_inner(options.side, 3, PLACE_PRICE, PLACE_SIZE));
        instructions.push(instruction(at(VAULT_PROGRAM), &place_accounts, &place_data));
        if options.extra_system_transfer {
            let mut data = vec![2, 0, 0, 0];
            data.extend_from_slice(&1u64.to_le_bytes());
            instructions.push(instruction(
                at(system),
                &[at(session), at(FEE_PAYER)],
                &data,
            ));
        }
        let mut message = vec![0x80, 2, 0, 5];
        message.extend(compact(keys.len()));
        for key in &keys {
            message.extend_from_slice(key);
        }
        message.extend_from_slice(&RECENT_BLOCKHASH);
        message.extend(compact(instructions.len()));
        for instruction in &instructions {
            message.extend_from_slice(instruction);
        }
        message.extend(compact(0));
        let mut wire = compact(2);
        wire.extend_from_slice(&[0u8; 128]);
        wire.extend_from_slice(&message);
        base64::engine::general_purpose::STANDARD.encode(wire)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::{PlatformOrderAction, PlatformOrderPrepareResponse};

    fn prepared(transaction_base64: String) -> PlatformOrderPrepareResponse {
        PlatformOrderPrepareResponse {
            schema_version: 2,
            contract_version: "2.0".to_owned(),
            order_control_id: "or_44444444444444444444444444444444".to_owned(),
            market_id: market_id(),
            action: PlatformOrderAction::Place,
            order_ids: vec![order_id()],
            transaction_base64,
            recent_blockhash: recent_blockhash(),
            last_valid_block_height: 400_000_000,
            expires_at_ms: 1_786_550_460_000,
        }
    }

    fn operation() -> PlatformOrderChallengeRequest {
        PlatformOrderChallengeRequest::Place {
            owner_wallet: OWNER_WALLET.to_owned(),
            session_public_key: SESSION_PUBLIC_KEY.to_owned(),
            account_sequence: None,
            client_order_id: "agent-42".to_owned(),
            side: PlatformTradeSide::Buy,
            order_type: PlatformOrderType::PostOnly,
            limit_price_atoms: PLACE_PRICE.to_string(),
            size_atoms: PLACE_SIZE.to_string(),
        }
    }

    fn verify(options: PlaceTransactionOptions) -> Result<(), String> {
        let prepared = prepared(place_transaction(options));
        let operation = operation();
        let market_id = market_id();
        verify_order_transaction(&OrderVerificationContext {
            challenge: None,
            operation: &operation,
            market_id: &market_id,
            prepared: &prepared,
            owner_wallet: OWNER_WALLET,
            session_public_key: SESSION_PUBLIC_KEY,
        })
    }

    #[test]
    fn decodes_a_v0_transaction_with_static_keys_and_instructions() {
        let decoded =
            decode_transaction(&place_transaction(PlaceTransactionOptions::default())).unwrap();
        assert_eq!(decoded.version, TransactionVersion::V0);
        assert_eq!(decoded.signature_count, 2);
        assert_eq!(decoded.num_required_signatures, 2);
        assert_eq!(decoded.num_readonly_signed, 0);
        assert_eq!(decoded.num_readonly_unsigned, 5);
        assert_eq!(decoded.static_account_keys.len(), 13);
        assert_eq!(decoded.static_account_keys[1], SESSION_PUBLIC_KEY);
        assert_eq!(decoded.recent_blockhash, recent_blockhash());
        assert_eq!(decoded.instructions.len(), 2);
        assert_eq!(decoded.instructions[1].account_indexes.len(), 12);
        assert_eq!(decoded.instructions[1].data[0], DELEGATED_ENVELOPE_TAG);
        assert_eq!(decoded.address_table_lookup_count, 0);
    }

    #[test]
    fn decoder_rejects_truncated_and_trailing_bytes() {
        let raw = base64::engine::general_purpose::STANDARD
            .decode(place_transaction(PlaceTransactionOptions::default()))
            .unwrap();
        let truncated = base64::engine::general_purpose::STANDARD.encode(&raw[..raw.len() - 1]);
        assert_eq!(
            decode_transaction(&truncated).unwrap_err(),
            "transaction is truncated"
        );
        let mut trailing = raw.clone();
        trailing.push(0);
        assert_eq!(
            decode_transaction(&base64::engine::general_purpose::STANDARD.encode(trailing))
                .unwrap_err(),
            "transaction carries trailing bytes"
        );
        assert_eq!(
            decode_transaction("not base64!").unwrap_err(),
            "invalid base64 payload"
        );
    }

    #[test]
    fn built_in_verifier_accepts_exactly_the_requested_place() {
        verify(PlaceTransactionOptions::default()).unwrap();
    }

    #[test]
    fn built_in_verifier_refuses_a_different_side() {
        let error = verify(PlaceTransactionOptions {
            side: 1,
            ..PlaceTransactionOptions::default()
        })
        .unwrap_err();
        assert!(error.contains("exactly the requested orders"), "{error}");
    }

    #[test]
    fn built_in_verifier_refuses_the_session_as_fee_payer() {
        let error = verify(PlaceTransactionOptions {
            session_pays: true,
            ..PlaceTransactionOptions::default()
        })
        .unwrap_err();
        assert!(error.contains("fee payer"), "{error}");
    }

    #[test]
    fn built_in_verifier_refuses_a_session_signed_system_transfer() {
        let error = verify(PlaceTransactionOptions {
            extra_system_transfer: true,
            ..PlaceTransactionOptions::default()
        })
        .unwrap_err();
        assert!(error.contains("system or token instruction"), "{error}");
    }

    #[test]
    fn built_in_verifier_refuses_another_market() {
        let error = verify(PlaceTransactionOptions {
            market: Some([7; 32]),
            ..PlaceTransactionOptions::default()
        })
        .unwrap_err();
        assert!(error.contains("another market"), "{error}");
    }

    #[test]
    fn built_in_verifier_binds_the_echoed_order_ids_and_blockhash() {
        let transaction = place_transaction(PlaceTransactionOptions::default());
        let operation = operation();
        let market_id = market_id();
        let mut wrong_ids = prepared(transaction.clone());
        wrong_ids.order_ids = vec!["order_33333333333333333333333333333333".to_owned()];
        let error = verify_order_transaction(&OrderVerificationContext {
            challenge: None,
            operation: &operation,
            market_id: &market_id,
            prepared: &wrong_ids,
            owner_wallet: OWNER_WALLET,
            session_public_key: SESSION_PUBLIC_KEY,
        })
        .unwrap_err();
        assert!(error.contains("order IDs do not match"), "{error}");
        let mut wrong_blockhash = prepared(transaction);
        wrong_blockhash.recent_blockhash = bs58::encode([9u8; 32]).into_string();
        let error = verify_order_transaction(&OrderVerificationContext {
            challenge: None,
            operation: &operation,
            market_id: &market_id,
            prepared: &wrong_blockhash,
            owner_wallet: OWNER_WALLET,
            session_public_key: SESSION_PUBLIC_KEY,
        })
        .unwrap_err();
        assert!(error.contains("blockhash"), "{error}");
    }

    #[test]
    fn twap_and_execution_verification_are_structural() {
        // A place envelope is not a TWAP instruction: the TWAP verifier rejects
        // it by inner tag while the execution verifier (structure only) accepts
        // the same session-signed envelope.
        let transaction = place_transaction(PlaceTransactionOptions::default());
        let twap_prepared = crate::PlatformTwapPrepareResponse {
            schema_version: 2,
            contract_version: "2.0".to_owned(),
            twap_control_id: "twctl_44444444444444444444444444444444".to_owned(),
            market_id: market_id(),
            action: crate::PlatformTwapControlAction::Place,
            twap_id: "twap_33333333333333333333333333333333".to_owned(),
            transaction_base64: transaction.clone(),
            recent_blockhash: recent_blockhash(),
            last_valid_block_height: 1,
            expires_at_ms: 1,
        };
        let twap_operation = crate::PlatformTwapChallengeRequest::Place {
            owner_wallet: OWNER_WALLET.to_owned(),
            session_public_key: SESSION_PUBLIC_KEY.to_owned(),
            side: PlatformTradeSide::Buy,
            total_size_atoms: "10".to_owned(),
            slices_total: 2,
            maximum_tolerance_bps: 1,
            interval_slots: 25,
            limit_price_atoms: "1".to_owned(),
        };
        let market_id = market_id();
        let error = verify_twap_transaction(&TwapVerificationContext {
            challenge: None,
            operation: &twap_operation,
            market_id: &market_id,
            prepared: &twap_prepared,
            owner_wallet: OWNER_WALLET,
            session_public_key: SESSION_PUBLIC_KEY,
        })
        .unwrap_err();
        assert!(error.contains("unexpected instruction (33)"), "{error}");

        let quote: crate::QuoteResponse =
            serde_json::from_str(strata_public_contract::contract_fixtures::QUOTE).unwrap();
        let execution_prepared = crate::ExecutionPrepareResponse {
            schema_version: 1,
            contract_version: "1.1".to_owned(),
            execution_id: "se_0123456789abcdef0123456789abcdef".to_owned(),
            quote_id: quote.quote_id.clone(),
            market_id: quote.market_id.clone(),
            side: quote.side,
            amount_in_atoms: quote.amount_in_atoms.clone(),
            minimum_output_atoms: quote.minimum_output_atoms.clone(),
            transaction_base64: transaction,
            recent_blockhash: recent_blockhash(),
            last_valid_block_height: 1,
            expires_at_ms: 1,
        };
        verify_execution_transaction(&ExecutionVerificationContext {
            quote: &quote,
            challenge: None,
            prepared: &execution_prepared,
            owner_wallet: OWNER_WALLET,
            session_public_key: SESSION_PUBLIC_KEY,
        })
        .unwrap();
    }
}
