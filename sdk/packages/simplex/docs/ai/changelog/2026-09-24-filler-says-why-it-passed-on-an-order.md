# 2026-09-24 — The filler says why it passed on an order

When no limit order serves an incoming order, the FX strategy logs `No limit order matches this
order` at `info`, with a `reason`. The reason names the furthest check any limit order got
through:

- no limit order is open
- none fills on the order's destination chain
- none takes the order's input token in
- none pays out the token the order asks for
- none accepts swaps from the order's source chain
- the rate: "it asks for X; the best limit order offers Y"
- the depth: the orders that meet its rate have nothing left to pay out

A multi-leg order gets one reason per leg. An input token the asset registry does not know is
named as that.

`whyUnmatched` in `src/orderbook/matching.ts` runs the same checks as `matchLimitOrders`, through
`firstFailed`, so the explanation cannot drift from the match.

The filler's own line now says `No strategy can fill this order` when no strategy could fill it,
in the log and as the activity feed's skip reason. `No profitable strategy found for order` is
kept for orders a strategy priced at a loss.
