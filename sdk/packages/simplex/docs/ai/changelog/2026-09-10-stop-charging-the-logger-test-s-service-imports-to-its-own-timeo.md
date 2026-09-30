# 2026-09-10 — Stop charging the logger test's service imports to its own timeout

`service wiring > routes services through their filler's context, not the process one` timed out
intermittently at its 20s override — measured at 14.5s, 20.5s (fail) and 16.7s across three
isolated runs, so roughly one run in three under load. The three services it needs were pulled in
with `await import()` inside the test body, which charges the import to that test's timeout budget.

The file's comment blamed `@polkadot` WASM crypto init. It is not that. A CPU profile of the same
import outside vitest puts 9.5s of an 11.7s import in Node's ESM loader — `package_json_reader`
resolving `exports` maps across the pnpm tree — against ~125ms of actual `@polkadot` execution.
Any one of the three services costs this, because each reaches `@hyperbridge/sdk`, whose barrel
drags in the whole dependency graph. Measured separately: `@hyperbridge/sdk` 11.4s, then
`FillerConfigService` 118ms, `ChainClientManager` 850ms, `CacheService` 33ms on top.

The imports are now static, which is what every other test file in the package already does. The
cost lands in vitest's collect phase, which no timeout bounds, so both `20_000` overrides are gone
(the second, on `LoggerContext isolation > does not write to the process-wide context`, came from
the same commit and never had an import to pay for — that test is synchronous and touches nothing
heavy). Test bodies now run in 113-352ms.

Verified with 10 consecutive runs of `vitest run --maxConcurrency=1 src/tests/logger.test.ts`: 10
passes, no timeouts. Collect ranged 5.5s-20.6s across those runs — one run spent 20.63s there,
which would have failed under the old structure and is now harmless. Three further runs alongside
five other sdk-importing suites at default concurrency: logger.test.ts passed in 107-258ms each
time.

Not bumping the package version: the change touches a test file only, and `files` publishes `dist`,
so a bump would ship an identical tarball. Note also that CI's simplex step runs `test:filler`,
which does not include this file — the flake only ever bit local full-suite runs.

The absolute numbers here swing wildly with machine load. On the same tree, with no change to
the code or the installed dependencies, the import measured 2.3s in collect on an idle box and
52.9s at a load average of 99. The test body still finished in 229ms in that 52.9s run. No fixed
timeout survives a 20x spread — which is the whole argument for moving the cost out of the timed
region rather than raising the limit.

Files: src/tests/logger.test.ts, docs/ai/changelog/ and docs/ai/decisions/ entries for this change.
