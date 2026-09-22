# 2026-09-21 — cNGN inputs match, and outbid bids are retracted

## Symbols match case-insensitively

`matchLimitOrders` compared a limit order's input leg, spelled as the orderbook lists the book
(`cNGN`), with the incoming order's input symbol from the asset registry, which upper-cases every
symbol (`CNGN`). Every swap paying cNGN in matched nothing and was skipped with
`No limit order matches this order`. `serves()` and the same-asset check in `FXFiller` now compare
through `normalizeSymbol`.

## A rival's completed fill retracts our bids

`EventMonitor.handleFill` stopped at `if (!ours) return`, so a solver outbid on an order never
retracted its bids. Their limit-order holds and 0.1 BRIDGE deposits stayed locked until the
stale-bid sweep, which only takes bids older than an hour.

`ScannedFill` now carries `complete`: `true` for `OrderFilled`, which the gateway emits only once
every leg is filled, and `false` for `PartialFill`. It is optional, and a scanner that leaves it unset
is treated as not complete. `orderFillObserved` passes it through. When another solver completes an
order this filler holds an unretracted bid on, `IntentFiller.handleRivalCompletion` retracts the
bids at once. The retraction returns the deposit and releases the holds. A rival's partial fill
leaves the bid standing, since it may still fill the rest.

## Each limit order starts at its own nonce

A posted op is built from the order's tokens, amounts, TTL and `orderNonce`, and nothing else, and
every new order started at `orderNonce` 0. The orderbook refuses an op it has seen with `REPLAYED`,
and the poster bumps the nonce once. So an order re-created on the terms of two earlier orders, which
had used 0 and 1 between them, was rejected for good. `LimitOrderService.create` now starts each
order at a random 64-bit nonce (`initialOrderNonce`), and resizes still step on from it by one.
`LimitOrderInsert.orderNonce` carries it to the store.

## Bids carry the limit order's own rate

`FXFiller` bid every limit order at the swapper's rate: the requested output against the whole
input. Bids from operators with different prices tied, and the executor ran them in the order they
arrived. Each bid now quotes its limit order's own rate:

- **Output:** the order's whole offer for the input (`min(offer, remaining - reserved)`), then what
  the wallet can fund.
- **Take, when the payout covers the ask:** the whole input, so the order still fills in one go. At
  the full offer this is exactly the limit order's rate.
- **Take, when the payout is short of the ask:** a partial fill at the order's own rate,
  `inputFor(payout, price)`, rounded up. It is capped at the most the gateway accepts,
  `payout * escrow / ask`.

The gateway credits the swapper `take * ask / escrow` and charges the solver the escrow it releases
at the bid's rate. So a better price reaches the swapper as surplus, which the swapper shares with
the protocol, and the executor takes the best rate first.

The fill event reports only the credited output, so settlement no longer draws a limit order down
by it:

- A hold records the bid's `take`.
- `settleFilledLimitOrder` reads the released input from the event and names the bid that filled:
  the hold whose take was released or, for a fill the gateway clamped, the best-rated bid whose take
  covers the release.
- It draws that bid's limit order down by what it was charged: `amount * released / take`, rounded
  up and never below the credit (`chargedFor`).
- A hold without a take falls back to the credited output.

## Multi-leg orders are filled leg by leg

`EventMonitor` dropped every order that did not have exactly one input and one output. It now passes
any order whose inputs and outputs pair up leg by leg, and skips only unpaired ones
(`Unpaired order legs`).

`FXFiller.calculateProfitability` prices each leg on its own:

- Matching: `matchLeg(order, leg)` matches the leg's input and output against the limit orders, and
  each matching limit order gets a bid on that leg.
- Quotes: a bid quotes its leg and zero on every other leg (`FillOptions.inputs` / `outputs`), which
  the gateway reads as skipping those legs.
- Partial fills: a bid on one leg of a multi-leg order leaves the others open, so it is a partial
  fill. An order carrying output calldata, which cannot be filled in parts, gets no bids.
- `BidPlan.leg` records the leg, and a hold's take is that leg's.
- `getOrderUsdValue` sums every leg's input for the confirmation curves.
