# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/sdk`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

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
