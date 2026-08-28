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

## Order cancellation (OrderCancelled → EscrowRefunded)

Verified by reading the handler, the service, and the gateway's event docs; the two-event split is what makes the
status guard necessary.

1. `cancelOrder` on IntentGatewayV2 emits `OrderCancelled(commitment, canceller)` on the chain the cancellation is
   initiated from. This is *not* terminal.
2. The log triggers `handleOrderCancelledEventV3`
   (`src/handlers/events/intentGatewayV3/orderCancelledV3.event.handler.ts`), which resolves the block timestamp
   and calls `IntentGatewayV3Service.recordOrderCancellation`.
3. That method always writes an `IOrderV3Cancellation` row keyed `{transactionHash}.{logIndex}`, so repeat
   cancellations each get their own record. It then advances the order to `CANCELLED` **only if the order is
   currently `PLACED`** — see Decisions for why.
4. `EscrowRefunded` is the terminal event and moves the order to `REFUNDED` via the existing
   `handleEscrowRefundedEventV3`. Two timings:
   - **Same-chain cancel:** both logs are in one transaction, `OrderCancelled` at the lower `logIndex`
     (`_cancelSameChain` emits before `_withdraw`). The order passes through `CANCELLED` to `REFUNDED` in the same
     block.
   - **Cross-chain cancel:** `OrderCancelled` fires on the destination chain, and `EscrowRefunded` on the source
     chain only once the cancellation has travelled through Hyperbridge. These are separate datasources with no
     ordering guarantee, which is the case the `PLACED` guard exists for.
5. A cancellation initiated from the source side can emit `OrderCancelled` and never be followed by
   `EscrowRefunded` — the route re-emits on every call and only refunds when the GET response returns. An order
   resting at `CANCELLED` is therefore an expected steady state, not necessarily an indexing gap.
