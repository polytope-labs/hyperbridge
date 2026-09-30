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

## The FX strategy runs without `[[pairs]]`

Boot used to build the FX strategy only when the config had at least one `[[pairs]]` block. A
config without one scanned orders it could never fill, and every order ended at `No strategy can
fill this order`, with no strategy left to give a reason. Prices come from limit orders, so the
strategy now always runs. `[[pairs]]` only names the markets the dashboard lists.

Boot now always asserts a confirmation policy for every configured chain. The built-in policies
cover the supported mainnets. A testnet chain needs its own `[confirmationPolicies."<id>"]`, as
it already did whenever pairs were configured.

## A pending bid no longer blocks the next one

A bid still holds its payout against the limit order that priced it. The hold is drawn down if the
bid wins and given back if it loses. It no longer counts against the next bid: a new bid is sized
from the order's `remaining`, not `remaining - reserved`, and `reserve` refuses only a single hold
larger than `remaining`.

A losing bid kept its hold until the retraction sweep found it, up to its one-hour TTL after its
order was refunded, so a few pending bids could stop an order bidding for an hour. On mainnet one
bid was cut to 877 of the 1,374 cNGN it would have offered, because earlier bids held the rest.

The trade: the order's size now bounds each bid and the fills that draw it down, not the total
promised across bids still pending. If several of those win at once, fills can pay out more than
the order's size, up to what the wallet holds. The dashboard shows what live bids hold, without
"leaving … to draw on".
