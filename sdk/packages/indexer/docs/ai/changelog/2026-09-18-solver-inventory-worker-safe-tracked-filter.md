# 2026-09-18 — Solver inventory: the tracked filter is a store read, not a cached set

`applyTokenTransfer` and `applyVaultShareTransfer` decided whether a `Transfer` involved a tracked
solver by consulting `trackedCache`, a module-level `Set` loaded once per process and mutated only by
`seedPendingSolvers`. That set is now gone: both handlers pass both sides of the transfer to the
per-solver store read they already performed, and let a missing row be the answer.

## Why the set could not be correct

SubQuery runs the mapping handlers in a worker thread per `--workers`, each thread a separate module
registry with its own copy of every module-level binding. Every `SUBQL_WORKERS` default in this
package is above one — 16 in `scripts/templates/partials/docker-command.hbs`, 6 in
`docker/docker-compose.local.yml` and `docker/docker-compose.nexus-ci.yml`, 2 in
`docker/docker-compose.solver-ci.yml` — and the block dispatcher rotates batches across all of them.

So the worker that ran the handler making a solver `TRACKED` was the only one whose set learned about
it. Every other worker held a set loaded before that solver existed and discarded its supported-token
`Transfer`s at the first filter, before any store read, RPC or log line.

The harm was one-directional and it over-reported. A solver's outgoing `Transfer`s are what a fill is
made of, so the dropped events were spends: the published `SolverInventory.balance` stayed at its
pre-spend value and the orderbook quoted depth that was no longer there. Nothing repaired it short of
the 24-hour `RECONCILE_INTERVAL_SECS` sweep or a process restart, and the solvers it hit hardest were
the freshly discovered ones, whose first fills are the ones that matter.

## Why the store read is a complete filter

`SolverInventory` and `SolverVaultShares` rows are written only by `applyReading`, which runs only
from `seedPendingSolvers` on the `PENDING` → `TRACKED` transition and from reconciliation. A row
therefore exists for exactly the solvers this node tracks, and `TrackedSolverStatus` has no third
state that could separate the two. Both handlers already read the row and already skipped on a miss,
so removing the set in front of them changes which events reach the read, not what the read decides.

The cost is up to two keyed store reads per supported-token `Transfer` where there were none for a
non-solver. That is the price of the correctness, and the reason the old comment claimed the filter
"must cost neither a store read nor an RPC". If it ever shows up in throughput, the replacement is a
cache versioned off `SolverInventoryHead` — not another set loaded once per process.

## Interface

`resetSolverInventoryCache` no longer exists. It was exported for tests and had no production caller.

`etag` and `lastSkipReport` in `solverWatchlist.service.ts` and `deployed` in `utils/multicall.ts` are
per-worker in the same way and are left alone: they cost a redundant fetch or a redundant `getCode`
probe per worker, and neither can drop an event.

Files: `src/services/solverInventory.service.ts`,
`src/services/__tests__/solverInventory.service.test.ts`
