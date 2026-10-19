# 2026-10-16 — Retraction is per bid

Retracting a bid takes it out of Hyperbridge's discovery and returns its BRIDGE deposit (placing
it also pays a BRIDGE fee). It does not stop a placer that already fetched the bid: that placer can
still execute it until its `validUntil` (`bidValiditySeconds`, default 300, plus the 30 s discovery
pad). Retraction narrows the exposure; `bidValiditySeconds` remains the hard bound.

A bid that executes after it was retracted is settled like any other fill: by provenance
(`StoredBid.pricedBy`), recorded once per `(transactionHash, logIndex)`, followed by
`LimitOrderService.syncToChain`. Nothing is held locally, so retraction never has to give anything
back. The chain still enforces the budget (see
[the decision](../decisions/2026-10-16-limit-order-budgets-and-vault-liquidity-come-from-the-chain.md)).

## Interfaces

`BidStore.markRetracted(commitment, hash, rows?)` and `markDead(commitment, rows?)` take an
optional list of row ids (`StoredBid.id`). Without it they act on the whole commitment; an empty
list matches nothing.

**Breaking change for custom `BidStore` implementations.** `BidStore` is public, and the `rows`
filter is new on both methods. An implementation that ignores it still type-checks, but it closes
or flags every bid on the commitment instead of the rows named, so retracting one limit order's bid
would retract another's. A custom store must honour `rows` before upgrading.

`LimitOrderEvent` gains `{ kind: "expired" }` and `{ kind: "rejected" }`. Neither is added to the
public `SimplexEvents`.

## A retraction pass closes the rows it read

`IntentFiller.enqueueRetraction(commitment, { bids?, rows? })` retracts every live bid on the
commitment, or only those named by bid identifier or by row id. When the pass starts it snapshots
the rows it will close: unretracted, `success || pending`, and in scope. An empty snapshot (every
targeted row already retracted) skips the pass, and so does a Hyperbridge connection that is
already gone. A bid stored while the pass is in flight is not in the snapshot; it stays live for a
later pass or the sweep.

Each bid in the snapshot has its own outcome:

| Outcome | Row |
|---|---|
| Retracted | Marked retracted with its extrinsic hash. |
| `BidNotFound` | Marked retracted without a hash. Terminal: bids only leave the pallet by retraction. |
| No bid identifier (legacy row) | Marked retracted without a hash. |
| Failed | Left unretracted and flagged `dead` by row, so the next sweep retries it. |
| Pooled (`pending`) | Same as failed. The pass stops here; the bids after it stay open, since the account's next extrinsic queues behind the pooled one. |

The sweep queues one pass per commitment, scoped to its due rows.
`handleOrderFilledOnChain` and `handleRivalCompletion` act when any row on the commitment is live,
not only the newest.

## No bids on an order that has completed

Our complete fill and a rival's completion record the commitment in `completedOrders` (the last
1,000, oldest dropped). The two `EventMonitor` listeners record it when the event arrives, before
the work is queued on `settlementQueue`, which waits behind reconcile passes and orderbook requests;
a partial fill records nothing. `executeOrder`'s multi-bid loop stops before placing on a completed
order. A bid stored after completion is retracted at once with
`enqueueRetraction(commitment, { bids })` and its row marked `dead`, unless the
`pendingRetractions` handoff (a fill seen before any row existed) already retracts the whole
commitment.

## No bids on behalf of a limit order that has closed

`executeOrder` places the bid plans cached when the order was evaluated, so a plan can outlive the
limit order that priced it. Before each placement it reads that limit order and skips the plan unless
the order is `open` or `resizing`; an order that cannot be read counts as not standing. After the bid's
row is stored it reads the order again, and a bid that went out while the order closed is retracted at
once with `enqueueRetraction(commitment, { bids: [bid] })`. The re-check covers a cancel, expiry or
close that took its snapshot of the order's rows before this one was stored. Other plans of the
same swap order, priced by orders that still stand, are placed as usual.

## A limit order that can no longer stand behind its bids

`IntentFiller.retractBidsForLimitOrder(limitOrderId)` reads `BidStore.byLimitOrder`, keeps the live
rows, groups them by commitment and calls `enqueueRetraction(commitment, { rows })`. Another limit
order's bid on the same swap order is untouched, and a row a pass cannot close is flagged `dead` by
row only. It never throws; the stale-bid sweep is the backstop. The read is capped at the 500 newest
rows, so a limit order with more bid rows than that leaves its oldest live bids to the sweep. It
runs:

- **On cancel.** `LimitOrderController.cancel` calls it after `LimitOrderService.cancel`. A failure
  does not fail the cancel.
- **When the service closes the order.** `Simplex` listens to `LimitOrderService` and calls it on
  `filled` (including a close reached by `reconcileLimitOrderTallies` or `resize`, which never pass
  through settlement), `expired` (`retire` writes `expired` only over `open` or `resizing`) and
  `rejected` (`post`'s guarded write of `rejected`). Other event kinds do nothing.
- **After a fill, by status.** Once `settleFilledLimitOrder` has synced a limit order to its tally,
  it retracts that order's bids if the order is no longer `open` or `resizing`. While the order is
  live its bids are left to the chain: `debitOrder` refuses a fill past the budget, and the gas of
  such a revert is the accepted cost. The bid that just executed is not excluded; retracting it
  returns its deposit or `BidNotFound`, both terminal. These passes overlap harmlessly: each
  snapshot skips rows an earlier pass already retracted.

## Attributing a fill to a retracted bid

A retracted bid stays a candidate when a fill is attributed, since it can still execute until its
`validUntil`. The escrow the fill released decides first, since a bid releases its signed take, or
less when the gateway clamps it, so only a bid whose take covers the release can have been it:

1. Among the live bids whose order pays a delivered token, those whose take covers the release,
   best rate first.
2. Then among the retracted bids in the window, those whose take covers the release, best rate
   first. A retracted bid is attributed only when no live bid could have been it: a live bid the
   escrow rules out does not hide it.
3. Only then the payout fallbacks (the payout that matches the delivery, the closest one above it,
   the order the bids went out in), among the live bids if there are any.

A retracted bid is in the window when it was stored no more than `bidValiditySeconds` + the 30 s
discovery pad + `RETRACTED_BID_EXECUTION_MARGIN_SECONDS` (120) before the fill. The margin covers
the tail between `validUntil` and the transaction landing (the window is converted to blocks at the
fastest block time seen, so a slower chain keeps a bid executable longer in wall-clock time) and
matches the permit skew allowed for the same execution tail. The fill's time is when the
scanner's event reached the filler. It is not the time settlement runs, which can be long after,
behind the settlement queue, and would stretch the window.

## Shutdown drains before it disconnects

The boot `shutdown` stops `LimitOrderLifecycle` before `IntentFiller.stop()`. That only clears its
timers: an expiry tick already in flight can still emit after the drain, and the retractions it
queues are dropped because the Hyperbridge connection is gone, leaving those rows live for the next
run's sweep.

`stop()` clears the timers, drains the evaluation queue, then the chain queues, the settlement queue
and any rebalance, then the retraction queue, and disconnects Hyperbridge last. There is no timeout
on the first stage: shutdown waits for in-flight settlement and tally work (such as a
`reconcileLimitOrderTallies` pass) to finish before it drains retractions. The retraction drain is
bounded by `RETRACTION_DRAIN_TIMEOUT_MS` (30 s); on timeout the queue is cleared with a warning and
its rows stay live for the next run's sweep.
