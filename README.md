# Strata Rust SDK

Official native Rust bindings and terminal access for Strata's versioned public
contract.

This workspace publishes three crates:

- [`strata-public-contract`](crates/strata-public-contract) — strict public data
  types and compatibility fixtures.
- [`strata-sdk`](crates/strata-sdk) — async client with fail-closed response
  validation.
- [`strata-agent-cli`](crates/strata-agent-cli) — the read-only `strata-agent`
  terminal command.

Sonar is Strata's unified liquidity and matching system. A Sonar quote considers
the complete eligible market and returns one composition-opaque economic result.
The public crates do not expose private routing, venue selection, or matching
internals.

## SDK quick start

```toml
[dependencies]
strata-public-contract = "0.1"
strata-sdk = "0.1"
```

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

## Terminal quick start

```sh
cargo install strata-agent-cli
strata-agent capabilities --json
strata-agent markets --json
strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000 \
  --slippage-bps 50 \
  --json
```

## Contract guarantees

- Token amounts cross the public boundary as unsigned base-10 atomic strings.
- Unknown fields and unsupported contract versions fail closed.
- Quotes are rebound to the requested market, side, and input amount.
- Expiry, minimum output, fee labels, and core economic invariants are checked.
- Product capability policy is discovered from Strata instead of hard-coded.

Version `0.1.x` is read-only. It cannot prepare, sign, or submit transactions
and never accepts wallet, keypair, or session-key material.

Public product documentation lives at
[stratabook.app/docs](https://stratabook.app/docs/hello-agents). Security issues
should be reported privately as described in [SECURITY.md](SECURITY.md).
