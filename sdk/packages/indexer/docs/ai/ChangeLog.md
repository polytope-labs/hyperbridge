# ChangeLog

AI-maintained log of code changes in `sdk/packages/indexer`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog (there is none for the indexer; the published packages use changesets `CHANGELOG.md` files).

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-28 — Index the gateway's OrderCancelled event

`09888bd1` added `OrderCancelled(bytes32 indexed commitment, address canceller)` to IntentGatewayV2. It was the
only contract event with no counterpart in the indexer: the ABI already carried the other reshaped events from
this cycle (`OrderFilled`/`PartialFill`/`EscrowReleased`/`EscrowRefunded` with their token arrays, `DeploymentAdded`,
`DestinationProtocolFeeUpdated`), so cancellation was the whole gap.

Added a `CANCELLED` order status, an `IOrderV3Cancellation` entity recording who cancelled and where, a handler,
and the datasource wiring. The `canceller` is worth storing separately from the order's `user`: the
destination-side cancel route is permissionless once the order has expired, so the two are not the same account
in general.

`recordOrderCancellation` only advances the status from `PLACED` — `updateOrderStatus` overwrites unconditionally,
and a cross-chain cancel is initiated on the destination chain while its refund lands on the source chain, so
without the guard a late-indexed cancellation would clobber a `REFUNDED` that had already been recorded. See
Decisions for why the guard sits in the service rather than in `updateOrderStatus`.

Files: `src/configs/abis/IntentGatewayV3.abi.json`, `src/configs/schema.graphql`,
`src/services/intentGatewayV3.service.ts`, `src/handlers/events/intentGatewayV3/orderCancelledV3.event.handler.ts`,
`src/mappings/mappingHandlers.ts`, `scripts/templates/evm-chain.yaml.hbs`.

## 2026-08-19 — Pool rates renormalize by the leg's own standard amount

`resolvePoolLeg` used to require `standardAmount === 10 ** inputDecimals` exactly, and `updateLiquidityPools` derived a chain sample as `medianPrice * scale`, which silently assumes that same one-unit probe. Both now work for whatever standard amount the phantom order carries: the rate is `medianPrice * scale * 10 ** inDecimals / standardAmount`, exact for any probe size, whole-token or not, and it collapses to the old expression when the probe is one unit. Verified against production: the four live one-unit rates (Base cNGN/USDC both directions, BSC cNGN/USDT both directions) reproduce byte-for-byte, and a 1000x probe with a 1000x quote yields the identical rate.

Motivation is quote precision on the pallet side, not the indexer's: a leg's output integer IS the published price, so one whole cNGN priced into 6-decimal USDC quotes ~715 base units and the price grid is 1/715 = 0.14% coarse. A curve of 1398 cNGN/USDC cannot publish finer than 1398.6014. At a 1000-token probe the same quote is 715,307 and the error drops to 0.00008%. The indexer had to stop hard-coding the one-unit assumption before that bump was possible.

The exact-value check was the only tripwire for a registry/pallet decimals disagreement, so it was replaced rather than deleted: the probe must now be plausible relative to one whole input token — at most `MAX_STANDARD_UNITS` (1e6) of them and at least `1/MAX_STANDARD_SUBDIVISIONS` (1/1e3) of one. Every realistic mismatch (6 vs 18, 6 vs 12, 8 vs 18) is a factor of 1e6 or more and still refuses attribution; the existing BSC test for an 18-decimal amount copied onto 6-decimal cNGN still passes, and a new test covers the same bug seen from the other side.

`ResolvedPoolLeg` gained `inDecimals` for the renormalization. The SDK's `PhantomSnapshotQuoter` needed no change — it already divides by `snapshot.standardAmount`.

The renormalization is exported as `poolRateFromQuote(medianPrice, resolved, standardAmount)` rather than living inline in `updateLiquidityPools`, which needs SubQuery store mocks to exercise. It is the one piece of arithmetic a wrong probe size would silently corrupt, so it is now a pure function pinned directly by tests: the four live mainnet rates at the deployed 1-unit probe, the identical rates at a 1000-unit probe, the extra digits the bump buys, and a non-whole 1.5-token probe. The 1-unit cases matter most — they are the currently deployed configuration, and at exactly one whole unit the two powers in the formula cancel, so those tests are the proof that raising the probe size elsewhere cannot move a published rate.

Files: `src/services/liquidityPool.service.ts`, `src/services/__tests__/liquidityPool.service.test.ts`.

## 2026-08-14 — Seed cumulative from daily rows; isolate seed failures (review fixes on #1085)

Review of the seeding change found the cumulative seed was computed on a basis the running code does not maintain: `updateCumulativeVolume` drops same-timestamp updates per record, so the chain-wide `IntentGatewayV3.FILLED` cumulative loses the second of any two same-block fills (even by different fillers) while per-filler cumulatives only collide within one filler. Seeding from summed filler cumulatives therefore assumed an equality the guard breaks going forward. The seed now derives the cumulative from the guard-free daily buckets it already aggregates, deleting the second table scan; paging reuses the existing `readAllPages` helper instead of a hand-rolled loop. The seed call is also wrapped in its own try/catch so a store error cannot swallow the fill's status, points, and user activity — on failure only the gateway volume update is skipped, leaving the marker uncreated so the next fill retries the seed and recovers the skipped fill from the filler daily rows. Added a test pinning the known same-block divergence between the FILLED cumulative and the daily/per-filler series.

Files: `src/services/volume.service.ts`, `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/volume.service.test.ts`.

## 2026-08-14 — Seed gateway volume from existing filler history (#1085)

On a resumed database the new `IntentGatewayV3.FILLED` series would have started from zero while the per-filler series already carried history. Added `VolumeService.seedAggregateVolume(aggregateBaseId, componentIdPrefix)`: a one-time, per-chain initialization that sums every existing `CumulativeVolumeUSD` and `DailyVolumeUSD` record matching the component prefix into the aggregate's cumulative record and per-day buckets. The aggregate's cumulative record doubles as the done-marker. Called from `updateOrderStatus` before the fill's own volume updates so the seed never counts the triggering fill, and the seeded record keeps the components' max `lastUpdatedAt` so the cumulative same-timestamp guard does not swallow that fill. Extended the `VolumeService` tests to cover seeding.

Files: `src/services/volume.service.ts`, `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/volume.service.test.ts`.

## 2026-08-13 — Gateway-level daily filled volume (#1085)

Daily filled volumes were only recorded per filler (`IntentGatewayV3.FILLER.<address>.<chain>.<date>`), so there was no single entry showing total volume filled through the intent gateway per chain per day. Added one `VolumeService.updateVolume` call with the filler-independent base ID `IntentGatewayV3.FILLED`, in the same FILLED branch of `updateOrderStatus` that records the per-filler volume. This produces `DailyVolumeUSD` records like `IntentGatewayV3.FILLED.EVM-8453.2026-07-30` and a `CumulativeVolumeUSD` record `IntentGatewayV3.FILLED.EVM-8453`, queryable from the existing `dailyVolumeUSDs` query. Also added the first unit tests for `VolumeService`.

Files: `src/services/intentGatewayV3.service.ts`, `src/services/__tests__/volume.service.test.ts` (new).
