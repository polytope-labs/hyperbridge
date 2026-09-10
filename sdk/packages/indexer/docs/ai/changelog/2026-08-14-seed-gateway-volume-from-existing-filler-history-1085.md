# 2026-08-14 — Seed gateway volume from existing filler history (#1085)

On a resumed database the new `IntentGatewayV3.FILLED` series would have started from zero while the per-filler series already carried history. Added `VolumeService.seedAggregateVolume(aggregateBaseId, componentIdPrefix)`: a one-time, per-chain initialization that sums every existing `CumulativeVolumeUSD` and `DailyVolumeUSD` record matching the component prefix into the aggregate's cumulative record and per-day buckets. The aggregate's cumulative record doubles as the done-marker. Called from `updateOrderStatus` before the fill's own volume updates so the seed never counts the triggering fill, and the seeded record keeps the components' max `lastUpdatedAt` so the cumulative same-timestamp guard does not swallow that fill. Extended the `VolumeService` tests to cover seeding.

Files: `src/services/volume.service.ts`, `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/volume.service.test.ts`.
