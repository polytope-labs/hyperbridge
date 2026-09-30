# 2026-08-27 — Fix: the ranged block scan names the events key and its type outright

The `state_queryStorage` change in the entry below broke the phantom-filler E2E: zero bids on every chain, because the poll delivered no orders at all. Verified against a live simnode; the fix is verified there too.

The read asked for `api.query.system.events.key()`. polkadot-js builds a `StorageKey` from that argument and needs two things from it — key bytes, to match the change set the node returns, and `meta`, to decode that change set's value — and there are three accessors that each supply a different subset:

| argument | key bytes | metadata | result |
|---|---|---|---|
| `entry.key()` | correct | none | value typed `Raw`: undecoded bytes |
| `entry` (decorated) | **`[object Promise]`** | yes | matches no change set; value falls back to the entry's empty default |
| `entry.creator` | correct | yes | correct |

The first was the shipped bug: iterating `Raw` yields numbers, `phantomOrdersFrom` destructured `event` off one, and the tick threw on every tick forever. Nothing failed at the RPC layer, so it read as a poll that never found an order. The second was the first attempt at a fix, and is worse — it fails *silently*, returning an empty vector, because `StorageKey` derives key bytes by calling its argument and calling a decorated entry runs the query.

Rather than depend on picking correctly between them, the read now computes the key itself — `twox_128("System") ++ twox_128("Events")`, mirroring `system_events_storage_key` in `parachain/simtests` — calls `queryStorage.raw` so polkadot-js does no formatting at all, and decodes with `registry.createType("Vec<EventRecord>", value)`. A plain entry's key is a pure function of its pallet and item names, so there is nothing to look up and nothing to choose.

Two changes so this class of fault cannot be silent or fatal again. `phantomOrdersFrom` now rejects anything that is not a vector of event records instead of quietly finding nothing in it — a block reading as empty is the silent miss the block cursor exists to rule out. And `scanRangeAtOnce` treats that rejection like a refused method: the ranged path is abandoned for the life of the process, the error is reported once through `onError` so the degradation is visible, and the per-block path — which decodes through `api.at` rather than from a passed key — carries the poll from there. Losing the fast path costs requests; losing every bid is what it cost before.

The unit tests missed it because the harness handed back pre-decoded records whatever it was asked for. It now models what a node does: change sets are keyed by the real events key, and a request naming any other key gets no changes for it — silence, not an error. Nine tests fail if the key is wrong. `phantom-range.simnode.test.ts` covers the rest, against a real node, because the decisive behaviour is polkadot-js's handling of a real reply and no mock can stand in for it.

Files: `src/chains/intentsCoprocessor.ts`, `src/tests/pollPhantomOrders.test.ts`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `docs/ai/Flow.md`.
