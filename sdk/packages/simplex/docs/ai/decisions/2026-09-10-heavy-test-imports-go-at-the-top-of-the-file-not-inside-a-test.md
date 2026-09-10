# 2026-09-10 — Heavy test imports go at the top of the file, not inside a test

Decided: `src/tests/logger.test.ts` imports `FillerConfigService`, `ChainClientManager` and
`CacheService` statically. A dynamic `await import()` in the test body put the ~10s cost of loading
`@hyperbridge/sdk` inside that test's timeout; a static import puts it in vitest's collect phase,
which nothing times out. This is also what every other test file in the package does, so the
dynamic import was the anomaly rather than a pattern.

Rejected: raising the timeout past 20s. It moves the boundary instead of removing it. The cost is
machine-dependent and grows with the dependency graph, so the next slow week reopens the same bug.

Rejected: `vi.mock`-ing `@hyperbridge/sdk` so the real service classes load against a stub. It
would cut the file to about a second, but the mock has to name every sdk export the three services
touch, and it silently rots the moment one of them reaches for another. The failure would surface
as a confusing collect-time `X is not a function` in a test about logging. This test is worth
keeping honest — its whole claim is that the *real* services route through their filler's context.

Rejected: hoisting into `beforeAll` with a generous hook timeout. Same effect as the static import
but with a hook timeout still to tune, and it does not match the rest of the suite.

Rejected: deduping `@polkadot/util` to kill the "multiple versions" warning. The warning is real
but it is not this bug, and it is not a lockfile problem. `pnpm-lock.yaml` already pins simplex to
`@polkadot/util@14.0.3`; the 13.5.9 tree belongs to `packages/indexer` via `@subql/utils@2.x`, a
separate importer that pnpm is entitled to resolve separately. The duplicate only appears on a
machine whose `node_modules` predates simplex's `^13` -> `^14` bump — there, the on-disk symlink
still points at 13.5.9 and a plain `pnpm install` fixes it. Even resolved, it would not have moved
the timeout: `@polkadot` accounts for ~125ms of an 11.7s import.

The real lever on suite speed, if it is ever worth pulling, is that `@hyperbridge/sdk`'s barrel
entry point makes every importer resolve the entire graph — `FillerConfigService` pays it for two
symbols. That is an sdk export-map change with production consequences, not a test fix.
