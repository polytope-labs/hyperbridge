# 2026-09-16 — One swap can draw on several limit orders

The matcher returned exactly one order, to keep the draw-down a one-to-one piece of bookkeeping. That
convenience cost fills: when the best-priced order's depth did not cover the swap, the filler skipped
it cross-chain or under-filled it same-chain while holding inventory a level down that would have
cleared the rest.

It also left simplex out of step with what the orderbook had already told the swapper.
`SwapQuote.rate` is documented as "the clearing price, the best at which the orders at it or better
can fill the trade together", and `RouteLiquidity.maxFillableIn` as "one order cross-chain, combined
same-chain". The quote a swapper acted on was built from several of our orders and we honoured one.

`matchLimitOrders` walks levels best payout first and stops as soon as the ask is covered, so a swap
one order covers still draws on one. `matchLimitOrder` stays as the single-best wrapper for callers
that only need to know whether anything matches. Ordering is deterministic to the last comparison,
payout then what is left then id, because the draw-down after a fill walks the same orders in the
same sequence.

## What a bid holds

A bid row carries `reservations`, a list of `{ limitOrderId, amount }`, in the order the payout draws
on them. `IntentFiller.holdAll` takes every hold or none: a bid that reserved part of what it means
to pay would promise output no order is holding for it, so a hold that cannot be taken gives back the
ones already taken and the order is skipped. `claimReservation` hands the whole list over exactly
once, as it did with the single pair.

A fill shares out over the holds in that same order, each taking what it held until the delivery runs
out. An order the delivery never reached takes no draw-down and simply gets its hold back. Each
draw-down still precedes its own release, so a crash mid-way understates capacity rather than
advertising output already paid.

Existing databases get a `reservations` column; the two columns it replaces are left behind rather
than migrated, since a hold outlives its bid by minutes and nothing reads a settled one.
