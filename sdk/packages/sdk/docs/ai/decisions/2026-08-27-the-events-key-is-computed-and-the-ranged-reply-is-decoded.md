# 2026-08-27 — The events key is computed, and the ranged reply is decoded explicitly

Chosen: `SYSTEM_EVENTS_KEY = twox_128("System") ++ twox_128("Events")`, `queryStorage.raw`, and `registry.createType("Vec<EventRecord>", value)`.

Why not ask polkadot-js for the key: the formatted call builds a `StorageKey` from its argument and takes both the key bytes and the decoding metadata from it, and the three obvious accessors each get a different subset right — `entry.key()` has the bytes and no metadata (values arrive as `Raw`), the decorated `entry` has metadata but yields `[object Promise]` as its bytes (matches nothing, value silently empty), and only `entry.creator` has both. All three type-check, none fails loudly, and two of them ship a poll that finds no orders. Shipping the first cost a red E2E; the second was caught only by running against a real node.

A plain entry's storage key is `twox_128(pallet) ++ twox_128(item)` and nothing else, which is why `parachain/simtests` computes it directly in `system_events_storage_key`. Computing it here removes the choice entirely, and `.raw` removes the formatting layer that made the choice matter — at the cost of naming the value type, which is stable and asserted against a live node.

Alternatives considered:

- **`entry.creator`.** Correct, and verified working. Rejected as the primary because it is one non-obvious accessor away from two that fail silently, and nothing in its shape says so — the next person to touch this line has the same three-way choice.
- **Keeping the formatted call and adding a test that the key matches.** That is what the unit test now does anyway, but it only constrains the call site; the decode still happens inside polkadot-js against metadata this code never sees.
