# 2026-08-21 — HTTP provider no longer caches responses, so a failed phantom poll read is retried

`IntentsCoprocessor.http()` built its `HttpProvider` with polkadot-js defaults, which cache every request that names a block hash by storing the request promise itself — rejected promises included — under a 30s TTL refreshed on every hit. `pollPhantomOrders` retries the block it failed on with identical parameters each tick, so after one reset connection (`fetch failed` / `ECONNRESET`) every later tick got the same rejection back from the cache in a few milliseconds — the same `state_getRuntimeVersion(parentHash)` hash in every log line — and the node never saw another request. With the 15s poll interval inside the 30s TTL the entries never expired, so a filler stayed wedged until restart, placing no phantom bids. The provider is now constructed with cache capacity 0, which sends every request to the node. The cache bought nothing for this api: the poll reads each block once, and `api.at(hash)` reuses registries at the api layer regardless.

`intentsCoprocessorHttpCache.test.ts` pins the dependency's behaviour against a stub node that RSTs one request, and that the provider the coprocessor builds retries it.

Files: `src/chains/intentsCoprocessor.ts`, `src/tests/intentsCoprocessorHttpCache.test.ts`.
