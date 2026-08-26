<p align="center">
  <a href="https://stratabook.app">
    <img src="https://raw.githubusercontent.com/alsk1992/strata-sdk-rs/main/assets/readme-hero.svg?v=20260826-2" alt="Strata Rust SDK — The deepest book in DeFi" width="100%" />
  </a>
</p>

<p align="center">
  <a href="https://stratabook.app">Trade</a> ·
  <a href="https://stratabook.org/docs/agent-sdks">Docs</a> ·
  <a href="https://crates.io/crates/strata-sdk">crates.io</a> ·
  <a href="https://docs.rs/strata-sdk">docs.rs</a> ·
  <a href="https://github.com/alsk1992/strata-mcp">MCP</a>
</p>

# `strata-sdk`

The official async Rust SDK for Strata. Read live markets and books, request
Sonar quotes, trade through owner-controlled Vault sessions, and manage Intent,
Strand and Current liquidity from one strongly typed client.

| Live surface | What it gives you |
| --- | --- |
| Market data | Markets, books, best prices, marks, candles, trades and streams |
| Sonar | Exact-input, exact-output and asset-to-asset quotes |
| Trading | Quote execution, resting orders and TWAPs through capped Vault sessions |
| Market making | IntentBook seats, Strands, Currents, maker status, fills and reputation |

Each request is checked against Strata's live capability catalog, so a paused
operation stops immediately across the SDK, MCP and hosted API.

## Install

```toml
[dependencies]
strata-public-contract = "0.2"
strata-sdk = "0.2"
```

## Request a Sonar quote

```rust
use strata_public_contract::{QuoteRequest, QuoteSide, DEFAULT_MAXIMUM_TOLERANCE_BPS};
use strata_sdk::StrataClient;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let strata = StrataClient::production()?;

// Exact input: sell 0.01 SOL, receive at least `minimum_output_atoms` USDC.
let quote = strata
    .quote(QuoteRequest {
        market_id: "SOL/USDC".into(),
        side: QuoteSide::Sell,
        amount_in_atoms: Some("10000000".into()),
        amount_out_atoms: None,
        maximum_tolerance_bps: DEFAULT_MAXIMUM_TOLERANCE_BPS,
    })
    .await?;

println!("Sonar output: {}", quote.amount_out_atoms);
println!("Minimum:      {}", quote.minimum_output_atoms);
println!("Price impact: {}%", quote.price_impact_pct);

// Exact output: buy 1 SOL. Strata inverts its best route and returns the USDC
// that delivers it as `amount_in_atoms`; `maximum_tolerance_bps` is the usual
// optional lower floor (zero → exactly 1 SOL or the execution fails closed).
let buy_one_sol = strata
    .quote(QuoteRequest {
        market_id: "SOL/USDC".into(),
        side: QuoteSide::Buy,
        amount_in_atoms: None,
        amount_out_atoms: Some("1000000000".into()),
        maximum_tolerance_bps: DEFAULT_MAXIMUM_TOLERANCE_BPS,
    })
    .await?;
println!("Spend USDC atoms: {}", buy_one_sol.amount_in_atoms);
# Ok(())
# }
```

Sonar returns one Strata quote with expected user-net output, consumed input,
fees, user-net minimum output, price impact, and expiry in one typed result.
Gross route output for an external route-quality comparison is exactly
`amount_out_atoms + output_fee_atoms`; all-in user comparisons use
`amount_out_atoms`.

## Client operations

| Method | Result |
| --- | --- |
| `platform_capabilities()` | Operations currently live through the public 2.0 contract |
| `platform_action_graph()` | Complete customer-safe entity, operation, and workflow graph |
| `platform_status()` | Product-level readiness and live-operation count |
| `platform_assets(...)` / `platform_markets(...)` | Paginated public asset and market identity |
| `platform_book(...)` / `platform_best_bid_ask(...)` | Sequenced Strata book snapshot and best prices |
| `connect_market_data(...)` | Validated Strata book, best-price, trade, and status stream |
| `platform_trades(...)` / `platform_candles(...)` / `platform_mark(...)` | Typed public market history and marks |
| `platform_fees(...)` / `platform_market_status(...)` | Public economics and market readiness |
| `platform_execution_status(...)` | Recoverable immediate-execution receipt |
| `connect_executions(...)` | Sequenced per-market stream for watched execution handles |
| `platform_twaps(...)` | Sanitized progress for wallet-owned TWAPs |
| `connect_twaps(...)` | Sequenced per-market TWAP progress stream with snapshot recovery |
| `platform_account_market(...)` / `platform_account_snapshot(...)` | Externally authorized private orders and fills |
| `connect_account(...)` | Externally authenticated, sequenced private account stream |
| `platform_portfolio(...)` / `platform_account(...)` | The whole account in one public read by wallet: balances (total / available / locked, USD), positions, open orders, recent fills across every live market |
| `platform_vault_*_prepare(...)` | Owner Vault actions prepared with Strata as sponsored fee payer (`preparation_id`, `sponsored`). Onboarding is one signature: `platform_vault_setup_prepare` needs only wallet + session key (policy optional), or name `session_public_key` on the first `platform_vault_deposit_prepare` and the deposit registers it (`registers_session`) |
| `platform_vault_submit(...)` / `platform_vault_submission(...)` | Submit the owner-signed preparation through Strata (idempotent) and read its durable outcome |
| `platform_portfolio_history(...)` | Genuine stored account-equity history |
| `platform_maker_status_for_wallet(...)` (also `platform_maker_status(signer)`) | A maker's products, exposure, health, and kill state — public by wallet address, no signature |
| `platform_maker_reputation_for_wallet(...)` (also `platform_maker_reputation(signer)`) | A maker's reliability, tier, and signed-quote eligibility — public by wallet address |
| `connect_maker_for_wallet(...)` (also `connect_maker(signer)`) | Sequenced maker fill and exposure stream — public by wallet address |
| `platform_maker_start(...)` / `platform_maker_stop(...)` | Label-aware maker quickstart: decimal size, safe defaults, byte-exact verification, external signing, idempotent submission, and chain-derived confirmation |
| `platform_maker_quickstart_prepare(...)` / `platform_maker_submit_prepared(...)` | Split the same verified flow across an external wallet bridge |
| `platform_maker_strand_prepare(...)` / `platform_maker_strand_submit(...)` | Prepare and submit externally signed Strand upsert, recenter, enable/disable, or cancel actions |
| `platform_maker_current_prepare(...)` / `platform_maker_current_submit(...)` | Prepare and submit externally signed Current upsert or cancel actions |
| `platform_maker_intent_prepare(...)` / `platform_maker_intent_submit(...)` | Prepare and submit a sponsored Vault-session post or permanent revoke for an existing curated IntentBook seat |
| `platform_maker_intent_execute(...)` | Verify, session-sign, and submit that exact IntentBook packet in one call |
| `platform_rewards(...)` / `platform_referrals(...)` | Public community and owner-scoped state |
| `platform_bugs(...)` / `platform_bug_submit(...)` | Signed public bug-report workflow |
| `capabilities()` | Features currently available through the public contract |
| `action_graph()` | Live operation topology and external signing boundaries |
| `markets()` | Strata markets, token decimals, and Sonar quote readiness |
| `quote(request)` | A short-lived Sonar economic quote — exact input (`amount_in_atoms`) or exact output (`amount_out_atoms`) |
| `execution_challenge(...)` | Canonical authorization bytes for an external signer |
| `execution_prepare(...)` | Quote-bound partially signed transaction — `ExecutionPrepareRequest::Direct` (the quote binding, one signature) or `::Authorized` (a signed challenge) |
| `execution_submit(...)` | Idempotent submission of an externally signed transaction |
| `execute_quote(...)` | One-signature Vault-session execution when enabled: direct prepare, binding checks, verifier (`DefaultTransactionVerifier` or your own), then the session signs only the transaction |
| `order_challenge(...)` | Canonical authorization bytes for a resting-order action |
| `order_prepare(...)` | Partially signed order-control transaction — `PlatformOrderPrepareRequest::Direct` (the operation itself, one signature) or `::Authorized` (a signed challenge) |
| `order_submit(...)` | Idempotent submission of an externally signed order action |
| `order_status(...)` | Durable recovery of an ambiguous order submission |
| `execute_order(...)` | One-signature place/cancel/replace/batch flow when enabled: direct prepare, binding checks, verifier (`DefaultTransactionVerifier` decodes the transaction and requires exactly this operation), then the session signs only the transaction |
| `twap_challenge(...)` | Canonical authorization bytes for a bounded TWAP action |
| `twap_prepare(...)` | Partially signed TWAP-control transaction — `PlatformTwapPrepareRequest::Direct` (the action itself, one signature) or `::Authorized` (a signed challenge) |
| `twap_submit(...)` | Idempotent submission of an externally signed TWAP action |
| `execute_twap(...)` | One-signature TWAP placement or cancellation when enabled: direct prepare, binding checks, verifier (`DefaultTransactionVerifier` or your own), then the session signs only the transaction |

## Market making: Intent, Strands and Currents

The high-level Rust path takes human inputs and hides product arrays, token
atoms, market IDs, tick math, expiry slots, and confirmation polling:

```rust
use strata_public_contract::PlatformMakerControlProduct;
use strata_sdk::{
    PlatformMakerQuickstartRequest, PlatformMakerQuickstartSide, StrataClient,
};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
# let strata = StrataClient::production()?;
# let maker_signer = todo!("implement MakerTransactionSigner with your wallet or HSM");
let live = strata
    .platform_maker_start(
        &PlatformMakerQuickstartRequest {
            market: "SOL/USDC".into(),
            product: PlatformMakerControlProduct::Current,
            spread_bps: 5,
            size: "0.01 SOL".into(),
            duration: Some("10m".into()),
            levels: None,          // defaults to three
            level_step_bps: None,  // defaults to spread_bps
            side: PlatformMakerQuickstartSide::Both,
            async_only: false,
        },
        &maker_signer,
        None,
    )
    .await?;

println!("confirmed at slot {}", live.maker_status.current_slot);

strata
    .platform_maker_stop(
        "SOL/USDC",
        PlatformMakerControlProduct::Current,
        &maker_signer,
        None,
    )
    .await?;
# Ok(())
# }
```

`MakerTransactionSigner` exposes only a public key and one
`sign_transaction(...)` callback. Before calling it, the SDK decodes the exact
native-v0 Solana transaction and checks its signer, market, action, expiry,
spread, depth, exposure, and every other economic field. After signing it also
proves the message is still byte-identical before submission.
`platform_maker_start` returns only after the chain-derived status matches;
`platform_maker_stop` skips signing when the product is already absent.

Both maker products also expose every low-level control. Request exact
transaction bytes, verify and sign those bytes outside Strata, then submit the
signed transaction with an idempotency key:

```rust
use strata_public_contract::{
    PlatformMakerControlSubmitRequest, PlatformMakerCurrentPrepareRequest,
    PlatformMakerStrandPrepareRequest,
};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
# let strata = strata_sdk::StrataClient::production()?;
# let market_id = "market_33333333333333333333333333333333";
# let maker_wallet = "5Ji61Fbeb22Yntgv1hhHeSSLgdEdZchHeM1Tv1MjGhSL";
let prepared = strata
    .platform_maker_strand_prepare(
        market_id,
        PlatformMakerStrandPrepareRequest::Cancel {
            maker_wallet: maker_wallet.into(),
        },
    )
    .await?;

// Verify `prepared.transaction_base64`, sign it in your wallet or signer, then:
# let signed_transaction_base64 = "AQ==";
let submitted = strata
    .platform_maker_strand_submit(
        market_id,
        PlatformMakerControlSubmitRequest {
            maker_control_id: prepared.maker_control_id,
            signed_transaction_base64: signed_transaction_base64.into(),
            idempotency_key: "strand-cancel-1".into(),
        },
    )
    .await?;

// Current uses the identical prepare/sign/submit boundary.
let _current = strata
    .platform_maker_current_prepare(
        market_id,
        PlatformMakerCurrentPrepareRequest::Cancel {
            maker_wallet: maker_wallet.into(),
        },
    )
    .await?;
# let _ = submitted;
# Ok(())
# }
```

Upserts use `PlatformMakerStrandPrepareRequest::Upsert` or
`PlatformMakerCurrentPrepareRequest::Upsert`. Current bands track the market's
live Strata mark automatically.
Amounts are base-asset atoms,
encoded as unsigned decimal strings. `platform_maker_status_for_wallet(...)`
reports the resulting live controls, remaining exposure, expiry, and health.

Existing curated IntentBook seats use `PlatformMakerIntentPrepareRequest` with
`platform_maker_intent_prepare(...)` / `platform_maker_intent_submit(...)`, or
the one-call `platform_maker_intent_execute(...)`. The built-in
`DefaultTransactionVerifier` binds the Vault, owner, session, market, intent,
account roles, side, price band, and maximum fill before the session signs.
Strata pays the network fee and records it for bounded recovery from a later
deposit. `Revoke` permanently closes that curated seat; it is not a pause.

For maker funding, initialize the market Vault if needed, submit the active
Strand or Current, then use `platform_vault_deposit_prepare(...)` and
`platform_vault_submit(...)`. Available collateral remains in that market while
at least one control is live and returns to the canonical Vault balance after
the final control is disabled, exhausted, expired, or cancelled.

Token amounts use unsigned decimal strings in atomic units. The client validates
contract compatibility, quote binding, lifetime, and economic fields before
returning data to the caller.

The 2.0 methods use stable product identities and the Strata book. They expose
no private implementation details. Signed account reads use Strata's
server time and exact SDK-generated authorization bytes; wallet keys remain in
the agent owner's `AccountSigner` implementation.

`maximum_tolerance_bps` is *yours*: the most you accept below the quoted
output, `0` by default (`DEFAULT_MAXIMUM_TOLERANCE_BPS`), applied in
`minimum_output_atoms` and echoed back on the quote. `price_impact_pct` is
*measured*: how far the quoted fills' average price sits from the best price
before your order. They are unrelated — a quote can show `0` impact with a
25 bps tolerance, or 40 bps of impact with `0` tolerance.

## Terminal companion

```sh
cargo install strata-agent-cli

strata-agent markets
strata-agent quote --market SOL/USDC --side sell --amount-atoms 10000000
```

Add `--json` for scripts, pipes, and agents.

Execution uses external-owner implementations of `SessionSigner` and
`ExecutionVerifier`. `execute_quote(...)` is one signature per action: it
prepares directly from the quote binding (no challenge, no message signature),
validates the quote, minimum output, expiry, prepared response, and receipt,
and calls the verifier before the session adapter can sign the transaction. It
never accepts private key bytes. The agent owner controls permission and signer
policy; MCP exposes the same prepare and submit operations when live
capabilities allow them. The two-step challenge path (`execution_challenge` +
`ExecutionPrepareRequest::Authorized`) remains available, with
`validate_execution_authorization` to check the payload before signing it.

Resting-order actions use the same external-owner boundary. The high-level
`execute_order(...)` helper sends the operation itself to `orders/prepare`
(`PlatformOrderPrepareRequest::Direct`), checks the prepared market, action,
and order IDs against the request, then requires an `OrderVerifier` before
requesting the one transaction signature. `DefaultTransactionVerifier` is the
built-in verifier: it decodes the base64 v0/legacy transaction without any RPC
and refuses to sign unless the session key co-signs without paying fees, the
owner wallet is not asked to sign, the session signs only Vault-delegated
instructions of one program (never a system, token, or other well-known
program), and every delegated place/cancel matches the requested sides, prices,
sizes, order types, order IDs, and market exactly. Supply your own verifier for
stricter policies; the context carries the bound `operation`, `market_id`, the
`prepared` response, and (two-step path only) the `challenge`.

TWAP actions follow the same pattern. `execute_twap(...)` prepares directly
from the action, checks the prepared bindings, then requires a `TwapVerifier`
(`DefaultTransactionVerifier` applies the structural checks) before the session
adapter signs the one transaction.

See the [workspace README](https://github.com/alsk1992/strata-sdk-rs) for the
complete guide.
