# `strata-public-contract`

Shared Rust models for Strata markets, capabilities, and Sonar quotes.

Most applications should start with
[`strata-sdk`](https://crates.io/crates/strata-sdk), which includes these types
and the async HTTP client. Use this crate directly when you need the public
models without a network client.

## Install

```toml
[dependencies]
strata-public-contract = "0.1"
```

## What it provides

| Model | Represents |
| --- | --- |
| `CapabilityCatalog` | Features currently available from Strata |
| `MarketsResponse` | Markets, token decimals, and Sonar quote readiness |
| `QuoteRequest` | Market, side, exact input amount, and slippage |
| `QuoteResponse` | Output, fees, minimum output, price impact, and expiry |
| `ErrorResponse` | Stable public error details |

Token amounts are unsigned decimal strings in atomic units so values stay exact
across Rust, JSON, JavaScript, and terminal boundaries.

## Versioned fixtures

Enable the optional `fixtures` feature to consume the canonical JSON examples
used across the official Strata SDKs:

```toml
[dev-dependencies]
strata-public-contract = { version = "0.1", features = ["fixtures"] }
```

See the [workspace README](https://github.com/alsk1992/strata-sdk-rs) for the
complete guide.
