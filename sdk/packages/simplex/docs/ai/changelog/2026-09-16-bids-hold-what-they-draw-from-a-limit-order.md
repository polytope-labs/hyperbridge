# 2026-09-16 — Bids hold what they draw from a limit order

A bid now takes its payout out of the matched limit order before it goes out, and gives it back when
it loses. Without the hold, two chains evaluating at once both read the same `remaining` and between
them promise more output than the order has.

`IntentFiller` reserves against the limit order that priced the order, then records the bid carrying
both the order's id and the amount held. The reservation is taken before `executeOrder`, so a bid is
never sent against capacity that is already spoken for. An order whose limit order ran out of room in
the meantime is skipped rather than bid on.

## Settling the hold exactly once

A losing bid is retracted, which is where its hold is given back. A winning bid is *also* retracted
eventually, by the stale sweep, long after the fill has drawn the order down, so releasing on
retraction alone would undo a draw-down that already happened.

`BidStore.claimReservation` is what makes that safe. It hands the reservation over once and answers
null afterwards, so release and the fill-time conversion can both run without either needing to know
whether the other did. The bid row carries `reserved_amount` alongside `limit_order_id`, and claiming
nulls it under a guard on the value just read. Existing databases get both columns added in place.

Release happens on a successful retraction, on `BidNotFound` (nothing was ever on the pallet to
reclaim), and on a bid that failed outright without pooling. A pooled retraction keeps its hold: the
extrinsic may still land, so the bid has not settled. A bid that failed before reaching Hyperbridge at
all has no row to claim from, so it is released directly.

The window between a successful submission and the bid row being written is the one place a hold can
be orphaned, the same window that can already strand a deposit, and it is bounded by the same write.

`bootFiller` also now passes the limit order store to `FXFiller`, which it had not been doing since
the engine started pricing from limit orders.
