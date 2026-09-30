# 2026-08-27 — Hyperbridge HTTP reads are paced, and a block scan costs half the requests

Fillers were logging `[429]: Too Many Requests` from the Hyperbridge HTTP endpoint against a 10 req/s limit, on a poll whose interval is 15s. The interval was never the request rate. A tick costs one head read plus the cost of every block in its range, and `getPhantomOrdersInBlock` was costing four RPCs per block, not one: `api.at(hash)` with no version to go on resolves a registry by fetching `chain_getHeader(hash)` and then `state_getRuntimeVersion(parentHash)` — every block, because the `getUpgradeVersion` shortcut that would skip it only covers chains hardcoded in `@polkadot/types-known`. All of them go out back-to-back, so a tick averaging 0.4 req/s arrives as ~33 req/s. On a phantom order interval the fan-out in simplex's `handlePhantomOrders` adds one concurrent `offchain_localStorageGet` per configured chain (up to 16) on top.

Three changes:

- `http()` now builds a `RateLimitedHttpProvider`, an `HttpProvider` whose `send` waits on a `TokenBucket` (new, `src/utils/rateLimiter.ts`). Buckets are keyed by endpoint origin in a module-level map, so every coprocessor in a process pointed at the same node shares one budget. Default 8 req/s, `HYPERBRIDGE_RPC_MAX_RPS` to override.
- `getPhantomOrdersInBlock` takes an optional `knownVersion` and passes it to `api.at`, which drops both hidden reads. `pollPhantomOrders` establishes one per tick via `confirmedRuntimeVersion()` — read after the head, used only when it matches the previous reading, so a tick spanning a runtime upgrade falls back to exact per-block resolution. Steady state is 1 head + 1 version + 2 per block.
- `maxBlocksPerPoll` defaults to 20 rather than 500, and a 429 backs the poll off for a doubling number of ticks (capped at 8), reset by any tick that gets through. Ordinary failures still retry on the very next tick.

Files: `src/chains/intentsCoprocessor.ts`, `src/utils/rateLimiter.ts`, `src/tests/rateLimiter.test.ts`, `src/tests/intentsCoprocessorRateLimit.test.ts`, `src/tests/pollPhantomOrders.test.ts`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `docs/ai/Flow.md`.
