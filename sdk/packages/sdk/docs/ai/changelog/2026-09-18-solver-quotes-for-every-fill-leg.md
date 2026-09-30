# 2026-09-18 — Solver quotes for every fill leg

`FillOptions.inputs` is required: one take per leg, paired positionally with `outputs`, and the
ratio is the solver's rate. `encodeFillOrder(order, options)` validates that every leg is quoted
(a zero take with a zero output skips the leg) and `decodeFillOrder` returns `null` for calldata
that is not the current `fillOrder` shape or does not cover the order's legs. `previewRateFill`
reports credit, release, payment and surplus for a take at a given progress, with the contract's
rounding.

`BidManager` ranks bids by price normalized to the order's full input, so an oversized quote cannot
improve its rank while its signed amounts still bound funding; a bid without a take for every leg is
not a candidate. Multi-leg orders execute again. Before signing, the gateway, the solver's live
delegation and the configured `SolverAccount` implementation must all report release 3
(`assertGatewayRelease`, `supportsRateFills`); phantom aggregation applies the same gate to every
bid and prices each leg at its normalized output.

`estimateFillOrder` takes optional `inputs` and `outputs`; without them it simulates a full fill at
the order's rate, and custom outputs require explicit inputs. `FillOrderEstimate.inputs` reports
the takes the estimate used.

Removed: `FILL_ORDER_V1_ABI`, `FILL_ORDER_V2_ABI`, `HistoricalFillOptions`, `FillOptionsVersion`,
`getFillOptionsVersion`, `resetFillOptionsVersionCache`, `LEGACY_FILL_OPTIONS_IMPLEMENTATIONS` and
`CHAINS_WITHOUT_VALID_UNTIL`. Earlier SDK releases remain published for gateways below release 3.
