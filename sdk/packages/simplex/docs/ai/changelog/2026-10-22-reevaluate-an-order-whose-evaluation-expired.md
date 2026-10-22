# 2026-10-22 — An order whose evaluation expired before its bids is evaluated again

`IntentFiller` evaluates an order while it waits for the order's source confirmations, then queues
its bids on the destination chain. Everything the evaluation caches (the gas estimate, the bid plans,
the filler outputs) expires after `CACHE_EXPIRY_MS` (60 s), and the bid loop's pin
(`2026-10-20-bid-loop-outlives-the-evaluation-cache.md`) keeps only what is still live when the loop
starts. A confirmation wait or chain-queue wait longer than that left the loop with no plans and no
gas estimate. Every bid then failed in `prepareBidUserOp` with
`No cached gas estimate found for order <id>`, and the order was never evaluated again. Seen on a
BSC order confirmed 79 s after its evaluation.

## Behavior

`handleNewOrder` hands `executeOrder` a re-evaluation of the order (`evaluateOrder` with the same
`canFill` answers). When the queued task starts, it pins the order and then checks the gas estimate:
the first thing an evaluation caches, and what every bid signs. If it is gone, the order is evaluated
again before any bid goes out, and the bids use the strategy and plans that evaluation chooses.

- A re-evaluation that passes on the order sends no bid, and reports why as any evaluation does
  (`orderSkipped`). The expired evaluation is never used.
- The re-evaluation's entries are written after the pin, so they stay live for the whole loop.
- An evaluation still live when the task starts is used as before, so every bid on an order is
  priced by a single evaluation.
