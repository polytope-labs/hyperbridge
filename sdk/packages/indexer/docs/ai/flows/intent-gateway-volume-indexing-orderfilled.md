# Intent gateway volume indexing (OrderFilled, PartialFill)

The indexer is a SubQuery project: per-network YAML files in `src/configs/` bind contract addresses and events to handler functions, and generated entity models in `src/configs/src/types/` persist via the SubQuery global `store`.

1. An `OrderFilled` log from the IntentGatewayV3 contract triggers `handleOrderFilledEventV3` in `src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`. It decodes the log and calls, in order:

    - `IntentGatewayV3Service.recordFill(...)` — the fill rows, the order's cumulative `filled` totals, and the per-fill volume and points described below.
    - `IntentGatewayV3Service.updateOrderStatus(commitment, FILLED, ..., filler)` — order status, user activity and referrer points.
    - `discoverSolverFromFill(...)` — queues the filler for solver-inventory tracking (see the solver inventory flow). Store-only and unguarded: only the store can fail it, so the block retries rather than losing the discovery.
    - `IntentGatewayV3Service.recordOrderVolume("FILLED", outputTokens, timestamp)`, in its own try/catch — a separate, unconditional cumulative volume path (see the parallel-paths note).

    A `PartialFill` log triggers `handlePartialFilledEventV3`, which calls `recordPartialFill` the same way and also calls `discoverSolverFromFill`. It updates no status and does not call `recordOrderVolume`.

2. `recordFill` and `recordPartialFill` (`src/services/intentGatewayV3.service.ts`) key the fill by `{transactionHash}.{logIndex}`. If that row already exists the log is a replay, and the call returns before counting anything. Otherwise they write the fill rows, add the outputs to the order's `IOrderV3OutputAsset.filled`, and call `awardFillRewards`.

    - `awardFillRewards` prices the fill's non-zero outputs with `getOutputValuesUSD` (unknown tokens price as zero). A zero total records nothing. With partial fills, several solvers can fill one order, so each is credited for its own slice.
    - Volume is credited even when the order is not indexed yet (the fill arrived before `OrderPlaced`, possible across chains). Points are not: `awardFillPoints` needs the order row and returns without it. When `OrderPlaced` arrives, `backfillEarlyFills` replays those fills into `filled` and filler points. It does not replay volume, which was already credited.
    - `updateOrderStatus` on a missing order still stores a `PendingStatusMetadata` row and returns early. `flushPendingStatuses` later replays only the status, so the user-activity and referrer effects do not happen for that fill.

3. Volume recording first calls `VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")` — a one-time, per-chain initialization that backfills the gateway-level series from the already-indexed per-filler daily records, deriving the gateway cumulative from the same per-day sums (it no-ops once the gateway cumulative record exists, which is its marker). It must stay ahead of the updates below; reordering it after them double-counts the current fill. The seed has its own try/catch: on failure the fill's per-filler volume and points still proceed, and only the gateway volume update below is skipped — that leaves the marker uncreated, so the next fill retries the seed and recovers the skipped fill from the filler daily rows. Then come two `VolumeService.updateVolume` calls with the fill's USD total and timestamp, differing only in base ID:

    - `IntentGatewayV3.FILLER.<fillerAddress>` — per-filler series.
    - `IntentGatewayV3.FILLED` — gateway-level, filler-independent series (issue #1085).
      (Order placement, elsewhere in the same service, records the analogous user-side series with base ID `IntentGatewayV3.USER`.)

4. `VolumeService.updateVolume` (`src/services/volume.service.ts`) fans out to two upserts, both scoping the ID by chain: `getChainTypeId` appends the host state machine (for example `EVM-8453`) resolved from the SubQuery global `chainId`.
    - `updateCumulativeVolume` upserts `CumulativeVolumeUSD` with ID `<baseId>.<chain>`. It skips the addition when the record's `lastUpdatedAt` equals the incoming timestamp, so a second fill in the same block does not increment the cumulative counter. This guard fires per record: the chain-wide `IntentGatewayV3.FILLED` cumulative collides on any two same-block fills, even by different fillers, so it can lag the sum of the per-filler cumulatives; the daily series counts every fill and stays exact.
    - `updateDailyVolume` upserts `DailyVolumeUSD` with ID `<baseId>.<chain>.<YYYY-MM-DD>` (UTC day bucket). It has no same-timestamp guard, so every call increments the daily counter.
    - USD amounts are stored as bigints scaled by 1e18 (`toScaledUsd`).

Parallel paths that look similar but are not the same: `recordOrderVolume` (step 1) writes `IntentGatewayTokenVolume` and `CumulativeIntentGatewayVolumeUSD` (IDs keyed `chain-token-volumeType` / `chain-volumeType`). It does its own token pricing and skips tokens with no known price, while the per-fill path prices unknown tokens as zero through `getOutputValuesUSD`; their USD totals can therefore differ for the same fill. Do not expect `CumulativeIntentGatewayVolumeUSD` for FILLED to equal `CumulativeVolumeUSD` for `IntentGatewayV3.FILLED`: they also diverge on partially filled orders (`recordOrderVolume` sees only the completing fill's outputs) and on same-block fills (only the `VolumeService` cumulative counter deduplicates).
