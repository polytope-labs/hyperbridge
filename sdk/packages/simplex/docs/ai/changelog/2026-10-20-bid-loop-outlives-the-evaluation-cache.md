# 2026-10-20 — Every bid on an order goes out, however long the earlier ones take

`IntentFiller.executeOrder` sends one bid per cached `BidPlan`, in turn. Each is signed by
`ContractInteractionService.prepareBidUserOp` with the gas estimate `estimateGasFillPost` cached
when the order was evaluated, and `CacheService` entries expire after `CACHE_EXPIRY_MS` (60 s). A
`place_bid` can sit in Hyperbridge's pool for 20 s or more, so on an order with several plans the
estimate expired partway through the loop and the remaining bids failed with
`No cached gas estimate found for order <id>`. `EventMonitor` drops repeat order ids, so the order
was never evaluated again and those limit orders never bid (simplex testnet swaps run
39275362211, scenario `multi-leg-same-input`: TIMEOUT with 2 fills).

## Behavior

The bid loop pins the order before it reads the plans. `CacheService.pinOrder(orderId)` returns the
release, which `executeOrder` calls when the queued task settles, however it ends. While the order
is pinned, every cache entry keyed by its id (gas estimate, bid plans, filler outputs, inputs and
budget, funding prepends, the partial-fill and fee-check-waived flags) is aged as of the pin rather
than now, and the stale-data sweep leaves it alone.

- An entry fresh when the loop began stays fresh until the loop ends, and so does each plan's
  outputs, flags and funding calls written during it.
- An entry that had already expired, for example while the order waited in the chain queue, stays
  expired: the loop sends nothing, as before.
- Once released, the entries expire on the usual clock.

Later bids are not re-estimated: each signs the gas cost its payout was priced against.

`src/tests/core/bid-loop-cache-expiry.test.ts` covers both cases and runs in `test:unit`.
