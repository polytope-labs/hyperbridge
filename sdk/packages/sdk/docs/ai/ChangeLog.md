# ChangeLog

AI-maintained log of code changes in `sdk/packages/sdk`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/sdk/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-27 — A block scan is one `state_queryStorage` call, and the poll caps at 10 blocks

The previous entry left the scan at one request per block for events. `state_queryStorage(keys, from, to)` reads a key across a whole block range in one call, so the scan's request cost no longer depends on how many blocks it covers: a tick is now four requests flat — head, runtime version, the range's two bounding block hashes as one batched request, and the ranged read. `maxBlocksPerPoll` drops from 20 to 10 at the same time, because the cost moved rather than vanished (`sc-rpc` warns the method is `O(|keys| * dist(from, to))` in time and memory) and because it still bounds the per-block fallback.

`getPhantomOrdersInRange` is the new read; the event decoding it shares with `getPhantomOrdersAtHash` moved into a `phantomOrdersFrom` helper. polkadot-js formats a `Vec<StorageChangeSet>` reply into `[blockHash, valuesPerKey]` pairs with each value already typed from the storage key's metadata, so nothing is decoded by hand.

Two properties of the RPC drive the rest of the change. It answers with **diffs** — `query_storage_unfiltered` drops a block's change set when the value matches the previous block's — so a quiet chain's consecutive blocks come back as one entry. That is safe because a `PhantomOrderRegistered` commitment is derived from the block number, so a block that registered orders can never encode identically to another; absent provably means no orders. And it is **gated by `--rpc-methods`**, answered as `Method not found` when denied. The node here already runs unsafe RPC to serve `offchain_localStorageGet`, so it is normally available; a refusal switches the poll to the per-block path permanently and is handled rather than reported.

`scanRangeAtOnce` also declines the ranged read when the tick has no confirmed runtime version, or when that version is no longer the one the api's registry was built for — `state_queryStorage` declares no historic block hash, so rpc-core decodes its reply against the connect-time registry, which an upgrade leaves stale. After an upgrade the poll stays on the per-block path, which resolves the right registry, until the process restarts.

Files: `src/chains/intentsCoprocessor.ts`, `src/tests/pollPhantomOrders.test.ts`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `docs/ai/Flow.md`.

## 2026-08-27 — Concurrent Hyperbridge calls travel as one JSON-RPC batch request

Follow-up to the pacing change below. The endpoint's limit is counted in HTTP requests, and the reads here arrive as bursts of concurrent calls, so JSON-RPC 2.0 batching turns a burst into one request. Substrate supports it: `sc-rpc-server` sets `batch_request_config` on jsonrpsee, and `sc-cli` resolves `BatchRequestConfig::Unlimited` unless the operator passes `--rpc-disable-batch-requests` or `--rpc-max-batch-request-len`. polkadot-js does not — `HttpProvider.send` encodes exactly one call per POST and the package exports no batch API — so `http()` now builds a `BatchingHttpProvider` (`src/utils/batchingHttpProvider.ts`) instead of the `RateLimitedHttpProvider` it replaces.

Calls queued within one macrotask go out together, capped at 32 per request. The token bucket is charged once per HTTP request rather than once per call, which is the whole point. A lone call is sent as a plain request object, not a one-element array, so the common case is byte-for-byte what the base provider sent and needs no batch support at all. Flushes are serialised — one request in flight at a time — because overlapping flushes interleave against a `maxBatchSize` a refusal can shrink underneath them, which fragmented a burst into more requests than it needed. Both refusals are handled without losing a call: `-32005` disables batching for the provider's life and retries the calls singly, `-32010` halves the batch size and retries.

Batching only helps callers that have more than one call in flight, so the block scan now fetches a range's block hashes as one concurrent wave before reading events. `chain_getBlockHash` is the half of the pair that parallelises safely — it takes no historic block hash, so concurrent calls never trigger polkadot-js's per-hash registry resolution and cannot race each other's registry state. The events reads stay sequential for exactly that reason; see Decisions for why. `getPhantomOrdersAtHash` was split out of `getPhantomOrdersInBlock` to make that possible.

A tick now costs `3 + n` HTTP requests for n blocks rather than `2 + 2n`: a 20-block catch-up is 23 requests where the previous entry left it at 42 and it began at 81. A single-block tick is unchanged at 4 — the hashes there are one call, batched or not. The fan-out on a phantom order interval is where it bites hardest: up to 16 concurrent `offchain_localStorageGet` reads become one request.

Files: `src/chains/intentsCoprocessor.ts`, `src/utils/batchingHttpProvider.ts`, `src/tests/batchingHttpProvider.test.ts`, `src/tests/intentsCoprocessorRateLimit.test.ts`, `src/tests/pollPhantomOrders.test.ts`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `docs/ai/Flow.md`.

## 2026-08-27 — Hyperbridge HTTP reads are paced, and a block scan costs half the requests

Fillers were logging `[429]: Too Many Requests` from the Hyperbridge HTTP endpoint against a 10 req/s limit, on a poll whose interval is 15s. The interval was never the request rate. A tick costs one head read plus the cost of every block in its range, and `getPhantomOrdersInBlock` was costing four RPCs per block, not one: `api.at(hash)` with no version to go on resolves a registry by fetching `chain_getHeader(hash)` and then `state_getRuntimeVersion(parentHash)` — every block, because the `getUpgradeVersion` shortcut that would skip it only covers chains hardcoded in `@polkadot/types-known`. All of them go out back-to-back, so a tick averaging 0.4 req/s arrives as ~33 req/s. On a phantom order interval the fan-out in simplex's `handlePhantomOrders` adds one concurrent `offchain_localStorageGet` per configured chain (up to 16) on top.

Three changes:

- `http()` now builds a `RateLimitedHttpProvider`, an `HttpProvider` whose `send` waits on a `TokenBucket` (new, `src/utils/rateLimiter.ts`). Buckets are keyed by endpoint origin in a module-level map, so every coprocessor in a process pointed at the same node shares one budget. Default 8 req/s, `HYPERBRIDGE_RPC_MAX_RPS` to override.
- `getPhantomOrdersInBlock` takes an optional `knownVersion` and passes it to `api.at`, which drops both hidden reads. `pollPhantomOrders` establishes one per tick via `confirmedRuntimeVersion()` — read after the head, used only when it matches the previous reading, so a tick spanning a runtime upgrade falls back to exact per-block resolution. Steady state is 1 head + 1 version + 2 per block.
- `maxBlocksPerPoll` defaults to 20 rather than 500, and a 429 backs the poll off for a doubling number of ticks (capped at 8), reset by any tick that gets through. Ordinary failures still retry on the very next tick.

Files: `src/chains/intentsCoprocessor.ts`, `src/utils/rateLimiter.ts`, `src/tests/rateLimiter.test.ts`, `src/tests/intentsCoprocessorRateLimit.test.ts`, `src/tests/pollPhantomOrders.test.ts`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `docs/ai/Flow.md`.

## 2026-08-25 — Intent quotes use aggregate indexed pool rates by default

`IntentGateway.quoteIntent` now prices orders from the pair-centric indexer's depth-weighted aggregate `LiquidityPool.buyRate` and `sellRate`. Source and destination chains resolve the configured token deployments, while the quote converts the pool's whole-token rate into raw amounts with configured decimals, applies the source gateway protocol fee, and exposes the selected rate and timestamp in metadata. Reverse sell-rate reciprocals round up so quotes do not overpromise output. Phantom snapshot and Uniswap V4 pricing remain explicit compatibility strategies. Live sequential tests cover exact-input USDC to cNGN and exact-output cNGN to USDC across BSC and Base, including their different token decimal scales. The dead `binance.llamarpc.com` BSC default was replaced with `bsc-rpc.publicnode.com` after it blocked those live checks.

Files: `src/configs/chain.ts`, `src/protocols/intents/IntentGateway.ts`, `src/protocols/intents/LiquidityEngine.ts`, `src/protocols/intents/index.ts`, `src/protocols/intents/quote/index.ts`, `src/protocols/intents/quote/indexedRates.ts`, `src/protocols/intents/quote/types.ts`, `src/tests/sequential/intentGateway.test.ts`, `package.json`, `CHANGELOG.md`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `../../../docs/content/developers/sdk/api/intent-gateway.mdx`, `../../../docs/content/developers/evm/intent-gateway/placing-orders.mdx`.

## 2026-08-24 — Pool-priced phantom bids are haircut 30bps before aggregation

A phantom bid that declares Uniswap V4 positions is quoting off those pools, and a pool price is what a trade gets before the pool takes its fee — so the amount such a bid names is richer than what the solver clears once the sourcing swap goes through. `aggregatePhantomBids` now nets 30bps out of every leg amount on a bid whose declaration carries positions, before the quote reaches the zero-check, the weighted median, or the bidder rows. Non-declaring bids are untouched: wallet inventory has already paid its cost of goods.

The haircut runs off the declaration rather than the positions that survive the on-chain ownership check, so a quote is priced on the same basis the solver priced it on — including on a chain with no V4 contracts configured, where declared positions contribute no weight. A leg whose amount the haircut rounds to zero falls into the existing declined-leg path.

`UNISWAP_QUOTE_HAIRCUT_BPS` and `applyUniswapQuoteHaircut` are exported from both `@/protocols/intents` and the `intents-helpers` sub-path, so the indexer and simplex read the same number instead of restating it.

Files: `src/protocols/intents/phantom-aggregation.ts`, `src/protocols/intents/index.ts`, `src/intents-helpers.ts`, `src/tests/phantomAggregation.test.ts`.

## 2026-08-21 — HTTP provider no longer caches responses, so a failed phantom poll read is retried

`IntentsCoprocessor.http()` built its `HttpProvider` with polkadot-js defaults, which cache every request that names a block hash by storing the request promise itself — rejected promises included — under a 30s TTL refreshed on every hit. `pollPhantomOrders` retries the block it failed on with identical parameters each tick, so after one reset connection (`fetch failed` / `ECONNRESET`) every later tick got the same rejection back from the cache in a few milliseconds — the same `state_getRuntimeVersion(parentHash)` hash in every log line — and the node never saw another request. With the 15s poll interval inside the 30s TTL the entries never expired, so a filler stayed wedged until restart, placing no phantom bids. The provider is now constructed with cache capacity 0, which sends every request to the node. The cache bought nothing for this api: the poll reads each block once, and `api.at(hash)` reuses registries at the api layer regardless.

`intentsCoprocessorHttpCache.test.ts` pins the dependency's behaviour against a stub node that RSTs one request, and that the provider the coprocessor builds retries it.

Files: `src/chains/intentsCoprocessor.ts`, `src/tests/intentsCoprocessorHttpCache.test.ts`.

## 2026-08-19 — Fix the refund-POST gas pin #1144 left behind

#1144 split `CANCEL_MESSAGE_GAS = 800_000n` into `SOURCE_GET_RESPONSE_GAS` and `REFUND_POST_GAS`, both 1M, but left `orderCanceller.test.ts` asserting the POST at 800k — main's own CI has failed the concurrent-sdk step since it merged, and every PR cut from it inherited the red check. The pin now matches the shipped constant, with a comment naming the origin so the next reprice updates both.

Files: `src/tests/orderCanceller.test.ts`.

## 2026-08-18 — `SigningAccount` shrinks to `signTypedData` alone

`SigningAccount` is the contract a solver's signing backend satisfies to submit bids (`SubmitBidOptions.solverSigner`). It declared `signMessage(messageHash, chainId)`, which nothing in this package ever called: bids are signed as EIP-712 UserOperations in `BidManager.prepareSubmitBid` via `signTypedData`, and `GasEstimator`'s one `signMessage` call is viem's own method on a locally derived account, not this interface. The requirement propagated out to every implementer — including `@hyperbridge/simplex`, whose public `Signer` satisfied this type (it no longer extends it; simplex adapts with `sdkSigningAccount` at the two call sites) — so removing it here is what let that interface shrink to what a solver actually needs.

`signTypedData`'s second parameter went the same way. EIP-712 carries the chain id in `domain.chainId` — that is what the digest covers — and `BidManager` was passing it alongside a payload that already contained it, for the benefit of no implementation in this package.

`signRawHash` followed in the same day's follow-up: once simplex's `Signer` required `signAuthorization` and `signTransaction`, no caller in either package handed a raw digest to the interface, leaving `signTypedData` as the whole contract.

Type-only narrowing: it can break a caller of a removed member, and there are none inside the workspace.

Files: `src/types/index.ts`, `src/protocols/intents/BidManager.ts`.
