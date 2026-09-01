# ChangeLog

AI-maintained log of code changes in `sdk/packages/indexer`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog (there is none for the indexer; the published packages use changesets `CHANGELOG.md` files).

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-09-01 — Refresh a pool's LP balances on every order fill

A pool's published depth is the sum of its bidders' output-token inventory, measured when the last phantom bid
window closed. A fill spends some of that inventory, so between windows the depth advertises capacity the
solvers no longer hold — and the depth is what a taker sizes an order against.

`OrderFilled` now re-reads the balances of every LP recorded as backing the pools the fill traded through and
republishes the depths from them: `PoolBidder.liquidity`, `PoolChainLiquidity.depth`/`bidCount`/unrestricted
slice, `PoolRoute.depth`/`bidCount`, and the pool's own `sellDepth`/`buyDepth`/bid counts. A bidder left holding
nothing loses its row, so the "every row is a bidder with capacity" invariant survives, and the routes it alone
declared go with it. Rates are untouched — nothing here observes a quote — though a pool's merged rate can still
move, because the chains are weighted by the depths that just changed.

`PartialFill` refreshes on the same terms, through the same service method: it spends inventory identically, and
being the same-chain path (`IntrinsicIntents`) its source and destination are one chain. Cross-chain fills are
all-or-nothing, so between the two handlers every fill that moves inventory now triggers a refresh.

The pools are resolved from the order's own two sides: the input symbols on its source chain paired with the
output symbols on the destination chain, via the same token registry the snapshot path uses. An order in
untracked assets resolves to no pool and costs nothing, which is most of them.

Two guards keep the cost bounded and the data honest. A pool whose last snapshot postdates the fill is skipped
entirely — that snapshot already read balances this fill had moved, and during a resync it is every pool, so
replaying history triggers no RPC at all. And a chain whose balances cannot be read in full is left exactly as
indexed rather than partially republished: a failed read is indistinguishable from a zero balance, so half a
chain would report the unread bidders as departed.

`lastUpdatedBlock`/`lastUpdatedAt` are deliberately not moved anywhere. They date the snapshot that priced the
pool, in Hyperbridge blocks; a fill carries an EVM block number of another chain, which is not comparable with
them and would wreck the `MAX_SAMPLE_AGE_BLOCKS` staleness filter.

The balance-reader wiring (`setAggregationFetch(safeFetch)`, the per-chain RPC map, the per-block memo) moved out
of `handlePhantomOrderPrices.handler.ts` into `src/utils/solverBalance.ts` so both paths read balances the same
way; the memo key is now a string so it can identify a block of either chain. The pool-level merge and the
unrestricted-slice split were extracted into `mergeChainRowsIntoPool` and `bidderDepths`, shared by the snapshot
writer and the refresh — the two must publish the same numbers from the same bidders.

Unrelated but in the way: `phantomOrder.handlers.test.ts` was red before this change — all 14 of its
`handlePhantomOrderPrices` cases threw `sortPoolSymbols is not a function`. Its `@hyperbridge/sdk/intents-helpers`
mock lists only the three entry points the handlers call, and the pool-token registry re-exports its symbol
ordering from that same module, so mocking it dropped `poolSlug`/`sortPoolSymbols` and every pool attribution
threw. The mock now spreads `jest.requireActual` and overrides only those three; importing the SDK under jest has
worked since `transformIgnorePatterns` was added. That suite covers the handler wiring this change moved, so it
had to run to verify the move — all 16 cases pass.

Files: `src/services/liquidityPool.service.ts`, `src/services/intentGatewayV3.service.ts`,
`src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`,
`src/handlers/events/intentGatewayV3/partialFilledV3.event.handler.ts`,
`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`, `src/utils/solverBalance.ts` (new),
`src/services/__tests__/liquidityPoolRefresh.service.test.ts` (new),
`src/handlers/events/substrateChains/__tests__/phantomOrder.handlers.test.ts`.

## 2026-08-28 — Decode phantom bids of either `fillOrder` shape

`extractFillDataVm2` built one ethers `Interface` from `FILL_ORDER_ABI`. When `FillOptions` gained `validUntil`
that ABI moved to the v2 shape, and since ethers validates the selector before decoding, every v1-shaped bid
started throwing. The throw was swallowed by a `continue`, the function returned null, and `aggregatePhantomBids`
drops a null bid with no log line — so the loss was silent.

That is not a future risk: `@hyperbridge/sdk` is a `workspace:*` dependency, so the indexer picked up the v2 ABI
immediately, while every mainnet gateway still runs the pre-`validUntil` implementation. `getFillOptionsVersion`
therefore returns 1 and simplex encodes v1 bids, all of which were being discarded — no bidder rows, no median,
no pool-rate snapshot from any of them.

Now tries both shapes, mirroring the SDK's `decodeFillOrder`. The selectors differ (`0x5cfb1ea5` vs
`0xa5470064`), so neither can mis-decode the other's payload. The v1 ABI is imported from the SDK rather than
re-declared here, so the two cannot drift apart again.

Verified by running the real function against both shapes: before the change v1 returned null and v2 decoded;
after, both decode, and a wrong target or non-`fillOrder` calldata still returns null.

Also adds `transformIgnorePatterns` to the jest config. The SDK's CJS bundles `require()` ESM-only packages
(`p-queue` -> `eventemitter3`/`p-timeout`, `lodash-es`), which jest cannot parse untransformed, so *any* test
importing `@hyperbridge/sdk` or its `intents-helpers` sub-path failed to load at all against a freshly built SDK.
That was blocking the new test, and it was also silently keeping the existing `phantom-decode.test.ts` from
running — that file passes again now.

Files: `src/utils/phantom-decode.ts`, `src/utils/__tests__/phantom-decode.fill.test.ts`, `jest.config.ts`.

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
