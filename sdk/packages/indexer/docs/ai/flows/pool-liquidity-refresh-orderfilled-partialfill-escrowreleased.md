# Pool liquidity refresh (OrderFilled, PartialFill, EscrowReleased, vault Deposit/Withdraw/Transfer)

Verified 2026-09-08 by unit tests against a mocked store (`inventoryReading.service.test.ts` for the EVM half,
`liquidityPoolFold.service.test.ts` for the Hyperbridge half); the store behaviour the split rests on was read in
`@subql/node-core` 19.3.1 and the forked substrate node.

The snapshot flow measures a pool's depth once per bid window. This flow keeps it honest in between, when
fills have spent some of the inventory it is a sum of. It is split across two kinds of node because each
SubQuery node process serves `Entity.get` from a private cache that no other process's write invalidates and
flushes whole rows with no locking: the pool family is therefore written by the Hyperbridge node only, and the
EVM nodes publish readings for it to fold.

**EVM side — publish (`src/services/inventoryReading.service.ts`)**

1. Order fills, partial fills, escrow releases and vault capital movements reach it. Inventory publication is
   best-effort — stale depth is recoverable. Required vault principal reads still propagate failures:
   - `handleOrderFilledEventV3` and `handlePartialFilledEventV3` call `IntentGatewayV3Service.publishInventoryAfterFill`,
     which loads the order row for its **source** chain (a fill carries the inputs' addresses but not the chain they
     live on), resolves the pools with `poolsForFill`, and calls `publishPoolInventory`. No order row, or no
     registry-tracked pair, means nothing to publish — the common case, and what keeps this off most fills' path.
   - `handleEscrowReleasedEventV3` (source chain) calls `publishInventoryAfterEscrowRelease`: the solver was just paid
     the order's inputs back, so its inventory there ROSE. The event names no filler, so the handler first reads
     the gateway's `_filled(commitment)` at that block.
   - `YieldVaultService.recordLedger` (vault `Deposit`/`Withdraw` and each tracked side of an ordinary
     share `Transfer`) ends with the same call for (chain, lp, underlying token), after its own
     known-solver gate and duplicate-log guard. Mint/burn, self and zero-share transfers are excluded.
   The last two name a solver and a token but no pool, so they enter through `publishProviderInventory`.
2. Both entry points select `PoolBidder` rows of **this chain only** with field queries (which go to Postgres;
   `get` would serve the process cache), and collapse them to distinct (solver, output token) targets. A target
   whose rows were all sampled (`lastUpdatedAt`) or refreshed (`refreshedAt`) after the event is skipped — that
   reading already saw what the event moved, and during a resync this is true of every replayed event, so
   backfilling costs no RPC. A target whose token the registry no longer tracks is skipped with a warning.
3. Each target's inventory is read pinned to the event's block on this chain: wallet ERC-20 plus ERC-4626
   `maxWithdraw` (`getTotalSolverBalance`) PLUS the Uniswap V4 positions the solver declared, each read on-chain,
   owner-checked, and valued in the output token. **If any read fails, or the chain has no configured RPC,
   nothing is published**: a failed read looks exactly like a zero balance, and a zero reading drops the bidder.
4. One `SolverInventoryReading` row per target is written, id `{chain}-{token}-{solver}`, holding the raw balance,
   the pinned block, the event time as `observedAt`, and the trigger. Nothing else is written here — not the
   pool, not the chain row, not the bidder, not the balance series.

**Hyperbridge side — fold (`foldInventoryReadings` in `src/services/liquidityPool.service.ts`)**

5. `handleInventoryFold` (`src/handlers/events/liquidity/inventoryFold.block.handler.ts`) runs on every Hyperbridge
   block, registered inside the `enableLiquidityIndexing` block of the substrate manifest template so only the
   Hyperbridge node runs it.
6. It pages the whole reading table (the store has no range operators) and drops readings an in-process memo has
   already folded at that observation time. A quiet block ends here after one page.
7. Per chain with new readings, it loads every `PoolBidder` row of that chain and groups them by pool. A row takes
   a reading when provider, chain and output token match (case-insensitively) and the reading's `observedAt` is
   later than both the row's `lastUpdatedAt` and its `refreshedAt`. The raw balance is normalised to 1e18 through
   the registry; a zero removes the row (every row is a bidder with capacity); otherwise `liquidity` and
   `refreshedAt` are written. Readings older than the row's snapshot — a lagging or resyncing EVM node's — are
   ignored, which is what makes the fold safe to repeat.
8. Every (pool, chain) that changed has its `PoolChainLiquidity` depth/bidCount/unrestricted slice and its
   `PoolRoute` rows recomputed from the surviving bidder rows (`republishChainRows`), with the direction set
   taken from before the removals so a direction whose bidders all vanished is zeroed rather than skipped. Routes
   are never *created* here: declarations only come from bids, so the surviving set can only shrink.
9. Each pool touched is re-merged into `sellDepth`/`buyDepth` through the same `mergeChainRowsIntoPool` the
   snapshot writer uses, with the fold's own block as the staleness reference — it is a Hyperbridge block, the
   unit the rows are stamped in.
10. Each reading that applied to at least one row extends `LiquidityProviderBalanceV2` with its raw balance, keyed
    by the fold's Hyperbridge block with the reading's `observedAt` as `snapshotTime`. A zero balance is not a row,
    matching the sweep; an existing row for that key is only ever raised, never lowered.
11. Nothing here writes `lastUpdatedBlock` or `lastUpdatedAt`, and nothing re-derives a rate. A pool's merged
    rate can still move, because the per-chain samples are depth-weighted and the depths just changed.

Store facts this depends on: a node sees only rows whose historical range contains its own current block time,
so an EVM node ahead of Hyperbridge writes readings the fold sees once Hyperbridge catches up; the mainnet
template flushes the store asynchronously every five seconds, which adds to that lag; and every field used in a
`getByFields` filter must carry `@index` (`chain`, `provider`, `tokenAddress` and `observedAt` on the reading do).

Where the V4 positions come from: `handlePhantomOrderPrices` calls `recordDeclaredPositions`
(`src/services/solverPositions.service.ts`), which writes one `SolverV4Positions` row per bidding solver — keyed
by the solver's address, holding the tokenIds `aggregatePhantomBids` verified and the chain the bid was for. A
bid is the only place a position is ever named. The EVM-side publication reads that row through a field query on
its provider link (`declaredV4Positions`) — it is written on another node, so `get` would serve a stale cached
copy, and the id itself is only indexed in historical mode — and a row recorded on another chain reads as none. A solver that bids without declaring has its row emptied; one that
skips a window keeps it. One row per solver assumes one V4 chain, and the writer warns rather than overwriting a
row from a different one.
