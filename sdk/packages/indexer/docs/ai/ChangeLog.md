# ChangeLog

AI-maintained log of code changes in `sdk/packages/indexer`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog (there is none for the indexer; the published packages use changesets `CHANGELOG.md` files).

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-14 — Seed gateway volume from existing filler history (#1085)

On a resumed database the new `IntentGatewayV3.FILLED` series would have started from zero while the per-filler series already carried history. Added `VolumeService.seedAggregateVolume(aggregateBaseId, componentIdPrefix)`: a one-time, per-chain initialization that sums every existing `CumulativeVolumeUSD` and `DailyVolumeUSD` record matching the component prefix into the aggregate's cumulative record and per-day buckets. The aggregate's cumulative record doubles as the done-marker. Called from `updateOrderStatus` before the fill's own volume updates so the seed never counts the triggering fill, and the seeded record keeps the components' max `lastUpdatedAt` so the cumulative same-timestamp guard does not swallow that fill. Extended the `VolumeService` tests to cover seeding.

Files: `src/services/volume.service.ts`, `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/volume.service.test.ts`.

## 2026-08-13 — Gateway-level daily filled volume (#1085)

Daily filled volumes were only recorded per filler (`IntentGatewayV3.FILLER.<address>.<chain>.<date>`), so there was no single entry showing total volume filled through the intent gateway per chain per day. Added one `VolumeService.updateVolume` call with the filler-independent base ID `IntentGatewayV3.FILLED`, in the same FILLED branch of `updateOrderStatus` that records the per-filler volume. This produces `DailyVolumeUSD` records like `IntentGatewayV3.FILLED.EVM-8453.2026-07-30` and a `CumulativeVolumeUSD` record `IntentGatewayV3.FILLED.EVM-8453`, queryable from the existing `dailyVolumeUSDs` query. Also added the first unit tests for `VolumeService`.

Files: `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/volume.service.test.ts` (new).
