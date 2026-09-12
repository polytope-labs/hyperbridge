# 2026-08-27 — The scan reads a range with `state_queryStorage`, and declines it rather than risk a stale registry

Chosen: read every block's events in one `state_queryStorage(keys, from, to)` call, falling back to per-block reads under three conditions.

This supersedes the "deferred" bullet in the entry below, which contemplated `state_getStorage.raw` per block with a hand-rolled decode. `state_queryStorage` is better on both counts it was deferred for: it is one call for the range rather than one per block, and polkadot-js decodes a `Vec<StorageChangeSet>` reply itself — `_formatOutput` types each value from the storage key's own metadata — so there is no hand-rolled decoding to get wrong.

Alternatives considered:

- **`state_getStorage` per block, batched.** Rejected once the ranged call was available: same request count only if the batch holds the whole range, and it asks the node for n storage reads instead of one range walk.
- **Trusting the ranged reply unconditionally.** Rejected. rpc-core decodes it against the *default* registry, because `state_queryStorage` declares no `isHistoric` parameter and so gets no registry swap. That registry is fixed at connect and an HTTP api has no `subscribeRuntimeVersion` to refresh it, so after a runtime upgrade the decode is silently wrong in exactly the way a stale registry always is — events read as a shape the scan does not recognise, and the block passes as carrying nothing. `scanRangeAtOnce` therefore requires the tick's confirmed version to equal `api.runtimeVersion`, and hands off to the per-block path otherwise. The cost is that an upgrade costs a process restart to get the cheap path back; the direction of failure is right.
- **Treating a refusal as an error.** Rejected: `--rpc-methods=safe` makes the method permanently unavailable, not intermittently. It is detected once (`Method not found`, which is also how `check_if_safe` denies) and the poll switches paths for good without reporting anything.

What had to be reasoned about rather than assumed: `query_storage_unfiltered` in `sc-rpc` pushes a change set only when a key's value differs from the previous block in the range, and drops the set entirely when empty. So blocks are *missing* from the reply, not merely empty — on a quiet chain, consecutive blocks whose events are just the timestamp inherent's `ExtrinsicSuccess` encode identically and collapse to one entry. This is only safe because `phantom_order_commitment` derives the commitment from the block number: a block that registered orders cannot encode like any other block, so "absent" implies "no orders". A change to how commitments are built would break that, which is why it is written down here.

Because one call covers the range, the cursor advances the whole way or not at all — there is no partial progress to preserve, unlike the per-block path.

Why `maxBlocksPerPoll` went to 10 rather than up: the ranged read is one request but not free work. `sc-rpc` documents it as `O(|keys| * dist(from, to))` in time *and* memory, so a wide range is one request the node spends a long time on — and the same number still bounds the fallback, which is one request per block.
