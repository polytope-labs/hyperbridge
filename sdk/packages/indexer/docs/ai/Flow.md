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

3. Volume recording is two `VolumeService.updateVolume` calls with the same USD total and timestamp, differing only in base ID:
   - `IntentGatewayV3.FILLER.<fillerAddress>` — per-filler series.
   - `IntentGatewayV3.FILLED` — gateway-level, filler-independent series (issue #1085).
   (Order placement, elsewhere in the same service, records the analogous user-side series with base ID `IntentGatewayV3.USER`.)

4. `VolumeService.updateVolume` (`src/services/volume.service.ts`) fans out to two upserts, both scoping the ID by chain: `getChainTypeId` appends the host state machine (for example `EVM-8453`) resolved from the SubQuery global `chainId`.
   - `updateCumulativeVolume` upserts `CumulativeVolumeUSD` with ID `<baseId>.<chain>`. It skips the addition when the record's `lastUpdatedAt` equals the incoming timestamp, so a second fill in the same block does not increment the cumulative counter.
   - `updateDailyVolume` upserts `DailyVolumeUSD` with ID `<baseId>.<chain>.<YYYY-MM-DD>` (UTC day bucket). It has no same-timestamp guard, so every call increments the daily counter.
   - USD amounts are stored as bigints scaled by 1e18 (`toScaledUsd`).

Parallel paths that look similar but are not the same: `recordOrderVolume` (step 1) writes `IntentGatewayTokenVolume` and `CumulativeIntentGatewayVolumeUSD` (IDs keyed `chain-token-volumeType` / `chain-volumeType`). It does its own token pricing and skips tokens with no known price, while the `updateOrderStatus` path prices unknown tokens as zero through `getOutputValuesUSD`; their USD totals can therefore differ for the same fill. Do not expect `CumulativeIntentGatewayVolumeUSD` for FILLED to equal `CumulativeVolumeUSD` for `IntentGatewayV3.FILLED`: they also diverge on fill-before-place races (only `recordOrderVolume` runs) and same-block fills (only the `VolumeService` cumulative counter deduplicates).
