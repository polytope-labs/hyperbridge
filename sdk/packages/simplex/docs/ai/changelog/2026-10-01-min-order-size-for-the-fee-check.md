# 2026-10-01 — The fee check applies below a minimum order size

The FX strategy refused every full fill whose `order.fees` did not cover its execution cost: the
fill gas plus, for a cross-chain order, the relayer fee. That check now applies only to orders
below a configured size.

`[simplex] minOrderSizeUsd` sets the size in USD. It defaults to 20, must be a number `>= 0`, and
is read through `FillerConfigService.getMinOrderSizeUsd()`.

- An order worth less than `minOrderSizeUsd` is skipped when its fees fall short, as before.
- An order worth at least `minOrderSizeUsd` is bid on whatever fees it carries. The margin in the
  limit order that prices it pays for the gas.
- `0` checks no order's fees.
- The order's size is its input in USD, from `FXFiller.getOrderUsdValue`. An order that has no
  dollar value is treated as below the minimum.
- Partial fills were never fee-checked and are unchanged. The same-asset spread check is unchanged.

An order let through this way scores `order.fees - executionCost`, which is negative. The filler's
profit floor refuses anything that does not score above zero, so the strategy marks the order with
`CacheService.setFeeCheckWaived`, and `IntentFiller.evaluateOrder` exempts a marked order from the
floor. The mark is cleared at the start of every evaluation and set only once the evaluation has
bids, as the partial-fill mark is. The recorded `profitUsd` of such a fill is the fee shortfall.

The `FX swap profitability evaluation` log line carries `feeCheckWaived`, and the fee-check skip
line carries `minOrderSizeUsd`.
