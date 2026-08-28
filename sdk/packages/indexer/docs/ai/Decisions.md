# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/indexer`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-28 — The VM2 decoder tries both `fillOrder` shapes, and the new test avoids the SDK root import

Alternatives to trying both shapes:

- **Track the gateway's version and pick one.** That is what `getFillOptionsVersion` does for *encoding*, where
  you must choose. Decoding has no such constraint: the selectors differ, so attempting both is unambiguous and
  needs no chain state, no RPC read, and no cache — all of which the SubQuery sandbox makes awkward.
- **Decode by selector lookup.** Equivalent in effect, more code, and it would duplicate the selector constants
  that the ABIs already encode.

Trying v2 then v1 mirrors `decodeFillOrder` exactly, which is the point: the two implementations of this decode
diverged once already, and keeping them structurally identical is what makes the next divergence visible.

The regression test lives in its own file, `phantom-decode.fill.test.ts`, importing only the `intents-helpers`
sub-path. The sibling `phantom-decode.test.ts` also imports `@hyperbridge/sdk` root for `CryptoUtils`, which is
not available on the sub-path. Splitting keeps the new test on exactly the entry point the indexer uses at
runtime, so it exercises the same module graph the sandbox loads.

The jest `transformIgnorePatterns` addition names the ESM-only packages explicitly rather than transforming all of
`node_modules`. The broad form is slower and pulls unrelated packages through ts-jest; the explicit list fails
loudly (an unparsed `export`) if the SDK's dependency graph grows another ESM-only package, which is the right
failure mode — silence here is what let the tests stop running unnoticed.

## 2026-08-19 — The standard-amount check bounds plausibility instead of pinning one unit

Chosen: `resolvePoolLeg` accepts any standard amount within a plausibility window around one whole input token, and `updateLiquidityPools` renormalizes the rate by the leg's own standard amount. The pallet is then free to raise the probe size to buy quote precision without the indexer rescaling every published rate by that factor.

Alternative rejected — keep `standardAmount === 10 ** inputDecimals`. It made the rate math a single multiplication, but it is what blocked the precision fix: a leg's quoted output integer IS the price, and one whole token of a 6-decimal asset priced into another 6-decimal asset only affords ~3 significant digits.

Alternative rejected — accept any whole multiple of one unit and carry the multiple as a divisor. Simpler arithmetic, but it silently waves through the exact bug the old check caught: an 18-decimal amount read against a 6-decimal registry entry is a clean multiple (1e12 of them), so it would have been read as a trillion-token probe and published a rate off by 1e12. It also needlessly forbids a non-whole probe, which the renormalization prices correctly.

Alternative rejected — drop the check entirely. The standard amount is the denominator of every published rate and nothing else in the pipeline notices when it disagrees with the registry's decimals; the failure is silent and needs a human to spot feed drift. The window is deliberately wide enough that no plausible probe size trips it and narrow enough that every realistic decimals mismatch does.

Not changed, deliberately: the filler floors its quoted output (`computeLegPolicyOutput` in simplex). That truncation looks like a 0.14% pricing error at a one-unit probe and is tempting to "fix" by rounding to nearest — but it is load-bearing. Flooring keeps the published rate at or below the filler's true curve rate, which is what makes a quote built from the snapshot honourable; the SDK quoter derives `amountOut` from `medianPrice / standardAmount`, and the gateway's fully-filled check has zero tolerance, so a published rate even one base unit above the curve turns every order into a partial fill. Precision belongs to the probe size, not the rounding mode.

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
