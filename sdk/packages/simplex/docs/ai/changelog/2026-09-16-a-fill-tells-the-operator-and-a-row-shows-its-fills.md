# 2026-09-16 — A fill tells the operator, and a row shows its fills

Two review gaps against #1267 §2.

**`limitOrderResized` and `limitOrderFilled` were never emitted.** Every event the operator saw came
from `LimitOrderController`, which only sees the create and cancel they asked for. A resize happens
inside a fill, so the one change an operator did not initiate, and most needs telling about, was the
one that arrived silently: the size on their screen simply went stale. `LimitOrderService` now
reports a resize and a dust-floor close through a listener `Simplex` attaches after boot, the same
way the filler is handed the service itself, and publishes them as `limit-order:resized` and
`limit-order:filled`. A listener that throws is caught and logged rather than unwound into the fill.

**`GET /api/limit-orders/:id` returned the order alone**, where the issue specifies it with the fills
that consumed it. `BidStore.byLimitOrder` reads them back on both backends, newest first, and the
route answers `{ order, fills }`. A `remaining` that shrank is only explicable next to the bids that
took the difference.
