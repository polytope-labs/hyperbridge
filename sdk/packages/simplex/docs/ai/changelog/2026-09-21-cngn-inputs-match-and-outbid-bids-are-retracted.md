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

## A partial fill of ours settles only the bid that executed

One solver can hold several bids on one order, one per limit order at each level of its book. When
one of them filled part of the order, the filler did two things wrong:

- It claimed the holds of every bid on the commitment and released all but one.
- It retracted every bid.

This threw away the levels that should fill the rest.

`orderFilledOnChain` now carries `complete`. On a partial fill of ours, `claimExecutedBid` finds the
executed bid among our live bids and claims its holds by bid identifier. The filler draws that limit
order down, and nothing is retracted. The other bids keep their holds and can still fill, and all of
them are retracted once the order completes.

`executedHold` names the executed bid for both kinds of fill. It picks the best-rated bid whose take
covers the released escrow, which is the one the executor takes first. Bids from one solver often
sign the same take, because a bid whose payout covers the ask takes the whole input.

Fills are settled one at a time, on `IntentFiller.settlementQueue`, in the order they were scanned.
Several fills of one order can land in one block scan. Each settlement names its bid from the holds the
previous one left, so side by side they read the same holds, and two of them claimed the same one.

Every bid now approves the gateway for exactly what it pays, unconditionally. It first resets the
allowance to zero, for tokens that refuse to move a non-zero allowance. `buildApprovalAndFillCalldata`
used to skip the approval when the allowance covered the bid at signing time. That allowance was
spent by a sibling bid that filled first, so the next level reverted with `ERC20InsufficientAllowance`.

## Limit orders default to a 365-day TTL

`ttlSecs` stays optional on `POST /api/limit-orders` and `simplex.limitOrders.create`. Without it an
order used to live `[orderbook] defaultTtlSecs`, or 900 seconds when that was unset. It now lives
365 days (`DEFAULT_LIMIT_ORDER_TTL_SECONDS`). Both the request and the config can still set it.

## Fill history survives a resize

The order detail listed an order's fills from the bid rows whose holds named it. Settlement clears
those holds, so an order that had been drawn down, and therefore resized, showed no fills.

`settleFilledLimitOrder` now records each draw-down with `LimitOrderStore.recordFill`, in the same
transaction. The record (`LimitOrderFill`) holds the swap order's commitment, the executed bid, the
amount drawn down and the fill transaction, and is kept in the `limit_order_fills` table. The
`orderFilledOnChain` event now carries the transaction hash.

`limitOrders.withFills(id)`, which backs `GET /api/limit-orders/:id`, returns these records as
`fills`, plus the bids still drawing on the order as `bids`. The UI lists each fill with its time,
amount and a link to its transaction.

## Creating a limit order matches the book however symbols are cased

`LimitOrderService.create` looked books up by exact symbol. The operator UI sends the asset
registry's spelling (`CNGN`), and the orderbook lists `USDC-cNGN`, so posting a USDC/cNGN order from
the UI failed with `No book trades USDC against CNGN`. `resolveBook` now compares through
`normalizeSymbol`. The request is then carried on in the book's own spelling (`spelledAs`), because
the rate direction, the dust floor and the published decimals all look symbols up by it.

The operator UI no longer shows a limit order's expiry, in the list or the detail. With a 365-day
default it was noise; an order that does lapse still shows as `Expired`.

## A later leg is sized from what an earlier leg left on the limit order

When two legs of one order matched the same limit order, each leg's bid was sized from the order's
full `available`. Together they promised more than the order holds, so the later leg was dropped at
reservation and that leg got no bid. `calculateProfitability` now tracks what each limit order has
already committed to earlier legs of the same swap order (`plannedOn`). A later leg bids on what is
left, as a partial fill, and is skipped only when nothing is left.

## Operator UI: limit order list and fills

- The live and closed limit order lists are paginated, 10 orders per page.
- `New limit order` is a primary pill button.
- The detail panel lists fills as a table (when, amount paid out, transaction link), with the total
  paid out across all fills below it.
