# Strata Public Contract

Shared Rust types for Strata market discovery and Sonar quotes.

Most applications should depend on [`strata-sdk`](https://crates.io/crates/strata-sdk),
which includes the HTTP client. Use this crate directly when you need Strata's
request and response models without a network client.

Token amounts are represented as unsigned decimal strings in atomic units so
they remain exact across JSON and language boundaries.

Enable the optional `fixtures` feature to use the canonical JSON examples shared
by the official Strata SDK test suites:

```toml
[dev-dependencies]
strata-public-contract = { version = "0.1", features = ["fixtures"] }
```

See the [repository README](https://github.com/alsk1992/strata-sdk-rs) for full
documentation.
