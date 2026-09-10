# 2026-08-20 — pairs.test.ts catches up with the probe and paymaster changes, and CI now runs it

`src/tests/pairs.test.ts` failed 12 of its 55 tests on main, unnoticed because no CI script ran
the file. Both failure groups were stale test expectations, not product regressions — each was
verified against the intent recorded in the 2026-08-19 entries before editing:

- Three phantom-probe tests still expected `quotePhantomFill` to cap its quote at the pair's
  `maxOrderSize`. The cap no longer rations probes (see "A phantom probe is no longer rationed by
  the pair's exposure cap", and the Decisions entry "The exposure cap governs fills, never
  probes") — that change updated `fx.one-sided-lp.test.ts` only and left this file behind. The
  assertions now expect the full unrationed quotes (e.g. 200,000 ZARP × 100 = 20,000,000 CNGN
  where the old cap produced 10,000,000), and the test names/comments no longer claim probes are
  capped.
- Nine profit-gates tests scored 0 where they expected a positive result (or the partial-fill
  flag). The leg loop now calls `paymasterReserveForToken` (see "Fill sizing reserves the
  paymaster's gas pull"), whose `hasPaymaster` check calls
  `configService.getCirclePaymasterAddress` / `getSimplexPaymasterAddress` — absent on the
  suite's `cfg` mock, so every evaluation threw `TypeError` into `calculateProfitability`'s catch
  and returned 0. Root cause pinned with a throwaway probe test against the exact mock shape. The
  mock now defines both getters as `() => undefined` (no paymaster configured → zero reserve),
  which restores the suite's exact spread/fee arithmetic.

`test:filler` now includes `src/tests/pairs.test.ts`, so the file runs in CI (the "Run simplex
test" step of `.github/workflows/test-sdk.yml`). It is pure-unit — no network, no env. Verified:
55/55 pass via `pnpm vitest run --maxConcurrency=1 src/tests/pairs.test.ts`, biome lint clean on
the file, and `vitest list` collects all four `test:filler` files after codegen.

Files: `src/tests/pairs.test.ts`, `package.json`, `docs/ai/{ChangeLog,Decisions}.md`.
