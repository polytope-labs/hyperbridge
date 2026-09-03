# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/sdk`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-09-03 — The phantom-order lag limit is a constant, not an option

Chosen: `MAX_LAG_BLOCKS` (60) lives beside the poll and always applies.

Alternative rejected — a `maxLagBlocks` option, off by default. It was written that way first, to preserve the
cursor's existing property: it advances only past blocks whose events were really read, so an outage delays
orders instead of losing them, which is what made the cursor a fix for the dropped-subscription bug and what a
consumer reading the feed as history would want. But no such consumer exists, and the feed is not history: a
phantom order is a standing invitation to bid that expires with its window. Past that window there is no caller
for whom walking the backlog is right, so the option only offered every caller a way to get it wrong — and the
one that got it wrong on mainnet would have had to opt in to be fixed.

The property is bounded rather than abandoned: inside the limit an outage still delays orders instead of dropping
them, and the tests pin both halves.

Alternative rejected — expose the lag and let the caller reset the poll. It moves the same decision one layer out
while making every caller reimplement the jump, and the poll would still need the head it already reads.

Removed in the same breath: `lookbackBlocks`, which started a cold cursor some blocks behind the head so a
restarting process could still bid on a window already open. That is the same late bid the age gate downstream
now refuses — a process that has just come up is, by definition, near the end of any window it reaches back for —
so the option was buying exactly the behaviour this change exists to stop. No caller set it.

## 2026-09-01 — Verified V4 positions are reported out of the aggregation (#1159)

Chosen: `aggregatePhantomBids` returns the tokenIds it verified alongside the balances it swept, and
`readV4Position` (params object, with a `blockTag`) plus `positionAmountOfToken` are exported so a consumer can
re-value them later with the same reads and arithmetic a leg was weighted by.

Alternative rejected — let the consumer decode the bids itself. It is possible (the indexer stores every bid's
raw payload) and was tried: everything needed is already done here once — fetch the bids, verify each signature
and delegation, decode `paymasterAndData`, check ownership on-chain — so redoing it downstream duplicates the
security-relevant half of this module, and the two copies would drift.

Alternative rejected — have the consumer carry the last window's position VALUE forward instead of the tokenId.
It needs no new plumbing and is wrong in exactly the case that matters: simplex funds fills out of these
positions, so a fill drains the position inside the fill transaction while wallet and vault balances barely move,
and a carried value keeps advertising precisely the inventory the fill just spent.

Positions are reported after the ownership check, not as declared. A declaration is a pointer, not a claim, and
recording an unowned one downstream would hand the fill path a position to value that the solver cannot spend.

`solvers` is reported for the opposite reason: every other field is filtered by what the solver turned out to
hold or declare, so a verified bidder holding nothing anywhere is absent from all of them. A consumer
reconciling per-solver state ("this solver bid and declared nothing, so empty its row") cannot see it otherwise,
and that bidder is exactly the one whose inventory is all in positions it may have just stopped offering.

## 2026-09-01 — `blockTag` is a parameter of the balance read, and a per-chain map on the memo (#1159)

Chosen: `getTotalSolverBalance` takes a `blockTag` defaulting to `"latest"`, and `memoizedSolverBalance` takes a
`Record<chain, blockTag>`.

Alternative rejected — a single `blockTag` on the memo. A refresh reaches across every chain a pool is quoted on,
while the event that triggered it happened on one; block numbers are per chain, so one tag applied to all of them
would read some other chain at an arbitrary point in its history. The map pins the event's chain and leaves the
rest at the head, which is the only correct reading available.

Alternative rejected — leave every read at the head. Simpler, and it is what a periodic sweep wants, but a
per-event re-read at the head is not replayable: reindexing an old fill would stamp today's balance onto it.

## 2026-08-28 — The legacy `fillOrder` ABI is exported, not re-declared downstream

The indexer needs the v1 shape to decode bids with ethers. Two ways to give it one:

- **Re-declare it there.** No SDK change, but then two definitions of the same historical ABI have to stay
  identical forever, with nothing enforcing it. The bug being fixed is precisely a decoder that fell out of step
  with the shapes in the wild, so adding a second source of truth for those shapes is the wrong direction.
- **Export the existing constant.** One definition. Costs a public export of something that is otherwise an
  implementation detail — acceptable, since the reason it exists (deployments predating `validUntil`) is a fact
  about the network rather than about this package.

Exported from `fillOrderCodec` and re-exported on the `intents-helpers` sub-path, which is the entry point that
exists for tools that cannot load the full bundle. Pulling `fillOrderCodec` onto that sub-path adds no new
runtime dependency — it imports only viem, the gateway ABI, and types, all already reachable there.

`decodeFillOrder` is deliberately not exported alongside it: it is viem-based, and the callers that need this
constant are the ones that cannot run viem.

## 2026-08-27 — The bid expiry rides in `FillOptions`, not in the bid signature

Chosen: `validUntil` is a field on `FillOptions`, checked by `fillOrder` at execution.

The alternative was to put the expiry in the ERC-4337 signature blob and return it from
`SolverAccount.validateUserOp` as a `validUntil` validation range — the mechanism 4337 provides for exactly this.
That was built first and then abandoned, for reasons worth recording:

- **It needs a new signed field.** `userOpHash` does not cover `op.signature`, so an expiry carried there is
  rewritable by whoever replays the bid. Making it tamper-proof meant changing what the solver signs (an EIP-712
  `BidValidity` digest) and widening the selection signature 162 → 168 bytes.
- **That is a `SolverAccount` redeploy.** The account is reached by EIP-7702 delegation, so a new version means a new
  address and every solver re-delegating — and, since the old account keeps accepting the old format forever, it
  retires nothing already signed.
- **`FillOptions` needs none of that.** The options are part of `callData`, which `userOpHash` *does* cover. The
  expiry is authenticated for free, with no signature format change, no account redeploy, and no migration.
- **It covers more.** The signature-side check only bounds solver-selection bids. A check in `fillOrder` bounds every
  path into it.

The cost is that this fires at execution rather than validation: an expired bid is included, the nonce is consumed
and the account pays that op's gas, where a validation-time range would have had the bundler drop it for free. That
is a bounded, one-off cost per bid — and consuming the nonce permanently retires the bid, which the validation-time
version does not do. Fund loss, the thing that matters, is prevented either way.

Denominated in blocks rather than a timestamp so it reads against the same clock as `order.deadline` (`_blockNumber()`,
the L2 block number where those differ), and so the two cannot disagree about what "expired" means.

`0` means unbounded. That is the right default for a solver filling directly — it is only exposed to its own
staleness — and it keeps every existing caller working. The protection is opt-in by the party that needs it.

## 2026-08-27 — The implementation address identifies the FillOptions shape

Chosen: `getFillOptionsVersion` reads the ERC-1967 implementation slot and checks the address
against `LEGACY_FILL_OPTIONS_IMPLEMENTATIONS`, a set of implementations deployed before
`validUntil` existed. Anything else is v2.

EIP-1967 standardises three slots — implementation, admin, beacon — all holding addresses. There
is no version field in the spec to read, and OZ `Initializable`'s `uint64` only moves under
`reinitializer(N)`, which this contract does not use. The implementation address is the only value
the proxy actually updates on upgrade, so it is what identifies the deployed code.

The list is of **legacy** implementations, not current ones, so the default is v2. That direction
is the whole point: a newly shipped implementation needs no edit here, and once every deployment
is upgraded the set is vestigial and still correct.

A single entry covers every chain that runs it. The protocol contracts are CREATE2-deployed, so
`0x976B268b06f545c4A2BF44866Aa2465bd8B3C67d` is the pre-`validUntil` implementation on those
chains — confirmed with the maintainers rather than inferred, since the CREATE2 claim in the tree
is about the proxies and does not by itself say anything about implementations.

`CHAINS_WITHOUT_VALID_UNTIL` covers the rest. The testnets have not been redeployed and their
implementation addresses are not tracked here, so the address check alone would read them as
current and every fill would revert on a selector that does not exist. It is checked before the
slot read, both because the address is uninformative there and because it saves a round trip.
Delete a chain from that set as its gateway is redeployed; once it is empty the address check
covers everything on its own. Listing known-good implementations instead
would be the version constant this replaced wearing a different hat — a value someone must
remember to update on every upgrade, where forgetting breaks every fill on that chain.

Only v2 answers are cached, keyed by proxy address. A deployment can move from legacy to current
but never back, so a v2 result is true forever; caching a v1 result would pin the old encoding
across the very upgrade that changes it, since the proxy address does not move and nothing would
invalidate it. A still-legacy gateway therefore costs one storage read per fill, an upgraded one
costs none.

Also considered and dropped: scanning the implementation's runtime code for the v2 `fillOrder`
selector. It needs no address list and self-updates, and the selector does survive `via-ir` and
the optimizer — but it is a heuristic (a 4-byte sequence can appear in non-dispatcher data), and
both failure directions break every fill on the chain, since the two shapes cannot decode each
other. An address match is exact.

Earlier still, and rejected: a `fillOptionsVersion()` getter on the contract. A hand-maintained
integer is a second source of truth that answers what a deployment claims rather than what it can
decode.

## 2026-08-27 — The events key is computed, and the ranged reply is decoded explicitly

Chosen: `SYSTEM_EVENTS_KEY = twox_128("System") ++ twox_128("Events")`, `queryStorage.raw`, and `registry.createType("Vec<EventRecord>", value)`.

Why not ask polkadot-js for the key: the formatted call builds a `StorageKey` from its argument and takes both the key bytes and the decoding metadata from it, and the three obvious accessors each get a different subset right — `entry.key()` has the bytes and no metadata (values arrive as `Raw`), the decorated `entry` has metadata but yields `[object Promise]` as its bytes (matches nothing, value silently empty), and only `entry.creator` has both. All three type-check, none fails loudly, and two of them ship a poll that finds no orders. Shipping the first cost a red E2E; the second was caught only by running against a real node.

A plain entry's storage key is `twox_128(pallet) ++ twox_128(item)` and nothing else, which is why `parachain/simtests` computes it directly in `system_events_storage_key`. Computing it here removes the choice entirely, and `.raw` removes the formatting layer that made the choice matter — at the cost of naming the value type, which is stable and asserted against a live node.

Alternatives considered:

- **`entry.creator`.** Correct, and verified working. Rejected as the primary because it is one non-obvious accessor away from two that fail silently, and nothing in its shape says so — the next person to touch this line has the same three-way choice.
- **Keeping the formatted call and adding a test that the key matches.** That is what the unit test now does anyway, but it only constrains the call site; the decode still happens inside polkadot-js against metadata this code never sees.

## 2026-08-27 — The fast path is never load-bearing: a ranged reply that will not decode falls back

Chosen: `phantomOrdersFrom` throws `EventDecodeError` on anything that is not a vector of event records, and `scanRangeAtOnce` responds by abandoning the ranged read permanently, reporting once, and letting the per-block path take over.

Why, concretely: the ranged read shipped asking for `system.events.key()` instead of the storage entry, so polkadot-js had no metadata to decode against and returned `Raw`. The poll found no orders in any block and no filler bid — for four minutes of E2E, and it would have been indefinite in production. The RPC never failed; only the decoding was wrong.

Alternatives considered:

- **Let the decode error propagate to `onError` and retry.** That is what happened, in effect, and it is a permanent outage: the next tick asks the same way and gets the same bytes. Retrying only helps a transient fault, and a type mismatch is not one.
- **Fall back silently.** Rejected: the fast path being off is worth knowing about, and this file's whole disposition is against silent degradation. Reported once, then quiet.
- **Validate the decoded value and skip just the bad block.** Rejected: it cannot distinguish "this block decoded to nothing" from "nothing decodes", and the cursor would advance past real orders either way.

The general shape worth keeping: the ranged read is an optimisation over a path that already worked, so every way it can fail should end at that path rather than at a stopped poll. The metadata-carrying key is pinned by a test, and the harness now decodes only when the key it was handed carries `meta` — reproducing the failure rather than papering over it.

## 2026-08-27 — The scan reads a range with `state_queryStorage`, and declines it rather than risk a stale registry

Chosen: read every block's events in one `state_queryStorage(keys, from, to)` call, falling back to per-block reads under three conditions.

This supersedes the "deferred" bullet in the entry below, which contemplated `state_getStorage.raw` per block with a hand-rolled decode. `state_queryStorage` is better on both counts it was deferred for: it is one call for the range rather than one per block, and polkadot-js decodes a `Vec<StorageChangeSet>` reply itself — `_formatOutput` types each value from the storage key's own metadata — so there is no hand-rolled decoding to get wrong.

Alternatives considered:

- **`state_getStorage` per block, batched.** Rejected once the ranged call was available: same request count only if the batch holds the whole range, and it asks the node for n storage reads instead of one range walk.
- **Trusting the ranged reply unconditionally.** Rejected. rpc-core decodes it against the *default* registry, because `state_queryStorage` declares no `isHistoric` parameter and so gets no registry swap. That registry is fixed at connect and an HTTP api has no `subscribeRuntimeVersion` to refresh it, so after a runtime upgrade the decode is silently wrong in exactly the way a stale registry always is — events read as a shape the scan does not recognise, and the block passes as carrying nothing. `scanRangeAtOnce` therefore requires the tick's confirmed version to equal `api.runtimeVersion`, and hands off to the per-block path otherwise. The cost is that an upgrade costs a process restart to get the cheap path back; the direction of failure is right.
- **Treating a refusal as an error.** Rejected: `--rpc-methods=safe` makes the method permanently unavailable, not intermittently. It is detected once (`Method not found`, which is also how `check_if_safe` denies) and the poll switches paths for good without reporting anything.

What had to be reasoned about rather than assumed: `query_storage_unfiltered` in `sc-rpc` pushes a change set only when a key's value differs from the previous block in the range, and drops the set entirely when empty. So blocks are *missing* from the reply, not merely empty — on a quiet chain, consecutive blocks whose events are just the timestamp inherent's `ExtrinsicSuccess` encode identically and collapse to one entry. This is only safe because `phantom_order_commitment` derives the commitment from the block number: a block that registered orders cannot encode like any other block, so "absent" implies "no orders". A change to how commitments are built would break that, which is why it is written down here.

Because one call covers the range, the cursor advances the whole way or not at all — there is no partial progress to preserve, unlike the per-block path.

Why `maxBlocksPerPoll` went to 10 rather than up: the ranged read is one request but not free work. `sc-rpc` documents it as `O(|keys| * dist(from, to))` in time *and* memory, so a wide range is one request the node spends a long time on — and the same number still bounds the fallback, which is one request per block.

## 2026-08-27 — Batching is transparent at the provider, and only the block-hash half of the scan was made concurrent

Chosen: coalesce concurrent calls into a JSON-RPC batch inside the provider, and make the poll fetch a range's block hashes concurrently while leaving the events reads sequential.

Why not batch at the call sites: the callers that burst are spread out — the offchain fan-out in simplex's `handlePhantomOrders`, the scan, balance reads — and an explicit batch API would have to be threaded through each. The provider is where a burst is already visible as concurrency, and coalescing there needs no call site to know it is happening.

Why the events reads are still sequential, which is the non-obvious half. Making them concurrent looks like the bigger win — it would take a scan of n blocks to two requests instead of n+1 — but it silently undoes the `knownVersion` work. `api.at(hash, version)` resolves its registry through `_getBlockRegistryViaVersion`, which sets `lastBlockHash` on the shared registry; the subsequent `system.events` read goes through rpc-core's own registry swap, which calls `getBlockRegistry(hash)` *without* a version and finds it only by that `lastBlockHash`. Sequentially that always hits. Concurrently, n interleaved `at` calls each overwrite it, so all but one miss and fall through to `_getBlockRegistryViaHash` — the two RPCs per block that naming a version exists to avoid. Batching would hide that from the request counter while doubling the node's work, which is the opposite of the intent.

Alternatives considered for the events half:

- **Accept the registry re-resolution**, since the extra calls batch anyway. Rejected: it trades node-side work for a lower request count, and the limit exists to bound work.
- **`state_getStorage.raw(key, hash)` with one registry for the range.** `.raw` skips the swap (`isScale && blockHash && …` in rpc-core's `_createMethodSend`), so this really is two RPCs per block and one request for the range. Deferred, not rejected: it means decoding events by hand through `registry.createLookupType(meta.type.asPlain)` rather than `apiAt.query.system.events()`, and a wrong type there decodes to a shape the scan reads as "no orders" — a silent miss. Verifying it needs a running node, which the unit tests do not have. Worth doing behind the simnode test.

Why one request in flight at a time: overlapping flushes interleave against a `maxBatchSize` that a `-32010` refusal shrinks underneath them, so the same burst fragmented into a different number of requests run to run. Serialising also makes each request as full as it can be. The cost is that a submission can wait a round trip behind a scan flush, which at the paced rate is smaller than the wait the bucket already imposes.

Why a macrotask window rather than a microtask: `Promise.all` starts its calls in one synchronous run, but each then advances through several microtask turns before reaching the provider. A microtask flush fires between those turns and splits one burst across several requests.

## 2026-08-27 — Request pacing lives at the provider, keyed by endpoint

Chosen: an `HttpProvider` subclass whose `send` waits on a shared `TokenBucket`, with one bucket per endpoint origin in a module-level map.

Alternatives considered:

- **Pacing inside `pollPhantomOrders`** — a sleep between blocks, or a queue around the scan. Rejected: the poll is not the only caller. Balance polling, the offchain fan-out in simplex's `handlePhantomOrders`, and the HTTP submission fallback all hit the same endpoint, and the limit counts them together. Pacing the loudest caller leaves the sum unpaced, and it is the sum the node sees.
- **A bucket per `IntentsCoprocessor`.** Rejected: several fillers in one process each hold their own coprocessor, so N instances would each pace to the full budget and collectively exceed it by N. The limit is a property of the endpoint, so the bucket is too.
- **`p-queue` with `intervalCap`,** which the package already depends on. Rejected: its interval is a fixed window that refills all at once, so a burst arriving just after a boundary is passed straight through, which is the exact shape being defended against. A token bucket refills continuously.
- **Reacting to 429s only** (backoff, no pacing). Kept as well, but not instead: a 429 is already a request spent, and a limiter that is shedding load may be counting rejections too. Backoff recovers from a breach; the bucket is what stops causing them.

Two consequences worth knowing. The bucket makes a large scan a *queue*, so anything sharing the endpoint waits behind it — which is why `maxBlocksPerPoll` dropped to 20 in the same change; at 500 a catch-up would have queued ~1000 requests, over two minutes of them, ahead of a bid submission that is worth nothing after five blocks. And the bucket is FIFO on purpose: without it, a caller arriving on an idle bucket takes the token a queued one was waiting for, and a steady arrival stream starves the queue.

Why not simply raise the limit with the provider: the endpoint is derived from the websocket's, not configured (phantom orders live in that node's offchain storage), so operators do not necessarily control it. `HYPERBRIDGE_RPC_MAX_RPS` exists for those who do.

## 2026-08-27 — The block scan names a runtime version, and drops it the moment it might be wrong

Chosen: `getPhantomOrdersInBlock` takes an optional `knownVersion` for `api.at`; the poll establishes one per tick by reading `state_getRuntimeVersion` after the head read, and uses it only when it matches the previous reading.

Why it matters: with nothing to go on, `api.at(hash)` resolves a registry through `_getBlockRegistryViaHash` — `chain_getHeader(hash)` plus `state_getRuntimeVersion(parentHash)` — on every block, because its cheap paths are a registry pinned to that exact hash (only ever the previous block's) or one matching a version the caller names. That is two of the four RPCs a block cost, and they are pure overhead for a scan walking consecutive blocks under one runtime.

Alternatives considered:

- **Pass `api.runtimeVersion` unconditionally.** Rejected: an HTTP `ApiPromise` has no `subscribeRuntimeVersion`, so that field is frozen at connect. After an upgrade it names a version whose registry is still in `#registries`, so `_getBlockRegistryViaVersion` matches it and decodes new blocks against old metadata — and the failure is silent. The events come back in a shape the scan does not recognise, the block reads as carrying no phantom orders, and the cursor advances past it. That is precisely the silent miss the block cursor exists to rule out.
- **Read the version once and refresh on a slow timer** (every few minutes). Rejected for the same reason at a smaller scale: it buys a cheaper check by accepting a window in which orders are silently dropped. One read per tick costs ~0.07 req/s.
- **Skip `api.at` entirely** — `rpc.state.getStorage.raw(eventsKey, hash)` decoded against `api.registry`. That is two RPCs per block with no per-tick read at all, but it decodes against the connect-time registry with no way to notice an upgrade, and it hand-rolls event decoding. Worse on the axis that matters to be cheaper on the one that does not.

Why comparing two readings is sound: `specVersion` only increases, and the version is read *after* the head, so two equal readings mean no upgrade landed between them and therefore none in the range about to be scanned. A reading that differs means one did, and that tick falls back to per-block resolution, which is exact. The gap left is a backlog reaching back past an upgrade — recovering from an outage that long means those bid windows closed many upgrades ago.

The per-tick read is skipped when there is nothing to scan, so a quiet tick still costs exactly one request.

## 2026-08-25 — Intent quotes default to directional indexed rates without fallback

Chosen: `quoteIntent` defaults to an `indexed_rates` strategy that selects the depth-weighted aggregate `LiquidityPool.buyRate` for base-to-quote orders and `sellRate` for quote-to-base orders. Source and destination chains resolve the configured token deployments; raw amounts are calculated from the indexer's 18-decimal whole-token pool rate and both tokens' configured decimals. A missing directional rate is an error.

Alternatives considered:

- **Keep defaulting to the legacy directional Phantom snapshot.** Rejected: those snapshots resolve through a canonical Base market and do not use the pair-centric pool rate, so quotes can disagree with the indexer's current market.
- **Quote directly from one source/destination pair of `PoolChainLiquidity` rows.** Rejected: those rows are inputs to the indexer's pool price. `LiquidityPool.buyRate` and `sellRate` are the maintained depth-weighted merge of fresh chain samples and are the intended market-level quote.
- **Silently fall back to Phantom or Uniswap when a rate is absent.** Rejected: an order would be priced from a different market than the caller requested, hiding stale or incomplete indexer coverage and producing another unfillable quote.
- **Remove the old strategies immediately.** Rejected for compatibility: callers that explicitly select them can continue doing so, while all calls without a strategy use the corrected path.

The result uses a new `indexed_rates` discriminant and includes the selected rate side, value, timestamp, pair symbols, chains, and protocol fee. This makes the price used to construct the order inspectable without exposing indexer internals.

The public sell rate is the reciprocal of the indexer's base-per-quote direction. That reciprocal rounds up at 18 decimals: rounding down would let a quote-token-to-base-token order request slightly more base output than the indexed direction supports. Buy rates are already indexer-floored outputs and remain unchanged.

## 2026-08-27 — Every phantom quote is haircut; the pool tier drops to 10bps and wallet quotes pay 5bps

Chosen: `UNISWAP_QUOTE_HAIRCUT_BPS` becomes 10bps and a new `PHANTOM_QUOTE_HAIRCUT_BPS` of 5bps is charged to every bid that does **not** declare Uniswap V4 positions. A bid pays one haircut or the other, never both — `poolPriced` selects which function runs, and the structure from 2026-08-24 (keyed off the signed declaration, applied to the quote before the zero-check and the median) is unchanged.

Alternatives considered:

- **Stacking the 5bps on top of the pool haircut.** Rejected: the two are the same kind of adjustment — margin between what a solver names and what the protocol is willing to publish — and the pool tier is already the larger of the two. Compounding them would price a pool-funded bid at 15bps for reasons no single rationale explains, and would make the shipped number harder to reconcile against the fee tier it is meant to track.
- **Leaving wallet-funded quotes unhaircut.** Rejected: a wallet quote is still the most optimistic number a solver names at bid time, and the median it feeds is published to takers as a rate they can trade against. A small uniform margin is what keeps the published rate on the executable side of the bid.
- **Applying the base haircut after the median instead of per quote.** Rejected for the same reason as before: it is now a haircut every bid pays, so applying it to the aggregate would be arithmetically similar but would split one rule across two places in the code.

The 5bps also applies to a bid whose declaration is absent or unparseable, which is the correct reading — no declaration is no claim to be pricing off a pool.

## 2026-08-24 — The 30bps pool haircut keys off the declaration, and lands on price only

Chosen: in `aggregatePhantomBids`, haircut a bid's quoted leg amounts by 30bps when its paymasterAndData declaration names Uniswap V4 positions.

Alternatives considered:

- **Keying off the positions that pass the ownership check** (the `positions` array, not `declaration.uniswapV4Positions`). Rejected: those are filtered by owner and skipped entirely when the chain has no `positionManager`/`stateView` configured, so the same bid would be priced two different ways depending on indexer config. The declaration is what the solver signed, and it is what says "this quote came off a pool".
- **Haircutting the weight instead of the price.** Rejected: the weight is deliverable inventory, read on-chain, and it is also what the liquidity sweep reports — discounting it would make a provider's reported inventory and the depth attributed to it disagree, which the sweep exists to prevent. The fee is a cost of the trade, not a reduction in the size held.
- **Applying it after the median, to the leg's published price.** Rejected: the median is liquidity-weighted across bids, so haircutting the aggregate would also discount wallet-funded quotes that never pay a pool fee, and it would change which quote wins the median only by accident. The haircut belongs on the individual quote, before it competes.
- **Making the rate configurable per chain or pool.** Deferred: the positions these bids declare sit in 30bps-tier pools, and a knob invites the number to drift out of sync between the indexer and simplex. A single exported constant is easy to widen into a lookup if a different tier ever shows up.

Why 30bps at all: without it, a pool-priced bid reads richer than a wallet-funded one on a fee it has not yet paid, so it wins the median and the published price is one nobody can actually execute at.

A consequence worth knowing: a leg amount small enough that the haircut rounds it to zero now takes the "solver declined this leg" path. That is the correct reading — a quote that rounds away is not a price — and it only bites at dust amounts.

## 2026-08-21 — The coprocessor's HTTP provider runs with its response cache off

Chosen: `new HttpProvider(httpUrl, {}, 0)` — capacity 0 disables polkadot-js's per-provider LRU outright, so every `send` reaches the node.

Alternatives considered:

- A shorter TTL, or a poll interval longer than the TTL. The TTL slides — each hit refreshes it — so any poll faster than the TTL keeps a cached rejection alive indefinitely, and a slower poll would heal after one TTL only by coupling the cadence to a cache constant; the poll was made faster on purpose (#1138).
- A provider wrapper, or an upstream fix, that drops an entry when its promise rejects. Correct, but more code holding state this api has no use for, and an upstream fix still needs the workaround until it ships: polkadot-js master has the same `send` as 16.5.6.
- Keeping the cache and retrying inside the poll tick. A retry hits the same cache key and gets the same rejection; only a different request shape would get past it.

Why: the HTTP api exists for one-shot reads whose whole value is that nothing persists between calls (see the `http()` and `pollPhantomOrders` doc comments). A promise cache is exactly such persistence, and its only effect on this api was the failure mode: the poll reads each block once, so there are no repeat hits to serve, and `api.at(hash)` already reuses decoded registries at the api layer. Disabling it removes the hazard without adding a mechanism.

## 2026-08-18 — `SigningAccount` describes only what the SDK calls

(Amended the same day: `signRawHash` was removed in a follow-up commit; see the closing paragraph.)

Chosen: drop `signMessage` from `SigningAccount`, leaving `signRawHash` and `signTypedData`.

Alternative considered: leaving it in place, since removing a member from a published interface is a public API change.

Why: the interface exists so a caller can hand the SDK a signing backend, and every member it declares is a cost paid by every implementer. `signMessage` was never invoked — bid UserOperations are EIP-712-signed through `signTypedData` — so the cost bought nothing, and it forced downstream signer abstractions (notably `@hyperbridge/simplex`'s `Signer`) to carry a dead method into their own public surface. The narrowing is safe in the direction that matters: implementers with an extra method still satisfy the type, and only a caller of `solverSigner.signMessage` would break, of which there are none.

`signTypedData`'s `chainId` parameter went for a sharper reason than disuse: it is redundant with EIP-712 itself. The digest covers `domain.chainId`, `BidManager` built payloads that always set it, and the one downstream implementation that needed a chain id (MPCVault, for its request envelope) could read it from the payload — and defaulted the argument to `1` when absent, which is a bad failure mode for a signature.

`signRawHash` went the same day, in the follow-up that made `signAuthorization` and `signTransaction` required members of simplex's `Signer`: once those are guaranteed, nothing in either package computes an authorization digest for the signer to raw-sign, and keeping the member would have recreated the `signMessage` situation. `SigningAccount` is down to the one method this package invokes — `signTypedData`.
