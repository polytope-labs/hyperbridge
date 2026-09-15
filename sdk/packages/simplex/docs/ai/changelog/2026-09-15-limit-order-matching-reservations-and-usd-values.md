# 2026-09-15 — Limit order matching, reservations and USD values

The pieces the filler needs to price from the operator's limit orders instead of from pair curves.
Nothing is wired into `FXFiller` yet, so behaviour is unchanged; the switch is a separate change.

## Matching

`matchLimitOrder` in `src/orderbook/matching.ts` picks the limit order an incoming order is priced
against. An order serves when it is `open` and past no operator expiry, its `fillChain` is the
incoming order's destination, its input symbol and output address line up with what is being
swapped, and its offer covers what was asked for. A cross-chain swap also has to come from a chain
the order declared; a same-chain swap ignores the declaration, as the orderbook does.

A limit order's input and output are fixed when it is created, so direction is not something the
matcher infers: an order that takes USDC in and pays cNGN out prices USDC to cNGN swaps and is never
turned around to price cNGN to USDC at the inverse rate. Trading both ways means holding an order
for each, and an operator can hold as many as they like at once.

`offer` is what the order pays for the incoming input at its own signed rate, floored to the output
token's raw unit so it stays inside the rate that was signed. When several orders serve, the largest
offer wins and a tie goes to the one with more left. Exactly one is returned, which is what keeps
drawing an order down on a fill a one-to-one piece of bookkeeping. Nothing matching means no fill:
there is no fallback price.

## Reservations

`payout` is `min(offer, remaining - reserved)`, so an order never offers more than it has left even
when the wallet holds more. `LimitOrderStore` gains `reserve` and `release`, and the bid row gains
`limit_order_id` so a fill can find the order it drew on. Existing databases get the column added in
place.

`reserve` is a compare-and-set guarded on the `reserved` it read, not a read followed by a write. Two
chains bidding against one limit order would otherwise both see room in the gap between the two and,
between them, promise more output than the order has.

## USD values

`usdFactorsFrom` in `src/orderbook/usd.ts` derives dollars from the limit orders themselves, which is
what replaces the anchor graph the curves fed. Dollar stables are pinned at $1 and never re-priced,
so a mis-set stable-against-stable market cannot move the anchors, and everything else is reached by
walking outward from them until nothing new is learned. Edges are sorted before the walk and a symbol
is priced by the first edge that reaches it, so two routes that disagree slightly still resolve the
same way on every run.

A symbol no order connects to a dollar stays unpriced, and `usdValueOf` answers null for it. That is
deliberate: the value sizes a confirmation wait, and inventing one would under-wait a large order
against a source chain that has not finalised.
