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
use strata_public_contract::{QuoteRequest, QuoteSide};
use strata_sdk::StrataClient;

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let strata = StrataClient::production()?;

let quote = strata
    .quote(QuoteRequest {
        market_id: "SOL/USDC".into(),
        side: QuoteSide::Sell,
        amount_in_atoms: "10000000".into(),
        slippage_bps: 50,
    })
    .await?;

println!("Sonar output: {}", quote.amount_out_atoms);
println!("Minimum:      {}", quote.minimum_output_atoms);
println!("Price impact: {}%", quote.price_impact_pct);
# Ok(())
# }
```

Sonar is Strata's unified liquidity and matching system. The response brings
together expected output, consumed input, fees, minimum output, price impact,
and expiry in one typed result.

## Client operations

| Method | Result |
| --- | --- |
| `capabilities()` | Features currently available through the public contract |
| `markets()` | Strata markets, token decimals, and Sonar quote readiness |
| `quote(request)` | A short-lived Sonar economic quote |

Token amounts use unsigned decimal strings in atomic units. The client validates
contract compatibility, quote binding, lifetime, and economic fields before
returning data to the caller.

## Terminal companion

```sh
cargo install strata-agent-cli

strata-agent markets
strata-agent quote --market SOL/USDC --side sell --amount-atoms 10000000
```

Add `--json` for scripts, pipes, and agents.

`0.1.x` covers market discovery and read-only Sonar quotes. It does not prepare,
sign, or submit transactions.

See the [workspace README](https://github.com/alsk1992/strata-sdk-rs) for the
complete guide.
