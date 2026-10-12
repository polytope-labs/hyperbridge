# 2026-09-18 — The gateway must report the supported release

The SDK speaks one gateway release, `SUPPORTED_INTENTS_VERSION`. It is 4: the release that binds
solver selection to the EntryPoint v0.9 UserOperation. Release 4 keeps the `fillOrder` shape release
3 introduced, with a take per leg in `FillOptions.inputs` (selector `0x68ddf058`).
`assertGatewayRelease` reads `version()` on the gateway and throws unless it reports that release.
A gateway without the getter, or on any other release, is refused rather than encoded for.
`supportsRateFills` applies the same check, and a bid is signed or counted only when the destination
gateway reports the supported release.

Testnet chains (`TESTNET_CHAINS`) also accept release 3. The testnet gateways already run the
userOpHash solver selection that release 4 brings to mainnet, but still report 3. Both checks take the
destination's state machine id to tell the two apart. Mainnet accepts 4 only.

`SolverAccount` has no `version()` getter, so the account is not version-checked. Phantom bids still
require the sender to be delegated to one of the chain's configured `SolverAccount` addresses.

Nothing is cached. A proxy keeps its address across upgrades, so a cached answer could outlive the
deployment it described. A revert or empty return is "no getter"; a transport or provider error
propagates.

Earlier SDK releases stay published for gateways that have not been upgraded. An SDK pinned to
release 3 keeps working against a gateway until governance upgrades it to 4, then refuses it, so a
filler has to move to this SDK release when its gateways move.
