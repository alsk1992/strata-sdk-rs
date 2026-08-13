# `strata-sdk`

The official async Rust client for live Strata markets and Sonar quotes.

## Install

```toml
[dependencies]
strata-public-contract = "0.1"
strata-sdk = "0.1"
```

## Request a Sonar quote

```rust
use strata_public_contract::{QuoteRequest, QuoteSide, DEFAULT_SLIPPAGE_BPS};
use strata_sdk::StrataClient;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let strata = StrataClient::production()?;

let quote = strata
    .quote(QuoteRequest {
        market_id: "SOL/USDC".into(),
        side: QuoteSide::Sell,
        amount_in_atoms: "10000000".into(),
        slippage_bps: DEFAULT_SLIPPAGE_BPS,
    })
    .await?;

println!("Sonar output: {}", quote.amount_out_atoms);
println!("Minimum:      {}", quote.minimum_output_atoms);
println!("Price impact: {}%", quote.price_impact_pct);
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
| `capabilities()` | Features currently available through the public contract |
| `action_graph()` | Live operation topology and external signing boundaries |
| `markets()` | Strata markets, token decimals, and Sonar quote readiness |
| `quote(request)` | A short-lived Sonar economic quote |
| `execution_challenge(...)` | Canonical authorization bytes for an external signer |
| `execution_prepare(...)` | Quote-bound partially signed transaction |
| `execution_submit(...)` | Idempotent submission of an externally signed transaction |
| `execute_quote(...)` | Authenticated Vault-session execution when enabled |
| `order_challenge(...)` | Canonical authorization bytes for a resting-order action |
| `order_prepare(...)` | Partially signed place or cancel transaction |
| `order_submit(...)` | Idempotent submission of an externally signed order action |
| `order_status(...)` | Durable recovery of an ambiguous order submission |
| `execute_order(...)` | Verified place or cancel flow when enabled |

Token amounts use unsigned decimal strings in atomic units. The client validates
contract compatibility, quote binding, lifetime, and economic fields before
returning data to the caller.

`DEFAULT_SLIPPAGE_BPS` is `0` for exact read-only quotes. Set a non-zero
execution tolerance explicitly only when your application is willing to accept
less than the quoted output; `minimum_output_atoms` is the resulting floor.

## Terminal companion

```sh
cargo install strata-agent-cli

strata-agent markets
strata-agent quote --market SOL/USDC --side sell --amount-atoms 10000000
```

Add `--json` for scripts, pipes, and agents.

Execution uses external-owner implementations of `SessionSigner` and
`ExecutionVerifier`. The SDK validates the one-time authorization, quote,
minimum output, expiry, blockhash, prepared response, and receipt. It calls the
verifier before the session adapter can sign and never accepts private key
bytes. The agent owner controls permission and signer policy; MCP exposes the
same separate challenge, prepare, and submit operations when live capabilities
allow them.

Resting-order actions use the same external-owner boundary. The high-level
`execute_order(...)` helper validates every signed action field, opaque order
identity, lifetime, and replay value before asking the session to sign. It then
requires an `OrderVerifier` before requesting the transaction signature.

See the [workspace README](https://github.com/alsk1992/strata-sdk-rs) for the
complete guide.
