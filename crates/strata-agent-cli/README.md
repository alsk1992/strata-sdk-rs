# `strata-agent`

Live Strata markets and Sonar quotes in a native terminal command.

## Install

```sh
cargo install strata-agent-cli
```

## Go from market to quote

```sh
strata-agent markets

strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000
```

The quote includes expected output, consumed input, fees, minimum output, price
impact, reference price, expiry, and the `Sonar` provider label.

## Commands

| Command | Use it to |
| --- | --- |
| `strata-agent capabilities` | Inspect features currently available from Strata |
| `strata-agent markets` | List markets ready for a Sonar quote |
| `strata-agent markets --all` | Include markets that are not currently quote-ready |
| `strata-agent quote …` | Request a quote for a market, side, and amount |

Human-readable output is the default. Add `--json` anywhere for stable
machine-readable output:

```sh
strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000 \
  --json
```

Token amounts are exact atomic-unit strings. The production Strata API is the
default; controlled environments can set `STRATA_API_BASE`.

Quotes default to zero execution tolerance. Set `--slippage-bps` explicitly
only when willing to accept a lower `minimum_output_atoms`; price impact is a
separate measure of the depth consumed by the quote.

`0.1.x` is read-only and does not require wallet or private-key material.

See the [workspace README](https://github.com/alsk1992/strata-sdk-rs) for the
complete guide.
