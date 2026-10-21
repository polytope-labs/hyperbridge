# 2026-10-21 — Bids go to the orderbook too, and to it alone without BRIDGE

The HyperFX orderbook now takes bids on swappers' orders (`submitBid(userOp)`) and serves them
under the order's commitment (`bids(commitment, leg, solver)`). Simplex posts every bid there, and
the SDK reads bids from there beside Hyperbridge, so a solver can bid without a BRIDGE balance.

## Where a bid goes

`FillerStrategy.executeOrder(order, bids?: BidPlacer)` takes the filler's bid venues in place of
the raw `IntentsCoprocessor`. The strategy still builds and signs the fill UserOp; `BidVenues`
(`src/core/bid-venues.ts`) decides who receives it:

- **The orderbook, always.** The same SCALE-encoded op Hyperbridge takes. The orderbook accepts it
  only from a solver holding an active limit order on the leg's book filling on the order's
  destination (`NO_LIMIT_ORDER`), serves it for `serverInfo.bidTtlSecs` (600 s on testnet), and
  has no way to withdraw it; the op's own `validUntil` bounds execution as before.
- **Hyperbridge, when the account holds BRIDGE for it.** `BridgeBalanceGate` reads the account's
  free balance over the coprocessor's query API and lets a bid through while it covers one bid's
  cost: the pallet's storage deposit (the on-chain `StorageDepositFee` when set, else the runtime
  constant), the existential deposit, and the extrinsic fee as `paymentInfo` estimates it. The
  reading stands for `BRIDGE_BALANCE_TTL_MS` (30 s), is lowered by one bid's cost after each
  placement that landed or pooled, and is zeroed when Hyperbridge answers `InsufficientBalance`
  or refuses the fee. A read that fails lets the bid through; Hyperbridge's own refusal costs
  nothing. Without BRIDGE the bid goes to the orderbook alone, logged once at `warn` until the
  balance is back.

The two venues are posted side by side. `FillResult.success`, `txHash`, `pending` and `error`
remain Hyperbridge's answer; `FillResult.orderbook` is the orderbook's (`SubmitBidResult`:
`accepted`, `unchanged`, `rejected` with a `RejectionCode`, or `failed`, including
`REQUEST_FAILED` for a request that got no answer). A bid is *placed* when Hyperbridge took or
pooled it or the orderbook holds it; that is what `orderExecuted.success` and the activity's
`bid` row now mean. Order intake no longer refuses a solver-selection order for want of a
Hyperbridge connection while an orderbook is configured.

`IntentFiller.setOrderbook(client)` hands the filler the `OrderbookClient` boot already builds
for `LimitOrderService`.

## What the bid store records

`StoredBid.onOrderbook` (and `BidInsert.onOrderbook`, SQLite column `on_orderbook`, added in
place on older databases) says the orderbook holds the bid. It never changes: nothing withdraws
an orderbook bid. `success` and `pending` keep their Hyperbridge meaning, so:

- The retraction sweep, `retractBidsForLimitOrder` and the fill-time retractions touch only rows
  with a Hyperbridge deposit (`success || pending`). An orderbook-only row is left to expire.
- Fill attribution (`recordExecutedFill`) counts an orderbook-only row as a live candidate, inside
  the same window a retracted bid gets (`bidValiditySeconds` + discovery pad + permit skew, back
  from the fill's block time), since nothing retracts it and an expired one cannot have executed.

A custom `BidStore` that drops `onOrderbook` loses fill attribution for orderbook-only bids, not
funds. `BidDto.onOrderbook` reaches the dashboard, whose order history shows **Orderbook** for a
bid Hyperbridge did not take but the orderbook holds.

`OrderbookClient.submitBid(userOp)` is the new document; `REJECTION_CODES` gains `NO_LIMIT_ORDER`
and `TOO_MANY_BIDS`. The schema fixture is refreshed from `polytope-labs/hyperfx-orderbook` main.

## SDK: bids from both sources

`HyperFxOrderbook.bids(commitment, { leg?, solver? })` returns `OrderbookBid[]` (solver,
commitment, leg, book, side, chains, tokens, `price`, `amountIn`, `amountOut` at 1e18,
`receivedAt`, `expiresAt`, and the SCALE-encoded `userOp`). `OrderExecutor.fetchBids` reads the
coprocessor (when configured) and the gateway's orderbook together and merges by
`(sender, bidId)`, where the bid id is `keccak256(callData)` as Hyperbridge files it; the
Hyperbridge copy wins a tie since it carries the deposit. One source failing leaves the other's
bids; both failing throws. `FillerBid.venue` is `"orderbook"` for a bid read there (its `filler`
is the solver's address, `deposit` is 0). `IntentGateway.executeOrder` no longer requires a
coprocessor, and `withOrderbook` now also redirects where bids are read from.
