# 2026-09-18 — Bids carry a take per leg

Release-3 gateways settle `fillOrder(order, options)` as one quote per leg: `options.inputs[i]` is
the most input the solver takes and `options.outputs[i]` the most output it pays. `FXFiller` now
caches a take next to each output it prices: the leg's full input, scaled by the pair's
`maxOrderSize` cap fraction when the cap binds, and zero for a leg it skips. `CacheService`
stores the takes with the filler outputs (`setFillerOutputs(orderId, outputs, inputs)`,
`getFillerInputs`), and `prepareBidUserOp` signs them into `FillOptions.inputs`.

`buildApprovalAndFillCalldata` encodes the current `fillOrder` shape only. Simplex bids solely on
gateways reporting release 3; the SDK's estimate refuses any other. Funding-limited takes and a
rate gate for balance-limited legs are not part of this change.
