# 2026-08-20 — The curve amount is the fill, and the exposure cap does not shorten it

Chosen: `targetOutput = policyMaxOutput`, unconditionally.

Overfilling is a feature of the protocol, not an accident the filler should suppress.
`IntrinsicIntents.sol` has an explicit `solverAmount > totalRequired` branch that splits the excess
between the beneficiary and the protocol (`surplusShareBps`); `quotePhantomFill` publishes
`policyMaxOutput` as our quoted rate, so paying `output.amount` advertises a price we do not
honour; and `calculateProfitability`'s own doc says we overfill "if the pair pricing makes that
attractive. This is how we stay competitive."

`desiredOutput` is redundant with the ration `computeLegPolicyOutput` already applies to
`token0ForLeg`, and it is no longer a ceiling on payout at all — it survives only as the price
gate's comparand and in the short-fill logs. `maxOrderSize` binds in the token0 dimension, before
the rate is applied, which is where the exposure actually is: a capped leg never pays out more than
the capped slice's worth at the curve.

The trade-off, taken knowingly: escrow releases as `fillAmount / totalRequired`, so on a capped leg
paying above the user's pro-rata ask draws down more input than the cap fraction nominally allots.
That is more of the user's token for the same outlay — the cap bounds what the filler spends, not
what it receives — and treating the receive side as the thing to ration was clamping the price the
operator configured.

`capLimited` had to follow, and is now `capFraction.lt(1) && policyMaxOutput < output.amount`
rather than `desiredOutput < output.amount`. It gates partial-fill eligibility, and with the payout
unclamped a curve far enough above the order's rate can cover the whole ask out of a capped slice.
That is a full fill; gating it as a partial would reject cross-chain and calldata orders the filler
can serve.

Alternatives rejected:

- _Clamp to `desiredOutput` on capped legs only._ What this replaced. It keeps the escrow draw
  exactly proportional to the cap, but at the cost of quoting one price and filling another on
  every capped order — the same defect as the unconditional clamp, just rarer and harder to see.
- _Keep the clamp and stop publishing `policyMaxOutput` from `quotePhantomFill`._ Makes the quote
  match the fill, but by degrading the quote to the order's own rate — which is the counterparty's
  number, not ours. The price feed would stop carrying any information about the operator's curve.
- _Re-enable `maxOverfillBps` as part of this change._ Deliberately left alone. The clamp at the
  overfill-ceiling block is still a no-op assignment (`const policyMaxOutput = rawPolicyMaxOutput`)
  and `recordOrderOutcome` is still always called with `false`, so `maxOverfillBps` and the halt
  subsystem remain dormant config. Restoring the payout makes that ceiling meaningful again and it
  should be either re-armed or deleted outright — a separate decision from fixing the payout, and
  one that changes the filler's loss bound rather than its price.

Left standing, and known stale: `curveSurplusUsd` (the P&L fallback for legs with no opposite
curve) measures `policyMaxOutput - output.amount` and calls the difference "ours". With the surplus
now paid out that term is structurally zero on uncapped legs. It is report-only telemetry — it
never rejects an order or feeds the execute score — so it under-reports rather than mis-fills, and
re-basing it on the opposite curve was left out of this change.
