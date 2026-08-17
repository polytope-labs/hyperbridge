# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/indexer`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-14 — Cumulative seed derives from daily rows, and a failed seed skips only the gateway update (#1085)

Chosen: `seedAggregateVolume` scans only `DailyVolumeUSD` and derives the aggregate's cumulative record from the per-day sums (`lastUpdatedAt` from their max). The alternative — summing the component `CumulativeVolumeUSD` rows — was the original implementation and was dropped after review: `updateCumulativeVolume` skips same-timestamp updates per record, so a chain-wide aggregate's cumulative drops the second of any two same-block fills (even by different fillers), while per-filler cumulatives only collide within one filler. Seeding from summed filler cumulatives therefore bakes in the equality "FILLED cumulative equals the sum of FILLER cumulatives", which the guard breaks from the first multi-filler block onward. Daily rows have no such guard, count every fill, and are the series the aggregate is paired with. The forward divergence itself is accepted, not fixed — fixing it means removing the cumulative guard, which would change every existing volume series — and is pinned by a test.

Also chosen: the seed call in `updateOrderStatus` has its own try/catch. A store error during the scan must not swallow the fill's status, points, and user activity (the handler's try/catch is around all of it). On failure, the gateway `updateVolume` call is skipped too, deliberately: writing it would create the cumulative record that doubles as the seed's done-marker, permanently preventing the backfill. Skipping leaves the marker absent so the next fill retries the seed, and the retry recovers the skipped fill's volume because it sums the filler daily rows, which include it. Nothing is lost or double-counted in either outcome.

Accepted tradeoff, noted for future readers: the scan uses `getByFields([], ...)` with an empty filter, which returns zero rows with no diagnostic signal if something is wrong upstream (the reason `PendingStatusService` moved off empty-filter reads). Acceptable here because the seed runs once per chain per deployment and a wrongly-empty result degrades to a zero seed plus correct forward counting.

## 2026-08-14 — Gateway volume seeding uses the aggregate cumulative record as its own done-marker (#1085)

Chosen: `VolumeService.seedAggregateVolume` runs on every fill but returns immediately when the aggregate's `CumulativeVolumeUSD` record exists; the record itself is the marker. Seeding runs lazily on the first fill per chain after deploy, before that fill's own volume updates.

Alternatives considered: a dedicated migration-marker entity (extra schema and codegen for one boolean, and a marker that can drift from the data it describes); a block handler or one-off script outside event flow (SubQuery has no migration hook, and a script against the store bypasses block-atomic writes).

Why this won: no schema change, per-chain by construction, and correct in both deployment modes with no configuration — on a resumed database the first fill seeds the full filler history, on a fresh from-genesis reindex the first fill sees no history and seeds a zero marker. Two invariants make the lazy placement safe and are worth preserving: the seed must run before the triggering fill's own `updateVolume` calls (otherwise that fill's filler record is summed and then added again), and the seeded cumulative record must carry the components' max `lastUpdatedAt` rather than the current timestamp (otherwise `updateCumulativeVolume`'s same-timestamp guard drops the triggering fill). Enumeration pages the whole table with `getByFields([], ...)` and filters IDs in code because the volume entities have no filterable columns — acceptable because it runs once per chain per deployment and the tables are small (series count, not event count).

## 2026-08-13 — Gateway daily volume reuses `DailyVolumeUSD` instead of a new indexed entity (#1085)

Chosen: record gateway-level filled volume with base ID `IntentGatewayV3.FILLED` through the existing `VolumeService.updateVolume`, landing in the same `DailyVolumeUSD` / `CumulativeVolumeUSD` entities as the per-filler and per-user records.

Alternative considered: a new `DailyIntentGatewayVolumeUSD` entity keyed `chain-volumeType-date`, written inside `IntentGatewayV3Service.recordOrderVolume` next to `CumulativeIntentGatewayVolumeUSD`. That would give typed, indexed `chain` and `date` columns (the `DailyVolumeUSD` ID is an opaque string with no filterable columns), following the pattern `BandwidthAppDailyConsumption` uses.

Why the reuse won: the issue explicitly asked for the entry to appear in the existing `dailyVolumeUSDs` query alongside the FILLER and USER rows, it needs no schema change or codegen, and it exactly parallels the existing filler-independent `IntentGatewayV3.USER` record. If consumers later need to filter or sort daily gateway volume server-side, revisit the dedicated-entity alternative.

## 2026-08-13 — Gateway volume recorded at the same call site as filler volume, not the unconditional handler site (#1085)

Chosen: the new `IntentGatewayV3.FILLED` update sits directly next to the `IntentGatewayV3.FILLER.<address>` update inside `updateOrderStatus`, sharing its `getOutputValuesUSD` pricing.

Alternative considered: `orderFilledV3.event.handler.ts` already calls `recordOrderVolume("FILLED", ...)` unconditionally, even when the fill is indexed before its `OrderPlaced`. Recording there would not miss fill-before-place races.

Why the shared site won: it guarantees the invariant that the gateway entry equals the sum of the per-filler entries for a given chain and day, because both come from the same pricing call on the same code path. `recordOrderVolume` prices tokens itself and skips tokens with no known price, so its totals can disagree with the filler totals. The cost is inheriting a known gap: when a fill arrives before its `OrderPlaced`, `updateOrderStatus` stores `PendingStatusMetadata` and returns early, and the later replay (`flushPendingStatuses`) restores status only, never volume. The per-filler records already under-count that case; the gateway record under-counts it identically, which keeps the two consistent.

Known pre-existing quirk (not changed): `updateCumulativeVolume` skips the addition when `lastUpdatedAt` equals the incoming timestamp, so two fills in the same block increment daily volume twice but cumulative volume once. Left as is; changing it would affect every existing volume series.
