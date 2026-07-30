# Strata SDK for Rust

Build native Rust applications and terminal workflows with Strata markets and
Sonar quotes.

The SDK provides typed, async access to Strata's public API. Sonar is Strata's
unified liquidity and matching system: one request returns the price, fees,
minimum output, price impact, and expiry for the complete Strata market.

## Quick start

Add the SDK and shared types to your project:

```toml
[dependencies]
strata-public-contract = "0.1"
strata-sdk = "0.1"
```

Request a quote:

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

## Use it from the terminal

Install the `strata-agent` command:

```sh
cargo install strata-agent-cli
```

Explore markets and request a quote:

```sh
strata-agent markets

strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000 \
  --slippage-bps 50
```

Add `--json` to any command for stable machine-readable output.

## Working with quotes

Token amounts use atomic units—the smallest unit of each token—and cross the API
boundary as decimal strings so they remain exact. A quote includes expected
output, consumed input, fees by token, minimum output, price impact, and expiry.

Quotes are short-lived. Request a new quote after expiry and always respect
`minimum_output_atoms`.

## Crates

| Crate | Purpose |
| --- | --- |
| [`strata-sdk`](crates/strata-sdk) | Async client for markets, capabilities, and Sonar quotes |
| [`strata-public-contract`](crates/strata-public-contract) | Shared request and response types |
| [`strata-agent-cli`](crates/strata-agent-cli) | The `strata-agent` terminal command |

## Available today

The `0.1.x` release supports market discovery and read-only Sonar quotes. It
does not prepare, sign, or submit transactions and never needs wallet or
private-key material.

## Documentation and support

- [Agent quick start](https://stratabook.app/docs/hello-agents)
- [SDK documentation](https://stratabook.app/docs/agent-sdks)
- [TypeScript SDK](https://github.com/alsk1992/strata-sdk-ts)
- [Report a bug or request a feature](https://github.com/alsk1992/strata-sdk-rs/issues)
- [Report a security issue](SECURITY.md)

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
