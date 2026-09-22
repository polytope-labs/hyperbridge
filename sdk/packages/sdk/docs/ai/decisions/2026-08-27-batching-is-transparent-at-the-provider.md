# 2026-08-27 — Batching is transparent at the provider

Chosen: coalesce concurrent calls into a JSON-RPC batch inside the provider.

Why not batch at the call sites: the callers that burst are spread out, and an explicit batch API would have to be threaded through each. The provider is where a burst is already visible as concurrency, and coalescing there needs no call site to know it is happening.

Why one request in flight at a time: overlapping flushes interleave against a `maxBatchSize` that a `-32010` refusal shrinks underneath them, so the same burst fragmented into a different number of requests run to run. Serialising also makes each request as full as it can be. The cost is that a call can wait a round trip behind another flush, which at the paced rate is smaller than the wait the bucket already imposes.

Why a macrotask window rather than a microtask: `Promise.all` starts its calls in one synchronous run, but each then advances through several microtask turns before reaching the provider. A microtask flush fires between those turns and splits one burst across several requests.
