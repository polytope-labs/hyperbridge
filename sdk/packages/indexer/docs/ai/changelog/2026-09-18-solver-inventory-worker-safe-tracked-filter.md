# 2026-09-18 — Solver inventory: the tracked filter moves to SubQuery's cross-worker cache

`applyTokenTransfer` and `applyVaultShareTransfer` decided whether a `Transfer` involved a tracked
solver by consulting `trackedCache`, a module-level `Set` loaded once per process and mutated only by
`seedPendingSolvers`. That set is now gone. The filter is the same shape — one set of tracked solvers
per chain — but it lives in `cache`, and the per-solver store read stays behind it as the authority.

## Why a module-level set could not be correct

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

## Why the filter is `cache` and not a store read

`cache` is SubQuery's cross-worker cache. The object itself lives on the main thread
(`InMemoryCacheService`); each worker's `WorkerInMemoryCacheService` proxies `get`/`set` to it over
the same host channel `store` already uses, and `WorkerCoreModule` installs that override, so it is
live under `@subql/node-ethereum`'s worker module. A `cache.set` from the worker that seeds a solver
is therefore visible to all of them — the property module state lacked. It is declared for this
package in `src/types/global.d.ts`, alongside `store` and `logger`.

A keyed store read cannot take the filter's place, even though `SolverInventory` and
`SolverVaultShares` rows exist for exactly the solvers this node tracks. `cacheModel.get` populates
its LFU only `if (record)`, so a _miss_ is never cached — and a miss is what every non-solver address
is. Filtering on the store would mean a fresh Postgres `findOne` per address per `Transfer`, on
tokens whose transfers are overwhelmingly not solvers'. From a worker both are the same single host
round-trip; the difference is entirely what the main thread does with it.

## Staleness, and what bounds it

Two mechanisms, because one is not quite enough:

- **`seedPendingSolvers` publishes each newly seeded solver to the cache**, right after the
  `TrackedSolver` row is saved and before any worker can see that solver's next `Transfer`. This is
  the path that covers ordinary discovery.
- **`advanceHead` rebuilds the set from the store**, past the `HEAD_INTERVAL_SECS` throttle it
  already applies. This exists for one race: two workers seeding at once, where the loser's store
  read predates the winner's write. The rebuild unions the store's rows with whatever the cache
  already held, so a lost update repairs itself rather than persisting.

The rebuild rides the head throttle rather than carrying a bound of its own, and that choice is the
point. The throttle is in seconds of block time, so the staleness bound is the same on every chain —
a block count would have meant roughly two minutes on Arbitrum and a hundred on Ethereum from one
constant. It is also gated on the shared `SolverInventoryHead` row, so the rebuild costs one
`getByFields` per chain per interval, not one per worker. The transfer handlers cannot carry a
seconds-based clock themselves: their `timestamp` is deliberately a lazy RPC, resolved only once a
tracked solver is involved, so reading it per `Transfer` would cost more than the filter saves.

`InMemoryCacheService` has no rollback hook, unlike the store cache, so a reorg that un-discovers a
solver leaves a stale member. That direction is harmless: the store read behind the hit finds no row.
The dangerous direction — a member missing — is what the publish and the rebuild cover. The cache is
also empty after a restart, which is correct rather than stale: the first use per chain rebuilds it
from the store.

## Interface

`resetSolverInventoryCache` no longer exists. It was exported for tests and had no production caller.

`src/types/global.d.ts` gains `cache`, which this package declared its own globals without.

`etag` and `lastSkipReport` in `solverWatchlist.service.ts` and `deployed` in `utils/multicall.ts` are
per-worker in the same way and are left alone: they cost a redundant fetch or a redundant `getCode`
probe per worker, and neither can drop an event.

Files: `src/services/solverInventory.service.ts`, `src/types/global.d.ts`,
`src/services/__tests__/solverInventory.service.test.ts`
