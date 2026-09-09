# Flow

AI-maintained map of how code paths in `sdk/packages/indexer` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## Substrate schema migrations (verified 2026-09-04, against the forked node-core)

How a change to `src/configs/schema.graphql` reaches the database. All nodes share one `schema.graphql` and one
`--db-schema=app`; the substrate node runs the forked `polytopelabs/subql-node-substrate` image with
`--allow-schema-migration`, the EVM nodes run the stock `subql-node-ethereum` image.

1. On boot each node's main thread runs `StoreService.init` (node-core). With the flag off (the EVM nodes) it calls the
   schema migration with a `null` baseline, which only emits `CREATE TABLE IF NOT EXISTS` — existing tables and their
   columns are left untouched.
2. With the flag on (the substrate node), `init` reads the previously applied schema from the `appliedSchemaSDL`
   metadata key, rebuilds it into a baseline `GraphQLSchema`, and diffs it against the current one. Additive changes
   (add nullable column, table, index, enum value, relation) are ALTERed in place; the new schema is written back to
   `appliedSchemaSDL` in the same transaction as the DDL.
3. A destructive diff (a removed or retyped field, a removed entity — a retype surfaces as remove+add) is refused with a
   fatal error unless `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true`, because it would drop the column and its data.
4. Compose gates every EVM node behind the substrate node's `/ready` healthcheck, and `init` (including the migration)
   completes before `/ready`. So the substrate leader applies the whole shared schema's DDL before any EVM node starts;
   the EVM nodes' `CREATE TABLE IF NOT EXISTS` is then a no-op. There is no multi-writer race.

The engine that performs the ALTERs (`SchemaMigrationService`) already existed; the change was giving it a real baseline
on restart instead of `null`. Adding a field is therefore: edit `schema.graphql`, `pnpm build`, restart. No rename, no
data loss, no reindex.

## Intent gateway volume indexing (OrderFilled)

The indexer is a SubQuery project: per-network YAML files in `src/configs/` bind contract addresses and events to handler functions, and generated entity models in `src/configs/src/types/` persist via the SubQuery global `store`.

1. An `OrderFilled` log from the IntentGatewayV3 contract triggers `handleOrderFilledV3Event` in `src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`. It decodes the log, then makes two independent calls, each in its own try/catch:
   - `IntentGatewayV3Service.updateOrderStatus(commitment, FILLED, ..., filler)` — order status, points, user activity, and the per-fill volume records described below.
   - `IntentGatewayV3Service.recordOrderVolume("FILLED", outputTokens, timestamp)` — a separate, unconditional cumulative volume path (see the parallel-paths note).

2. `updateOrderStatus` (`src/services/intentGatewayV3.service.ts`) first loads the `OrderV3Placed` entity by commitment.
   - If the order is not indexed yet (fill event arrived before the `OrderPlaced` event, possible across chains), it stores a `PendingStatusMetadata` row and returns early. `flushPendingStatuses` later replays the status onto the order once `OrderPlaced` arrives, but it replays only the status: none of the volume, points, or user-activity effects below happen for that fill. Volume records therefore under-count fill-before-place races, deliberately and equally for filler and gateway records.
   - Otherwise it saves the new status and, when the status is FILLED and a filler address is present, gathers the order's output assets (`IOrderV3OutputAsset` rows keyed `commitment-output-N`), prices them with `getOutputValuesUSD` (unknown tokens price as zero), and records volume.

3. Volume recording first calls `VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")` — a one-time, per-chain initialization that backfills the gateway-level series from the already-indexed per-filler daily records, deriving the gateway cumulative from the same per-day sums (it no-ops once the gateway cumulative record exists, which is its marker). It must stay ahead of the updates below; reordering it after them double-counts the current fill. The seed has its own try/catch: on failure the fill's status, points, and user activity still proceed, and only the gateway volume update below is skipped — that leaves the marker uncreated, so the next fill retries the seed and recovers the skipped fill from the filler daily rows. Then come two `VolumeService.updateVolume` calls with the same USD total and timestamp, differing only in base ID:
   - `IntentGatewayV3.FILLER.<fillerAddress>` — per-filler series.
   - `IntentGatewayV3.FILLED` — gateway-level, filler-independent series (issue #1085).
   (Order placement, elsewhere in the same service, records the analogous user-side series with base ID `IntentGatewayV3.USER`.)

4. `VolumeService.updateVolume` (`src/services/volume.service.ts`) fans out to two upserts, both scoping the ID by chain: `getChainTypeId` appends the host state machine (for example `EVM-8453`) resolved from the SubQuery global `chainId`.
   - `updateCumulativeVolume` upserts `CumulativeVolumeUSD` with ID `<baseId>.<chain>`. It skips the addition when the record's `lastUpdatedAt` equals the incoming timestamp, so a second fill in the same block does not increment the cumulative counter. This guard fires per record: the chain-wide `IntentGatewayV3.FILLED` cumulative collides on any two same-block fills, even by different fillers, so it can lag the sum of the per-filler cumulatives; the daily series counts every fill and stays exact.
   - `updateDailyVolume` upserts `DailyVolumeUSD` with ID `<baseId>.<chain>.<YYYY-MM-DD>` (UTC day bucket). It has no same-timestamp guard, so every call increments the daily counter.
   - USD amounts are stored as bigints scaled by 1e18 (`toScaledUsd`).

5. Back in the handler, a third independent try/catch calls `IntentGatewayV3Service.publishInventoryAfterFill`. Unlike the two above it is not a volume path: it re-reads the balances behind the pools this fill traded through on this chain and publishes them for the Hyperbridge node to fold into the pool rows (see the pool liquidity refresh flow below). Like `recordOrderVolume` it runs whether or not the order is indexed yet — but it needs the order row for the source chain, so a fill-before-place race resolves no pool and it returns immediately.

Parallel paths that look similar but are not the same: `recordOrderVolume` (step 1) writes `IntentGatewayTokenVolume` and `CumulativeIntentGatewayVolumeUSD` (IDs keyed `chain-token-volumeType` / `chain-volumeType`). It does its own token pricing and skips tokens with no known price, while the `updateOrderStatus` path prices unknown tokens as zero through `getOutputValuesUSD`; their USD totals can therefore differ for the same fill. Do not expect `CumulativeIntentGatewayVolumeUSD` for FILLED to equal `CumulativeVolumeUSD` for `IntentGatewayV3.FILLED`: they also diverge on fill-before-place races (only `recordOrderVolume` runs) and same-block fills (only the `VolumeService` cumulative counter deduplicates).

## Phantom price snapshot to pool rates (PhantomBidWindowExhausted)

Verified 2026-08-19 against live mainnet data.

1. `PhantomBidWindowExhausted` on Hyperbridge triggers `handlePhantomOrderPrices` (`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`). It loads the `PhantomOrderV2` and its registered `PhantomOrderLeg` rows, then calls `aggregatePhantomBids` from the SDK, which fetches every bid for the commitment, verifies each one (solver signature over the userOp hash plus an EIP-7702 delegation check), and reduces them per leg.

   A bid's `paymasterAndData` arrives in one of two shapes, and the SDK's `decodePhantomBidPaymasterAndData` reads both before the declaration is used: the bare declaration blob (every bid until simplex moved to Permit2), or the 234-byte EntryPoint v0.8 payload for the Simplex paymaster's PERMIT2 mode with the declaration appended after the permit (a bid built on simplex's real-bid path since #1223). A sponsored bid with nothing appended counts as having declared nothing — null accepted sources, no positions — the same as an empty field. The solver signature covers the whole field in both shapes, so `recoverBidSignerVm2` is unchanged; `phantom-decode.test.ts` checks the ethers digest over the long payload matches viem's.

   The chain ids in a declaration are decoded in the SDK without `TextDecoder`. That matters here specifically: the handler runs inside SubQuery's vm2 sandbox, where `TextDecoder` is not defined and the `util` fallback rejects a sandbox-created `Uint8Array`, so a decoder reaching for it threw inside the per-bid try/catch of `aggregatePhantomBids` — logged as "Failed to process bid for price snapshot", bid dropped, run continues. That was the whole failure behind bids with a source-chain declaration vanishing from the snapshots (verified 2026-09-09 against the live bids and the deployed indexer's data); `phantom-decode.sandbox.test.ts` runs the shipped bundle inside vm2 to keep it from coming back. A chain with a phantom order but no `solverAccount` in `config-mainnet.json` is skipped with "No SolverAccount configured for chain" — Gnosis (EVM-100) was, until its entry was added.

2. Per leg, a solver's quote is weighted by **its balance of that leg's OUTPUT token on the destination chain** — the inventory that actually backs the leg. Zero-weight quotes are dropped entirely, not down-weighted: they never reach the median, `bidCount`, or the bidder list. A leg where no bidder holds the output token is absent from the result, exactly as if nobody quoted it.

3. The leg's price is `weightedMedian` of the backed quotes — a **selection**, not a blend. It returns one bidder's exact integer, so a solver holding over half the leg's weight sets the published price verbatim, and the result can never be a value nobody quoted. `lowestPrice` and `highestPrice` are deliberately overwritten with the median so consumers cannot read an outlier bid as a tradeable bound.

4. `updateLiquidityPools` (`src/services/liquidityPool.service.ts`) turns those per-leg medians into pool rows. `resolvePoolLeg` maps a leg's tokens to a pool id and direction via the token registry, and the sample's rate is

   ```
   medianPrice * 10 ** (18 - outDecimals) * 10 ** inDecimals / standardAmount
   ```

   i.e. the quote renormalized from the probe size back to one whole input token. This holds for any standard amount the pallet configures; it collapses to `medianPrice * scale` when the probe is exactly one unit. Multiplications happen before the division, so only the last step truncates, by under one unit of 1e18 and downward.

5. Chain rows (`PoolChainLiquidity`, one per pool/chain/direction) are merged into the pool's single `sellRate`/`buyRate` by `weightedRate` — a depth-weighted **mean**, which unlike the median in step 3 does produce values no filler quoted. Samples older than `MAX_SAMPLE_AGE_BLOCKS` are excluded unless every sample is stale.

Precision note: a leg's quoted output integer *is* the price, to whatever resolution the output token's decimals allow. cNGN into 6-decimal USDC quotes ~715 base units, so the grid is 1/715 = 0.14% and the filler's floor rounding costs up to one full step. Chains whose output token has 18 decimals carry full precision on the same leg — which is why EVM-56 publishes `716845878136200` where Base publishes a bare `715`. The fix is a larger `standardAmount`, which step 4 now supports; see Decisions.md for why the filler's flooring must stay.

## Pool liquidity refresh (OrderFilled, PartialFill, EscrowReleased, vault Deposit/Withdraw)

Verified 2026-09-08 by unit tests against a mocked store (`inventoryReading.service.test.ts` for the EVM half,
`liquidityPoolFold.service.test.ts` for the Hyperbridge half); the store behaviour the split rests on was read in
`@subql/node-core` 19.3.1 and the forked substrate node.

The snapshot flow measures a pool's depth once per bid window. This flow keeps it honest in between, when
fills have spent some of the inventory it is a sum of. It is split across two kinds of node because each
SubQuery node process serves `Entity.get` from a private cache that no other process's write invalidates and
flushes whole rows with no locking: the pool family is therefore written by the Hyperbridge node only, and the
EVM nodes publish readings for it to fold.

**EVM side — publish (`src/services/inventoryReading.service.ts`)**

1. Four events reach it, each in its own try/catch — they read external RPCs, and stale depth is recoverable, so a
   failure must never stall indexing:
   - `handleOrderFilledEventV3` and `handlePartialFilledEventV3` call `IntentGatewayV3Service.publishInventoryAfterFill`,
     which loads the order row for its **source** chain (a fill carries the inputs' addresses but not the chain they
     live on), resolves the pools with `poolsForFill`, and calls `publishPoolInventory`. No order row, or no
     registry-tracked pair, means nothing to publish — the common case, and what keeps this off most fills' path.
   - `handleEscrowReleasedEventV3` (source chain) calls `publishInventoryAfterEscrowRelease`: the solver was just paid
     the order's inputs back, so its inventory there ROSE. The event names no filler, so the handler first reads
     the gateway's `_filled(commitment)` at that block.
   - `YieldVaultService.recordLedger` (vault `Deposit`/`Withdraw`) ends with the same call for (chain, lp,
     underlying token), after its own known-solver gate and duplicate-log guard.
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

## Phantom bid calldata decoding (`extractFillDataVm2`)

Verified by executing the function against both shapes; the selector check is what makes the two-interface
attempt necessary.

1. `handlePhantomOrderPrices.handler.ts` injects `extractFillDataVm2` into `aggregatePhantomBids` as
   `extractFill`. The SDK's own `extractFillData` is not used here: it decodes with viem, whose byte handling
   throws inside SubQuery's VM2 sandbox.
2. A bid's `callData` is the solver account's ERC-7821 `execute(mode, executionData)` batch. The batch is decoded,
   and each call whose `target` is the gateway is a `fillOrder` candidate. The bid's sender must be
   EIP-7702-delegated to one of the chain's `SOLVER_ACCOUNT_ADDRESSES`; the list carries the current
   SolverAccount and, during a redeployment, the one it replaced. The bid's sender must be
   EIP-7702-delegated to one of the chain's `SOLVER_ACCOUNT_ADDRESSES`; the list carries the current
   SolverAccount and, during a redeployment, the one it replaced.
3. `decodeFillOrderEither` tries the v2 interface (`FILL_ORDER_ABI`, with `validUntil`, selector `0xa5470064`)
   and then the v1 one (`FILL_ORDER_V1_ABI`, selector `0x5cfb1ea5`). ethers validates the selector before
   decoding, so exactly one can match and there is no payload that could be mis-decoded as the other shape.
   Which one a bid carries depends on the gateway it targets — solvers encode for the deployment they bid
   against, and gateways predating `validUntil` take v1.
4. A call matching neither shape is skipped, and if no call in the batch decodes the function returns null.
5. **Null is dropped silently upstream** — `aggregatePhantomBids` does `if (!fillData) continue` with no log. A
   decoding regression therefore shows up as missing pool rates rather than as an error, which is why the shapes
   are covered by tests rather than left to runtime observation.
