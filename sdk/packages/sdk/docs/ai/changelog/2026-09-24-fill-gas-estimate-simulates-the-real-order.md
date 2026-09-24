# 2026-09-24 — The fill gas estimate simulates the real order

`GasEstimator.estimateFillOrder` used to simulate a copy of the order with `session` set to the
solver's own address, so it could sign the solver selection itself. The commitment is a hash of
the whole order, so the copy had a commitment with no escrow behind it. A same-chain fill releases
the escrow held under its commitment through `_withdraw`, which reverts `UnknownOrder` when there
is none. Every same-chain estimate therefore reverted in simulation and fell back to fixed gas
values. Cross-chain fills release escrow on the source chain, so they were unaffected.

The estimate now simulates the real order. The selection check it would then fail, since only
the user holds the session key, is already switched off in simulation: `buildStateOverride`
rewrites params slot 5, which packs the call dispatcher with `solverSelection` in the byte above
it, with that byte cleared. The live mainnet gateway was checked to hold exactly that layout.
