# Resolving the deployed FillOptions ABI

`getFillOptionsVersion` asks the gateway for `version()` first. Release 3 encodes FillOptions ABI 3
(`inputs` after `outputs`); releases 2 and 3 encode ABI 2 (`validUntil` present). Any other
reported version is refused, since neither ABI is known to decode there. Release numbers and ABI
numbers are different sequences.

Only a gateway without the getter falls through to the historical rules: `CHAINS_WITHOUT_VALID_UNTIL`
names testnets still on the pre-`validUntil` code, and `LEGACY_FILL_OPTIONS_IMPLEMENTATIONS` holds
that code's ERC-1967 implementation address, which CREATE2 makes the same on every chain. Anything
else without a getter is ABI 2.

Resolution is never cached. A proxy keeps its address across upgrades and different chains reuse
addresses, so a cached answer could outlive the deployment it described. A missing-function error
selects the historical rules; a transport or provider error propagates.

ABI 3 bids additionally require release 3 from the configured `SolverAccount` implementation and
from the solver's live delegation, checked before signing.
