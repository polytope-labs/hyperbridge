# Resolving the deployed FillOptions ABI

`getFillOptionsVersion` reads gateway `version()` before applying historical chain or implementation
rules. Gateway release 4 uses FillOptions ABI 3 (`inputs` appended after `outputs`); releases 2 and 3
use ABI 2 (`validUntil` present). Zero, the locked raw implementation version, and unknown future
versions are rejected. Release versions and ABI versions are different numbers.

For an absent getter or historical version 1, `CHAINS_WITHOUT_VALID_UNTIL` and the ERC-1967
implementation address distinguish ABI 1 from ABI 2. `LEGACY_FILL_OPTIONS_IMPLEMENTATIONS` contains
the pre-validUntil deployment address. These rules support old deployments; they do not override a
new release detected at the same proxy address.

Resolution is fresh for each operation so upgrades and different chains sharing a proxy address
cannot reuse stale results. Genuine missing-function responses permit legacy resolution; provider
and transport failures propagate. Every ABI-3 bid also requires a compatible release-4 SolverAccount,
checked independently of the gateway before signing.

Escrow getter compatibility is separate: release 3 includes both token-keyed and per-leg deployments.
Use `readLegEscrow` and `readLegPartialFill` to select those getters.
