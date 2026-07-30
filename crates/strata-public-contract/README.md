# `strata-public-contract`

Strict data types and fixtures for Strata's versioned public product contract.

The crate deliberately contains no HTTP client, transaction builder, wallet
handling, or private Sonar implementation types. Enable the `fixtures` feature
to consume the canonical JSON examples used by the language SDK test suites.

Atomic token amounts are represented as unsigned base-10 strings so callers do
not lose precision at a JSON or language boundary.
