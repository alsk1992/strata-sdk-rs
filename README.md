# `strata-sdk`

The official async Rust client for live Strata markets and Sonar quotes.

The official hosted API currently has market, exact-output, and asset-to-asset
Sonar quotes enabled. The SDK still checks the live capability catalog before
each gated operation; that is a runtime safety check, not an inactive-feature
notice.

## Install

```toml
[dependencies]
strata-public-contract = "0.1"
strata-sdk = "0.1"
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
