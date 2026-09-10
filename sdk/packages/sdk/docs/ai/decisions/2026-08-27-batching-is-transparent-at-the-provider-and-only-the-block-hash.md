# 2026-08-27 — Batching is transparent at the provider, and only the block-hash half of the scan was made concurrent

Chosen: coalesce concurrent calls into a JSON-RPC batch inside the provider, and make the poll fetch a range's block hashes concurrently while leaving the events reads sequential.

Why not batch at the call sites: the callers that burst are spread out — the offchain fan-out in simplex's `handlePhantomOrders`, the scan, balance reads — and an explicit batch API would have to be threaded through each. The provider is where a burst is already visible as concurrency, and coalescing there needs no call site to know it is happening.

Why the events reads are still sequential, which is the non-obvious half. Making them concurrent looks like the bigger win — it would take a scan of n blocks to two requests instead of n+1 — but it silently undoes the `knownVersion` work. `api.at(hash, version)` resolves its registry through `_getBlockRegistryViaVersion`, which sets `lastBlockHash` on the shared registry; the subsequent `system.events` read goes through rpc-core's own registry swap, which calls `getBlockRegistry(hash)` *without* a version and finds it only by that `lastBlockHash`. Sequentially that always hits. Concurrently, n interleaved `at` calls each overwrite it, so all but one miss and fall through to `_getBlockRegistryViaHash` — the two RPCs per block that naming a version exists to avoid. Batching would hide that from the request counter while doubling the node's work, which is the opposite of the intent.

Alternatives considered for the events half:

- **Accept the registry re-resolution**, since the extra calls batch anyway. Rejected: it trades node-side work for a lower request count, and the limit exists to bound work.
- **`state_getStorage.raw(key, hash)` with one registry for the range.** `.raw` skips the swap (`isScale && blockHash && …` in rpc-core's `_createMethodSend`), so this really is two RPCs per block and one request for the range. Deferred, not rejected: it means decoding events by hand through `registry.createLookupType(meta.type.asPlain)` rather than `apiAt.query.system.events()`, and a wrong type there decodes to a shape the scan reads as "no orders" — a silent miss. Verifying it needs a running node, which the unit tests do not have. Worth doing behind the simnode test.

Why one request in flight at a time: overlapping flushes interleave against a `maxBatchSize` that a `-32010` refusal shrinks underneath them, so the same burst fragmented into a different number of requests run to run. Serialising also makes each request as full as it can be. The cost is that a submission can wait a round trip behind a scan flush, which at the paced rate is smaller than the wait the bucket already imposes.

Why a macrotask window rather than a microtask: `Promise.all` starts its calls in one synchronous run, but each then advances through several microtask turns before reaching the provider. A microtask flush fires between those turns and splits one burst across several requests.
