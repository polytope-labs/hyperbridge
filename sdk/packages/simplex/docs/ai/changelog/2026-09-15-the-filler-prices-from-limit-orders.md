# 2026-09-15 — The filler prices from limit orders

`FXFiller` now prices every incoming order against the operator's limit orders instead of against
pair curves. `canFill` asks the matcher, `calculateProfitability` pays what the matched order allows,
and `getOrderUsdValue` derives dollars from the same orders. An order that matches nothing is not
filled: there is no fallback price.

## What a fill is now worth

`payout` is `min(offer, remaining − reserved)`, brought down to the output token's own decimals, and
that replaces what the curve used to yield. Everything downstream is unchanged: the paymaster and
funding-venue reserves, spending the wallet first, sourcing the shortfall from a venue, gas
estimation, the cross-chain dispatch-fee affordability check, and both profit gates.

The leg loop and the per-pair budgets are gone. Orders reach the filler single-legged already
(`EventMonitor` drops anything else) and one incoming order draws on exactly one limit order, so
there was nothing left for the loop to iterate.

## What the bid signs

A release-3 fill is one quote per leg: `FillOptions.outputs[i]` is the most the solver pays and
`inputs[i]` the most escrow it takes for that output, and the gateway refuses a take above the share
the fill earns (`RateBelowOrder`). The take is `releasedInput` — the whole input on a fill that meets
the ask, the same proportion of it as the output on an under-fill — which is the escrow the gateway
releases for that fill anyway, and the figure the P&L below already reads. A payout too small to
release any escrow is skipped rather than signed with a zero take.

Two P&L terms changed with the price source. `fxMarginUsd` marked the open side of a pair against
its opposite curve; a limit order has no opposite side, so it is gone. `curveSurplusUsd` becomes
`payoutSurplusUsd`, what the matched order was willing to pay above what was asked for, which is
what pays for a partial fill's gas. The same-asset spread gate survives unchanged in spirit, now
keyed on the order's own input and output symbols rather than on a same-token pair.

`ContractInteractionService.cacheService` gains a matched-limit-order slot so the bid reserves
against the same order the price came from, and `AssetRegistry.symbolFor` is the reverse of
`getAddress`, replacing a hand-rolled lookup in `bootFiller`.

## What a `[[pairs]]` entry does

Nothing, for matching. The matcher reads the limit orders alone, so adding or removing a market no
longer changes what the filler will fill. `addPair` and `removePair` still enforce their structural
rules (no duplicate or reversed market, no orphaned USD anchor, never remove the last one) and the
curve fields are still carried, but both are on their way out with the curves themselves.
