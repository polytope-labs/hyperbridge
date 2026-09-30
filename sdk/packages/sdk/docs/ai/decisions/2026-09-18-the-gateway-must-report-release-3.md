# 2026-09-18 — The gateway must report release 3

The SDK speaks one `fillOrder` shape, the one with a take per leg in `FillOptions.inputs`
(selector `0x68ddf058`). `assertGatewayRelease` reads `version()` on the gateway and throws unless
it reports `3`; a gateway without the getter, or on any other release, is refused rather than
encoded for. `supportsRateFills` and `readRateFillCapability` apply the same check, and a bid is
signed or counted only when the destination gateway reports release 3.

`SolverAccount` has no `version()` getter, so the account is not version-checked. Phantom bids still
require the sender to be delegated to one of the chain's configured `SolverAccount` addresses.

Nothing is cached. A proxy keeps its address across upgrades, so a cached answer could outlive the
deployment it described. A revert or empty return is "no getter"; a transport or provider error
propagates.

Earlier SDK releases carried the pre-quote shapes and resolved them from the ERC-1967
implementation address. They stay published for gateways that have not been upgraded; this SDK
release does not talk to them.
