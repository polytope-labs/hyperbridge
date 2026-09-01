# Flow

AI-maintained map of how code paths in `sdk/packages/indexer` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

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

5. Back in the handler, a third independent try/catch calls `IntentGatewayV3Service.refreshPoolLiquidityAfterFill`. Unlike the two above it is not a volume path: it re-reads the balances behind the pools this fill traded through (see the pool liquidity refresh flow below). Like `recordOrderVolume` it runs whether or not the order is indexed yet — but it needs the order row for the source chain, so a fill-before-place race resolves no pool and it returns immediately.

Parallel paths that look similar but are not the same: `recordOrderVolume` (step 1) writes `IntentGatewayTokenVolume` and `CumulativeIntentGatewayVolumeUSD` (IDs keyed `chain-token-volumeType` / `chain-volumeType`). It does its own token pricing and skips tokens with no known price, while the `updateOrderStatus` path prices unknown tokens as zero through `getOutputValuesUSD`; their USD totals can therefore differ for the same fill. Do not expect `CumulativeIntentGatewayVolumeUSD` for FILLED to equal `CumulativeVolumeUSD` for `IntentGatewayV3.FILLED`: they also diverge on fill-before-place races (only `recordOrderVolume` runs) and same-block fills (only the `VolumeService` cumulative counter deduplicates).

## Phantom price snapshot to pool rates (PhantomBidWindowExhausted)

Verified 2026-08-19 against live mainnet data.

1. `PhantomBidWindowExhausted` on Hyperbridge triggers `handlePhantomOrderPrices` (`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`). It loads the `PhantomOrderV2` and its registered `PhantomOrderLeg` rows, then calls `aggregatePhantomBids` from the SDK, which fetches every bid for the commitment, verifies each one (solver signature over the userOp hash plus an EIP-7702 delegation check), and reduces them per leg.

2. Per leg, a solver's quote is weighted by **its balance of that leg's OUTPUT token on the destination chain** — the inventory that actually backs the leg. Zero-weight quotes are dropped entirely, not down-weighted: they never reach the median, `bidCount`, or the bidder list. A leg where no bidder holds the output token is absent from the result, exactly as if nobody quoted it.

3. The leg's price is `weightedMedian` of the backed quotes — a **selection**, not a blend. It returns one bidder's exact integer, so a solver holding over half the leg's weight sets the published price verbatim, and the result can never be a value nobody quoted. `lowestPrice` and `highestPrice` are deliberately overwritten with the median so consumers cannot read an outlier bid as a tradeable bound.

4. `updateLiquidityPools` (`src/services/liquidityPool.service.ts`) turns those per-leg medians into pool rows. `resolvePoolLeg` maps a leg's tokens to a pool id and direction via the token registry, and the sample's rate is

   ```
   medianPrice * 10 ** (18 - outDecimals) * 10 ** inDecimals / standardAmount
   ```

   i.e. the quote renormalized from the probe size back to one whole input token. This holds for any standard amount the pallet configures; it collapses to `medianPrice * scale` when the probe is exactly one unit. Multiplications happen before the division, so only the last step truncates, by under one unit of 1e18 and downward.

5. Chain rows (`PoolChainLiquidity`, one per pool/chain/direction) are merged into the pool's single `sellRate`/`buyRate` by `weightedRate` — a depth-weighted **mean**, which unlike the median in step 3 does produce values no filler quoted. Samples older than `MAX_SAMPLE_AGE_BLOCKS` are excluded unless every sample is stale.

Precision note: a leg's quoted output integer *is* the price, to whatever resolution the output token's decimals allow. cNGN into 6-decimal USDC quotes ~715 base units, so the grid is 1/715 = 0.14% and the filler's floor rounding costs up to one full step. Chains whose output token has 18 decimals carry full precision on the same leg — which is why EVM-56 publishes `716845878136200` where Base publishes a bare `715`. The fix is a larger `standardAmount`, which step 4 now supports; see Decisions.md for why the filler's flooring must stay.

## Pool liquidity refresh (OrderFilled)

Verified 2026-09-01 by unit test against a mocked store; the RPC read itself is the same
`memoizedSolverBalance` the snapshot flow above uses.

The snapshot flow measures a pool's depth once per bid window. This flow keeps it honest in between, when
fills have spent some of the inventory it is a sum of.

Four events reach this flow, each in its own try/catch — they read external RPCs, and stale depth is
recoverable, so a failure must never stall indexing:

1. `handleOrderFilledEventV3` and `handlePartialFilledEventV3` call
   `IntentGatewayV3Service.refreshPoolLiquidityAfterFill`, which resolves the pools from the order's two sides
   (`poolsForFill`) and refreshes every LP backing them. `PartialFill` only ever comes from the same-chain path
   (`IntrinsicIntents`); cross-chain fills are all-or-nothing and emit only `OrderFilled` (`ExtrinsicIntents`).
1b. `handleEscrowReleasedEventV3` (source chain) calls `refreshLiquidityAfterEscrowRelease`: the solver was just
   paid the order's inputs back, so its inventory there ROSE. The event names no filler, so the handler first
   reads the gateway's `_filled(commitment)` at that block — `_withdraw` writes the beneficiary in the same call
   that emits the event.
1c. `YieldVaultService.recordLedger` (vault `Deposit`/`Withdraw`) ends with a refresh for (chain, lp, underlying
   token), after its own known-solver gate and duplicate-log guard. An LP's own deposit only shifts inventory
   between the raw and vault halves of one total; the total moves when the counterparty is someone else, and no
   order event reports that.
   These last two name a solver and a token but no pool, so they enter through `refreshProviderLiquidity`
   instead — same core, different selection.
2. That method loads the order row for its **source** chain (a fill event carries the inputs' addresses but
   not the chain they live on) and calls `poolsForFill`, which pairs the input symbols on the source chain
   with the output symbols on the destination chain through the same token registry `resolvePoolLeg` uses.
   No order row, or no registry-tracked pair, means no pool and an immediate return — which is the common
   case, and is what keeps this off the critical path of most fills.
3. `refreshPoolLiquidity` (`src/services/liquidityPool.service.ts`) then, per pool:
   - **skips the pool entirely if `pool.lastUpdatedAt` is newer than the fill.** That snapshot already read
     balances this fill had moved. During a resync this is true of every replayed fill, so backfilling costs
     no RPC at all.
   - reads every `PoolBidder` row of the pool (all chains, paged to exhaustion) and groups them by chain.
   - per chain, re-reads each bidder's inventory of that row's `outputToken` and scales it to 1e18. Inventory
     is the sweep's definition — wallet ERC-20 plus ERC-4626 `maxWithdraw` (`getTotalSolverBalance`) — PLUS the
     solver's `LiquidityProviderV4Position` rows, each re-read and valued in that output token. Reads are pinned
     to the triggering event's block on its own chain, so a replay sees what the event left behind; other chains
     read at the head, where that block number would mean nothing. **If any read fails, or the chain has no
     configured RPC, the whole chain is abandoned untouched** — a failed read looks exactly like a zero balance,
     so a partial write would report the unread bidders as departed.
   - a position that no longer exists, or is found under another owner, loses its row: it is not this solver's
     inventory any more, and the row would otherwise be re-valued on every later refresh.
   - writes the survivors: a row whose balance is now zero is removed (every row is a bidder with capacity),
     then the chain's `PoolChainLiquidity` depth/bidCount/unrestricted slice and its `PoolRoute` rows are
     recomputed by re-reading **every** stored bidder row of that (pool, chain) — not just the ones re-read, so a
     provider-scoped refresh leaves the other bidders contributing what they were. Routes are never *created* here: declarations only come from
     bids, so the surviving set can only shrink.
   - re-merges the pool's chain rows into `sellDepth`/`buyDepth` through the same `mergeChainRowsIntoPool`
     the snapshot writer uses, with the freshest row's block as the staleness reference (the fill's own EVM
     block number is not comparable with the Hyperbridge blocks these rows are stamped with).
4. Each chain that was written also extends `LiquidityProviderBalanceV2` with the raw balances just read,
   keyed by Hyperbridge's head block (`chain_getHeader` on the configured Hyperbridge node, memoized per
   indexed block). A zero balance is not a row, matching the sweep; an existing row for that key is only ever
   raised, never lowered, because a refresh cannot see declared Uniswap V4 positions and the sweep's rule is
   that the larger of two readings is the complete one. A null head (no Hyperbridge RPC, or unreachable) skips
   the row and nothing else.
5. Nothing here writes `lastUpdatedBlock` or `lastUpdatedAt`, and nothing re-derives a rate. A pool's merged
   rate can still move, because the per-chain samples are depth-weighted and the depths just changed.

Where the V4 positions come from: `handlePhantomOrderPrices` writes a `LiquidityProviderV4Position` row per
(chain, tokenId) from the declarations `aggregatePhantomBids` verified — a bid is the only place a position is
ever named. Rows are replaced wholesale for the solvers that bid (a declaration is per bid); a solver that
skipped the window keeps what it last declared.

## Phantom bid calldata decoding (`extractFillDataVm2`)

Verified by executing the function against both shapes; the selector check is what makes the two-interface
attempt necessary.

1. `handlePhantomOrderPrices.handler.ts` injects `extractFillDataVm2` into `aggregatePhantomBids` as
   `extractFill`. The SDK's own `extractFillData` is not used here: it decodes with viem, whose byte handling
   throws inside SubQuery's VM2 sandbox.
2. A bid's `callData` is the solver account's ERC-7821 `execute(mode, executionData)` batch. The batch is decoded,
   and each call whose `target` is the gateway is a `fillOrder` candidate.
3. `decodeFillOrderEither` tries the v2 interface (`FILL_ORDER_ABI`, with `validUntil`, selector `0xa5470064`)
   and then the v1 one (`FILL_ORDER_V1_ABI`, selector `0x5cfb1ea5`). ethers validates the selector before
   decoding, so exactly one can match and there is no payload that could be mis-decoded as the other shape.
   Which one a bid carries depends on the gateway it targets — solvers encode for the deployment they bid
   against, and gateways predating `validUntil` take v1.
4. A call matching neither shape is skipped, and if no call in the batch decodes the function returns null.
5. **Null is dropped silently upstream** — `aggregatePhantomBids` does `if (!fillData) continue` with no log. A
   decoding regression therefore shows up as missing pool rates rather than as an error, which is why the shapes
   are covered by tests rather than left to runtime observation.
