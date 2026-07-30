# `strata-agent-cli`

Read-only terminal access to Strata and Sonar.

```sh
cargo install strata-agent-cli

strata-agent capabilities --json
strata-agent markets --json
strata-agent quote \
  --market SOL/USDC \
  --side sell \
  --amount-atoms 10000000 \
  --json
```

The command discovers the live public capability policy and validates responses
through `strata-sdk`. It has no wallet, keypair, transaction preparation,
signing, submission, RPC, or administrative flags.
