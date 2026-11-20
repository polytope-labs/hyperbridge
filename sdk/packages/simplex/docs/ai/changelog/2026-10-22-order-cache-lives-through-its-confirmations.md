# 2026-10-22 — An order's evaluation lives through its confirmations, and is redone if it expires first

`IntentFiller` evaluates an order while it waits for the order's source confirmations, then queues
its bids on the destination chain. Everything the evaluation caches (the gas estimate, the bid plans,
the filler outputs) expired 60 s after it was written, and the bid loop's pin
(`2026-10-20-bid-loop-outlives-the-evaluation-cache.md`) keeps only what is still live when the loop
starts. A confirmation wait longer than that left the loop with no plans and no gas estimate. Every
bid then failed in `prepareBidUserOp` with `No cached gas estimate found for order <id>`, and the
order was never evaluated again. Seen on a BSC order confirmed 79 s after its evaluation.

## Cache lifetime

- Every `CacheService` entry keyed by an order (gas estimate, bid plans, filler outputs and inputs,
  funding prepends, partial-fill and fee-check flags, matched limit orders, swap operations, pair
  classifications) stores its own `expiresAt`, set when it is written.
- The default lifetime, `CACHE_EXPIRY_MS`, is 300 s.
- While an order waits for its source confirmations, `handleNewOrder` registers how long they take
  with `CacheService.extendForConfirmations(orderId, waitMs)`. The wait is the required confirmations
  times the source chain's block time from `ContractInteractionService.blockTimeSeconds`, measured
  once at startup. Entries written in that time live the default plus the wait.
- The wait is released once the evaluation and the confirmations are both done. Entries keep the
  expiry they were written with; later entries get the default.
- With no block time to be had, the wait is 0 and the order keeps the default lifetime.
- A pinned order's entries are judged as of the pin, as before: live then means live for the whole
  bid loop.

## Re-evaluation

`handleNewOrder` also hands `executeOrder` a re-evaluation of the order (`evaluateOrder` with the
same `canFill` answers). Confirmations slower than the block time predicted, or a long chain-queue
wait, can still outlast an evaluation. So when the queued task starts, it pins the order and checks
the gas estimate: the first thing an evaluation caches, and what every bid signs.

- If the gas estimate is gone, the order is evaluated again before any bid goes out, and the bids use
  the strategy and plans that evaluation chooses.
- A re-evaluation that passes on the order sends no bid, and reports why as any evaluation does
  (`orderSkipped`). The expired evaluation is never used.
- An evaluation still live when the task starts is used as is, so every bid on an order is priced by
  a single evaluation.

`@hyperbridge/simplex` and `simplex-desktop` are 0.18.3.
