# Releasing

The three crates share one version and are released from the same reviewed
public commit in dependency order: contract, SDK, then terminal client.

1. Merge through protected `main` with CI passing.
2. Update all three crate versions and release notes in a reviewed pull request.
3. Create a signed `vX.Y.Z` tag on the reviewed commit.
4. Publish the matching GitHub release.
5. Approve the `crates-release` environment deployment.

The `publish.yml` workflow exchanges GitHub OIDC for a short-lived crates.io
credential and stores no long-lived registry token.

Each crate must be bootstrapped once from the reviewed public `v0.1.0` tag using
an owner-controlled interactive crates.io session. Afterward, configure the
trusted publisher for owner `alsk1992`, repository `strata-sdk-rs`, workflow
`publish.yml`, and environment `crates-release`.
