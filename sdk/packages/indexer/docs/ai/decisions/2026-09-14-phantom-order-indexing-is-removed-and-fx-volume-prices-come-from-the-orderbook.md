# 2026-09-14 — Phantom-order indexing is removed outright, and FX volume prices come from the orderbook

**Chosen: delete the handlers, services, entities and flows, rather than leaving them registered-off.** Dormant
code nothing runs drifts from the SDK it imports (`aggregatePhantomBids`, the bid decoders). It would also keep the
pool family's single-writer constraints (#1214) binding on changes that no longer need them. Git history keeps
it.

This supersedes two earlier decisions:
- `2026-09-01-declared-v4-positions-live-in-one-row-per-solver-1159.md`: Uniswap V4 positions are not solver
  inventory anywhere any more.
- The pool-family parts of `2026-09-08-one-writer-per-pool-row-evm-nodes-publish-readings-the.md`. Its store facts
  still stand, and solver inventory follows them: one writer per row, and field queries for every cross-node read.

**Chosen: FX volume pricing goes through one function, `fetchOrderbookUsdPrice`, mocked to return no price.**
Two alternatives lost:
- **A fixed placeholder rate.** Cumulative USD volume is incremented and never recomputed, so a made-up rate would
  be permanent.
- **Keep pricing from the last `LiquidityPool` rows.** Nothing would update those rows again, so every FX token
  would be priced at whatever the final snapshot said, indefinitely.

Until the orderbook query replaces the mock, FX tokens' USD rollups are skipped with a warning, exactly as for
any token without a price.

**Chosen: the yield ledger stops treating a `LiquidityProvider` row as proof of a solver.** Its only writer is
gone. A stale table would count a solver that stopped bidding forever, and an empty one would count nobody.
On-chain delegation was already the fallback and is now the whole test, besides an existing position.

**Kept: `FillerBid` and `handleBidPlaced`.** `place_bid` accepts any order's commitment, so recording bids is not
phantom-specific.
