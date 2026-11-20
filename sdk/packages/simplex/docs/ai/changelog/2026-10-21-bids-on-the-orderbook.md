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
- **Hyperbridge, when the account holds BRIDGE for it.** `BridgeBalanceGate` (one per filler)
  reads the account's free balance over the coprocessor's query API and lets a bid through while
  it covers one bid's cost: the pallet's storage deposit (the on-chain `StorageDepositFee` when
  set, else the runtime constant), the existential deposit, and the extrinsic fee as `paymentInfo`
  estimates it. The reading stands for `BRIDGE_BALANCE_TTL_MS` (30 s), is lowered by one bid's
  cost after each placement that landed or pooled, and is zeroed when Hyperbridge answers
  `Inability to pay some fees` or `InsufficientBalance` (not Substrate's generic 1010, which a
  stale nonce carries too). A read that fails lets the bid through and is not retried before the
  TTL, so a dead endpoint costs one wait per TTL. The gate logs once at `warn` when bids start
  going to the orderbook alone and once at `info` when the balance is back. Only the pool's fee
  refusal is free: `place_bid` is not `Pays::No`, so an included extrinsic that fails with
  `InsufficientBalance` on the deposit still pays its fee, which is what the gate is there to
  avoid.
- **Or no Hyperbridge at all.** `simplex.substratePrivateKey` and `simplex.hyperbridgeWsUrl` are
  optional (the URL is required only with a key). Without a key the solver still delegates its
  SolverAccount and funds the EntryPoint (`setupSolverSelection` no longer waits on a Hyperbridge
  connection) and bids on the orderbook alone. `simplex init` lets the key be skipped and then
  writes neither field; the UI wizard does the same.

The orderbook post goes out first and does not wait on the balance read. `FillResult.success`, `txHash`, `pending` and `error`
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
- **What retraction now guarantees.** The orderbook cannot withdraw a bid, so when a limit order
  is cancelled, re-priced or closed, retraction pulls the Hyperbridge copy only; the orderbook
  keeps serving the op until `bidTtlSecs`, and the op's own `validUntil` bounds it as before. The
  limit order's on-chain budget (`debitOrder`) is the only cap on what such a bid can still fill,
  which is the same cap a retracted Hyperbridge bid already executing was held to.
- `BidStats.successful` and `failed`, and `BidStore.failed()`, count a bid the orderbook alone
  holds as placed, not failed, in both stores: the dashboard's **Failed** figure means no venue
  took the bid.
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
coprocessor (when configured) and the gateway's orderbook together and merges by the op's
userOpHash, so a bid a solver filed on Hyperbridge under some other identifier still merges with
its orderbook copy; the Hyperbridge copy wins a tie since it carries the deposit. One source
failing leaves the other's bids; both failing throws; an orderbook bid whose op does not decode is
skipped on its own. `FillerBid.venue` is `"orderbook"` for a bid read there (its `filler` is the
solver's address, `deposit` is 0). The coprocessor is optional by contract: `IntentGateway.create`
without one reads bids from the orderbook alone, which the docs' setup examples now show;
`withOrderbook` also redirects where bids are read from. `IntentsCoprocessor`'s "Transaction
failed after N attempts" now carries the last attempt's error.

## End to end

The testnet swaps run gains `orderbook-only-bid`, the `same-chain` swap with solver 1 restarted
with no Hyperbridge key, so its bid can only reach the orderbook, and `orderbook-only-swapper`,
the same swap with the SDK given no coprocessor, so it reads bids from the orderbook alone. The
swap worker reads the SDK's bids from the orderbook under test and records which solvers each
venue served. See [the e2e note](2026-09-22-testnet-swap-e2e.md).
