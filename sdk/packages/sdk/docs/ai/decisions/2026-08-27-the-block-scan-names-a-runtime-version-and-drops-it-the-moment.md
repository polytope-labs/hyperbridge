# 2026-08-27 — The block scan names a runtime version, and drops it the moment it might be wrong

Chosen: `getPhantomOrdersInBlock` takes an optional `knownVersion` for `api.at`; the poll establishes one per tick by reading `state_getRuntimeVersion` after the head read, and uses it only when it matches the previous reading.

Why it matters: with nothing to go on, `api.at(hash)` resolves a registry through `_getBlockRegistryViaHash` — `chain_getHeader(hash)` plus `state_getRuntimeVersion(parentHash)` — on every block, because its cheap paths are a registry pinned to that exact hash (only ever the previous block's) or one matching a version the caller names. That is two of the four RPCs a block cost, and they are pure overhead for a scan walking consecutive blocks under one runtime.

Alternatives considered:

- **Pass `api.runtimeVersion` unconditionally.** Rejected: an HTTP `ApiPromise` has no `subscribeRuntimeVersion`, so that field is frozen at connect. After an upgrade it names a version whose registry is still in `#registries`, so `_getBlockRegistryViaVersion` matches it and decodes new blocks against old metadata — and the failure is silent. The events come back in a shape the scan does not recognise, the block reads as carrying no phantom orders, and the cursor advances past it. That is precisely the silent miss the block cursor exists to rule out.
- **Read the version once and refresh on a slow timer** (every few minutes). Rejected for the same reason at a smaller scale: it buys a cheaper check by accepting a window in which orders are silently dropped. One read per tick costs ~0.07 req/s.
- **Skip `api.at` entirely** — `rpc.state.getStorage.raw(eventsKey, hash)` decoded against `api.registry`. That is two RPCs per block with no per-tick read at all, but it decodes against the connect-time registry with no way to notice an upgrade, and it hand-rolls event decoding. Worse on the axis that matters to be cheaper on the one that does not.

Why comparing two readings is sound: `specVersion` only increases, and the version is read *after* the head, so two equal readings mean no upgrade landed between them and therefore none in the range about to be scanned. A reading that differs means one did, and that tick falls back to per-block resolution, which is exact. The gap left is a backlog reaching back past an upgrade — recovering from an outage that long means those bid windows closed many upgrades ago.

The per-tick read is skipped when there is nothing to scan, so a quiet tick still costs exactly one request.
