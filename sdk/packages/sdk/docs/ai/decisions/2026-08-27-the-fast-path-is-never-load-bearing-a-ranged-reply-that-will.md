# 2026-08-27 — The fast path is never load-bearing: a ranged reply that will not decode falls back

Chosen: `phantomOrdersFrom` throws `EventDecodeError` on anything that is not a vector of event records, and `scanRangeAtOnce` responds by abandoning the ranged read permanently, reporting once, and letting the per-block path take over.

Why, concretely: the ranged read shipped asking for `system.events.key()` instead of the storage entry, so polkadot-js had no metadata to decode against and returned `Raw`. The poll found no orders in any block and no filler bid — for four minutes of E2E, and it would have been indefinite in production. The RPC never failed; only the decoding was wrong.

Alternatives considered:

- **Let the decode error propagate to `onError` and retry.** That is what happened, in effect, and it is a permanent outage: the next tick asks the same way and gets the same bytes. Retrying only helps a transient fault, and a type mismatch is not one.
- **Fall back silently.** Rejected: the fast path being off is worth knowing about, and this file's whole disposition is against silent degradation. Reported once, then quiet.
- **Validate the decoded value and skip just the bad block.** Rejected: it cannot distinguish "this block decoded to nothing" from "nothing decodes", and the cursor would advance past real orders either way.

The general shape worth keeping: the ranged read is an optimisation over a path that already worked, so every way it can fail should end at that path rather than at a stopped poll. The metadata-carrying key is pinned by a test, and the harness now decodes only when the key it was handed carries `meta` — reproducing the failure rather than papering over it.
