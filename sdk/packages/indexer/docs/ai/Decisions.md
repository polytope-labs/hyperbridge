# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/indexer`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

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
