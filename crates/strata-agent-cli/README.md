# Strata CLI

Explore Strata markets and request Sonar quotes from your terminal.

## Install

```sh
cargo install strata-agent-cli
```

## Quick start

```sh
strata-agent markets

strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000 \
  --slippage-bps 50
```

Commands are human-readable by default. Add `--json` for scripts and agents:

```sh
strata-agent markets --json
strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000 \
  --json
```

Token amounts use exact atomic units. Sonar quotes include expected output,
fees, minimum output, price impact, and expiry.

The `0.1.x` release is read-only and does not require wallet or private-key
material.

See the [repository README](https://github.com/alsk1992/strata-sdk-rs) for full
documentation.
