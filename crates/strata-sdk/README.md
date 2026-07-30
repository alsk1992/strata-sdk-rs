# `strata-sdk`

Strict Rust bindings for Strata's versioned public agent contract.

```rust
use strata_public_contract::{QuoteRequest, QuoteSide};
use strata_sdk::StrataClient;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = StrataClient::production()?;
let quote = client
    .quote(QuoteRequest {
        market_id: "SOL/USDC".into(),
        side: QuoteSide::Sell,
        amount_in_atoms: "10000000".into(),
        slippage_bps: 50,
    })
    .await?;
println!("{}", quote.amount_out_atoms);
# Ok(())
# }
```

Atomic money values remain decimal strings at the public boundary. The SDK
strictly validates contract versions, response fields, quote binding, lifetime,
and economic invariants. Input and output fees are labelled separately. The
quote uses Strata's complete eligible market while exposing no Sonar
implementation types.

The separate `strata-agent-cli` crate exposes the same read-only surface:

```sh
strata-agent capabilities
strata-agent markets
strata-agent quote --market SOL/USDC --side sell --amount-atoms 10000000
```

Add `--json` to any command for stable machine-readable output.

Sonar is Strata's unified liquidity and matching system. Its result is
composition-opaque: this crate exposes the public economics, not private
routing, venue selection, or matching internals.
