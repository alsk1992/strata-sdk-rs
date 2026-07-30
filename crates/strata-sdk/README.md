# Strata SDK

The official Rust client for Strata markets and Sonar quotes.

## Quick start

```rust
use strata_public_contract::{QuoteRequest, QuoteSide};
use strata_sdk::StrataClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let strata = StrataClient::production()?;
let quote = strata
    .quote(QuoteRequest {
        market_id: "SOL/USDC".into(),
        side: QuoteSide::Sell,
        amount_in_atoms: "10000000".into(),
        slippage_bps: 50,
    })
    .await?;

println!("output: {}", quote.amount_out_atoms);
println!("minimum: {}", quote.minimum_output_atoms);
# Ok(())
# }
```

Sonar is Strata's unified liquidity and matching system. A quote includes
expected output, consumed input, fees by token, minimum output, price impact,
and expiry.

All token amounts use exact decimal strings in atomic units. Quotes are
short-lived, so request a new quote after expiry and always respect
`minimum_output_atoms`.

## Terminal client

Install the companion CLI:

```sh
cargo install strata-agent-cli
```

```sh
strata-agent markets
strata-agent quote --market SOL/USDC --side sell --amount-atoms 10000000
```

Add `--json` for machine-readable output.

The `0.1.x` release supports market discovery and read-only Sonar quotes. It
does not prepare, sign, or submit transactions.

See the [repository README](https://github.com/alsk1992/strata-sdk-rs) for full
documentation.
