# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/indexer`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-09-01 — Declared V4 positions get their own table, re-read rather than carried forward (#1159)

Chosen: a `LiquidityProviderV4Position` row per (chain, tokenId), written from the bid declarations the
aggregation verified, and re-read on every refresh.

Alternative rejected — carry the last sweep's position VALUE forward on `PoolBidder`. No new table and no extra
reads, and wrong in the one case that matters: simplex funds fills out of these positions, so a V4-funded fill
drains the position inside the fill transaction while wallet and vault balances barely move. A carried value
would keep advertising precisely the inventory the fill just spent — the overstatement this whole issue exists
to remove.

Alternative rejected — add the tokenIds to `PoolBidder` or `LiquidityProviderBalanceV2`. Both are live tables, so
it is a migration; and neither has the right shape: a position backs a solver across every pool it bids in and
every token in its pool's pair, so it belongs to (provider, chain), not to a pool row.

Keyed by (chain, tokenId) rather than (provider, chain, tokenId): a position that changes hands then moves to its
new owner instead of being recorded under both, and the ownership re-check has one row to correct.

Reconciliation is per solver, not per chain: the solvers that bid have their rows replaced wholesale (a
declaration is per bid, and what this window's bid omits the weights already ignore), while a solver that did not
bid keeps what it last declared — silence is not a withdrawal. A row also goes when the re-read finds the
position burned or under another owner, which is the only signal available for a solver that has stopped bidding
entirely.

## 2026-09-01 — Escrow releases and vault events refresh by provider, and depths re-sum from the store (#1159)

Chosen: `refreshProviderLiquidity(chain, provider, tokens)` beside the pool-scoped entry point, for the events
that move a solver's inventory without naming a pool.

Alternative rejected — resolve those events to pools and reuse the pool-scoped path. An escrow release names the
order, so its pools are resolvable, but a vault event names only a token; and re-reading every bidder of every
pool the solver touches costs an RPC per bidder to learn what one solver's balance did.

That entry point forced one change to the shared core, worth knowing about: the (pool, chain) depths are now
re-summed from the STORED bidder rows after the writes, not from the rows the refresh happened to re-read.
Summing the re-read subset was correct only because the pool-scoped path re-reads every bidder; with one solver's
rows in hand it would have erased everyone else's contribution.

Chosen: the escrow release resolves its filler from the gateway's `_filled(commitment)` mapping with one
`eth_call` at the event's block. `_withdraw` writes the beneficiary in the same call that emits the event, so the
mapping is authoritative from that block onwards, and the source chain's node never has to wait for the
destination chain's node to have indexed the fill.

Chosen: the vault refresh hangs off the end of `YieldVaultService.recordLedger` rather than off the handlers. The
"is this one of our solvers" gate and the duplicate-log guard already live there, and both are exactly the gates
the refresh wants.

## 2026-09-01 — Per-event reads are pinned to the event's block (#1159)

Chosen: the balance and position reads a refresh performs are pinned to the block of the event that triggered
them, via the SDK's `blockTag` parameter, and the read memo is keyed by that block so several events in one block
share one set of reads.

Alternative rejected — read at the chain head, as the periodic sweep does. It is what the first cut did, and it
is not replayable: reindexing an old fill would stamp today's balance onto it. The pool guard (skip a pool whose
last snapshot postdates the event) hid that by making the refresh a no-op during a resync; pinning the reads
makes the guard a cost optimization rather than the only thing standing between a replay and wrong data.

Only the event's own chain is pinned. Block numbers are per chain and a refresh reaches across every chain the
pool is quoted on, so the others stay at the head — the correct reading available for them.

## 2026-09-01 — The balance series keeps its shape; #1159 §3 is left as a comment (#1159)

Not done, deliberately: the `trigger` enum, the nullable `transactionHash`, the
`{chain}-{token}-{solver}-{blockNumber}` id shape for event-triggered rows, and ordering "current liquidity" by
`snapshotTime` rather than `blockNumber`. All four change `LiquidityProviderBalanceV2`, which is live, and the
value is provenance metadata rather than correctness. Event-triggered rows therefore keep borrowing Hyperbridge's
head block, and the design is recorded as a comment on `recordProviderBalances` so a later migration has it to
hand. `LiquidityProviderV4Position` is additive — a new table, not a change to an existing one — which is why it
is in this change and §3 is not.

## 2026-09-01 — The fill refresh re-reads balances, publishes no provenance of its own, and skips replayed fills

Chosen: on `OrderFilled`, re-read every recorded bidder's balance for the pools the fill traded through and
rewrite the depths from them, leaving rates and every `lastUpdatedBlock`/`lastUpdatedAt` alone.

Alternative rejected — subtract the filled amount from the filler's row. No RPC, exact for the one solver that
filled, and wrong for everyone else: a fill is not the only thing that moves inventory between windows
(rebalances, other pools' fills, withdrawals), and the arithmetic would drift from the chain with nothing to
correct it until the next window. Re-reading measures the thing the depth is defined as.

Alternative rejected — refresh only the fill's own chain. Cheaper, and it is the only chain this fill changed.
But the pool's depth is a cross-chain sum, and the request is for the pool's liquidity, not one chain's slice;
the per-chain memo already collapses the extra reads, and the two guards below mean this only ever runs at the
tip.

Alternative rejected — stamp the refresh with the fill's block and timestamp. `lastUpdatedBlock` is a Hyperbridge
block number, and the staleness window (`MAX_SAMPLE_AGE_BLOCKS`) is measured in those; an EVM block number is
numerically unrelated and would make every row look either astronomically fresh or unusably stale. Writing only
`lastUpdatedAt` would leave the pair describing two different events. A dedicated `refreshedAt` field was the
honest version of this and was dropped deliberately: it buys provenance metadata at the price of a schema change
on live pool entities, and the depth being fresher than its timestamp claims is the safe direction to be wrong in.

Chosen: a pool whose `lastUpdatedAt` is newer than the fill is skipped without reading a balance. Balances are
read at the chain head — as the snapshot path also reads them — so refreshing against a fill the pool has already
been sampled after would replace fresher data with a partial view of it. It also makes a historical resync free:
every replayed fill is older than the pools' current samples, so no RPC is issued at all.

Chosen: a chain whose balances cannot be read in full is left exactly as indexed. A failed read and a zero
balance are indistinguishable downstream, so publishing the reads that succeeded would report the rest of that
chain's bidders as having withdrawn — worse than a stale number, because it looks like news.

Accepted blind spot, documented at `refreshPoolLiquidity`: the refresh reads wallet ERC-20 plus redeemable
ERC-4626 positions, but a snapshot's weight can also include Uniswap V4 positions the bid declared, and only a
bid names those. A V4-funded bidder therefore shrinks to its liquid inventory until the next window restores it.
Errs downward, which costs a quote rather than a failed fill. The fix, if V4-funded solvers become material, is to
split the position share out in the SDK aggregation and persist it on `PoolBidder` so it can be carried forward —
which is a schema change, hence not done pre-emptively.

Chosen: `PartialFill` refreshes on the same terms as `OrderFilled`, through the same service method. It spends
output-token inventory identically — the only difference is that it is emitted by the same-chain path
(`IntrinsicIntents`), where source and destination are one chain, so the pair resolves against a single registry.
Cross-chain fills are all-or-nothing (`ExtrinsicIntents` emits only `OrderFilled`), so between the two handlers
every fill that moves inventory now triggers a refresh. The per-block balance memo means a partial fill and the
full fill that follows it in the same block read balances once.

Chosen: a refresh extends `LiquidityProviderBalanceV2` too, stamping its rows with Hyperbridge's head block read
live from the configured node. The series is keyed by Hyperbridge block and a fill has none of its own, so
something has to supply one, and the head is the only number that keeps the series monotonic — which is the single
property consumers read it for ("greatest blockNumber is the current balance").

Alternative rejected — leave the series to snapshots. It is what the first cut did, and it leaves the pool rows
and the balance rows disagreeing between windows: the depth knows the inventory is spent while the newest balance
row still reports it, and those two are meant to be the same measurement.

Alternative rejected — overwrite the newest existing row in place. No RPC and no new key, but it rewrites history:
that row claims to be the balance at its Hyperbridge block, and after the overwrite it is not.

Alternative rejected — a fill-shaped key (`…-fill-{evmBlock}-…`). Append-only and honest about provenance, but it
puts two unrelated block sequences in one column, so the greatest-blockNumber rule stops meaning "latest".

The stamp is a borrowed clock, not a claim: the balance was read at the EVM chain's head, not reconstructed at
that Hyperbridge block, and the schema description now says so. Two readings landing on one key resolve to the
larger, the rule the sweep already follows — a refresh is always the V4-blind reading, so it must not replace a
complete one. The cost is that a second fill while Hyperbridge is still on the same block does not lower the row;
one block later it does.

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
## 2026-08-26 — Substrate node resilience is fixed in a forked node image, not in the indexer

Chosen: the substrate SubQuery node image comes from the `polytope-labs/subql` fork (`polytopelabs/subql-node-substrate`), where the websocket provider is wrapped so requests wait out a disconnect, response caching is disabled, reconnects are unbounded, and the http provider retries rate limits with a client-wide pause. The indexer package only changes the image reference.

Alternative rejected — make the indexer tolerate it: retry handler RPC reads, lower `--workers`, rely on `restart: unless-stopped`. The exits come from inside the node (block fetcher and dispatcher), which handler code cannot reach, and a restart is not a reconnect: it drops the unfinalized cache and, under `--multi-chain`, forces a rewind. The rate limit on the hosted http RPC is per client, so per-call retries in handlers cannot coordinate with the node's own fetch traffic; only the provider can pause all of them.

Alternative rejected — upstream the changes first and wait. Worth doing, but the deployment needs the behaviour now; the fork mirrors how `polytopelabs/subql-node-ethereum` is already produced from `polytope-labs/subql-ethereum`.

Accepted: this moves substrate from the deployed node 5.9.1 to 6.4.7 (node-core 19.x), matching `polytope-labs/subql`'s main after it was synced to upstream. The fork PR is built on that main, so it carries only these behavioural changes as a single commit, not the version history.

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
