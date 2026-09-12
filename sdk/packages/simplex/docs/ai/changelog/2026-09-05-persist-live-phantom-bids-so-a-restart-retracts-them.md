# 2026-09-05 — Persist live phantom bids so a restart retracts them

Each phantom interval's batch retracts the previous interval's bid on that chain, refunding its
0.01 BRIDGE deposit — but the previous commitment lived only in `IntentFiller`'s memory, so the
first batch after every restart carried no retraction and stranded a deposit (the account had
0.5 BRIDGE reserved from fifty of them). `RuntimeState` gains `phantomBids` (chain → commitment);
the filler takes a `StateStore` and persists the map whenever a phantom bid lands or is pooled
(`rememberPhantomBid`), boot seeds it with `restorePhantomBids(restoredState.phantomBids)` before
`start()`, and `livePhantomBids()` exposes it. `StateStore.set` replaces the whole record, so all
writers now go through `patchRuntimeState` (`src/data/state.ts`), including pause/resume in
`Simplex` and the CLI's `setPaused` — a pause no longer wipes the phantom bids. Added tests for
persistence, restore precedence and merge-safe pauses.
Files: `src/data/{state,types}.ts`, `src/core/{filler,boot}.ts`, `src/simplex.ts`,
`src/bin/simplex.ts`, `src/tests/phantom-bid-persistence.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
