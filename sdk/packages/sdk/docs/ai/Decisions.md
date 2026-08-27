# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/sdk`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

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
