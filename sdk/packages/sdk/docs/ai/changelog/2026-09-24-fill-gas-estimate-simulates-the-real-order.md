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

With the real order, the same-chain simulation got further and reverted inside USDC instead. The
estimator faked the solver's token balances at `maxUint256 / 2`, which is exactly FiatTokenV2_2's
cap of 2^255 - 1, and a same-chain fill then credits the solver the released input. The fake
balances, deposit and allowances are now 2^128: far more than any real amount, and far below any
cap.

The real order only has escrow once it is placed. `IntentGateway.execute` quotes `order.fees`
through this estimate before placing it, so that same-chain simulation still reverted
`UnknownOrder` and fell back. For a same-chain order, `buildStateOverride` now also writes the
order's escrow into the gateway, `_orders[commitment][leg]` at storage slot 9 (checked against
the live mainnet gateway), set to each leg's input amount, and gives the gateway 2^128 of each
input token (or native balance) to release it.

The simulated op carries zero gas limits and `preVerificationGas`. Bundlers such as Alchemy's
return a non-zero value as given rather than estimating it, so the fixed placeholders came back as
the estimate.
