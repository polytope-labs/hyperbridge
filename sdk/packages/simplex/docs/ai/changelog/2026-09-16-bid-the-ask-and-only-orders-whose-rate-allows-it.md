# 2026-09-16 — Bid the ask, and only from orders whose rate allows it

Three findings from review, two of them live before the multi-order work.

## The bid is the ask, not the whole offer

`targetOutput` is not a ceiling the filler stays under, it is `solverAmount` at the gateway. On a full
fill the gateway sets `fillAmount = totalRequired` and `_splitSurplus` hands everything above it to
the beneficiary and the protocol, debiting the solver the whole bid; with output calldata all of it
goes to the protocol. So bidding a limit order's full offer when the swapper asked for less gave the
difference away, and it fired in the ordinary case, since finding an order that prices better than
the ask is the point.

Worse, that donation was booked as the profit that justified the fill: `payoutSurplusUsd` counts the
gap between the offer and the ask, and a partial's whole P&L is that figure.

The bid is now `min(offered, output.amount)`. The escrow released is identical either way, so what is
not bid is margin kept and `payoutSurplusUsd` describes something real. `maxOverfillBps` stays as a
warning about a limit order priced well away from the market, which is what an offer far above the
ask now means. The clamp it once performed was disabled in #964 for curve-priced legs, where the
computed amount was a curve's answer for that leg; under limit orders it is the operator's rate
applied to the whole input, which is a different thing.

## An order only takes part if its offer covers the ask

Escrow release is strictly proportional: `Released(filled) = escrowTotal * filled / totalRequired`.
A fill of `f` out of `T` releases `I * f / T`, so its effective rate is `T / I` whatever `f` is. Every
fill of an order is therefore paid at the swapper's rate, not its own, and an order may only take
part where that rate is inside its own terms: `T / I <= price`, which is exactly
`offer >= requestedOutput`.

That condition was dropped earlier in this stack as a ranking preference. It is not one. Without it a
single order whose offer fell short would partial-fill above the rate the operator signed.

## One fill may draw on several limit orders, and the total is the ask

Combining levels first summed per-order offers that were each computed against the whole input, which
bills one input to several orders and overpays by the difference between the best rate and the worst.
Clamping the bid to the ask removes that by construction: every order in the set already clears the
ask on rate, so what they add up to is depth rather than price. The total paid is the ask, each order
funds a slice of it, is drawn down by that slice, and receives that fraction of the input, so every
one of them settles at `T / I` and stays inside its own terms.

Each order in the set then bids for itself, best offer first, which
`2026-09-16-one-swap-can-draw-on-several-limit-orders.md` covers.

## Smaller

`orderbook-schema.graphql` is refreshed from `hyperfx-orderbook@main`, which has since removed
`SwapQuote.priceBucket`, added `Query.chains` with per-chain token decimals, and rewritten the quote
semantics. It is a pinned copy and would drift again silently, since the test
validates against the pin rather than against the server, so
`.github/workflows/check-orderbook-schema.yml` now fetches the schema and diffs it, weekly and on any
change to this package's orderbook code. `schema.test.ts` carries the one command that refreshes it.

Token decimals come from the orderbook's own registry too. `serverInfo` now carries `chains` with the
tokens each one registers, `limits()` already caches it for five minutes, and every post and repost
needs both sides of the pair, so this is two chain reads saved each time. It is also the registry the
server prices against, which is what makes it the right source rather than merely the cheap one; a
chain or symbol it does not list still falls back to the token. The matcher docstring no longer claims a cross-chain order reverts on any under-fill.

## A fill settles its holds in one transaction

Claiming what a bid held, working the orders down by what went out and giving the rest back are one
decision about the same holds, and they were three separate writes. A crash between two of them gave
a hold back against an order that was never drawn down, which leaves it advertising output already
paid. §6 asked for them to be one store transaction and this took the weaker route of ordering them,
on the reasoning that a transaction would have to span two stores. It does not: `SqliteDataStore`
hands the same `bids.db` connection to the bid store and the limit order store.

`LimitOrderStore.transaction` runs the settlement as a unit, with the same rollback guard the state
store uses, and the in-memory store restores a snapshot rather than leaving the default backend
weaker than the configured one. Only store writes go inside. Putting the order back on the book is a
round trip and now happens after, which is why `settleFill` splits: the draw-down belongs in the
transaction, `resize` does not.
