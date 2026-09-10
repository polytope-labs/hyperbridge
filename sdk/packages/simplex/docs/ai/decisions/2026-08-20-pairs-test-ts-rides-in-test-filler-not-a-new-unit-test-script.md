# 2026-08-20 — pairs.test.ts rides in test:filler, not a new unit-test script

Chosen: append `src/tests/pairs.test.ts` to the existing `test:filler` script, which CI's "Run
simplex test" step already executes on every simplex PR.

Alternatives rejected:

- _A dedicated `test:unit` script plus a new workflow step._ Cleaner taxonomy (the file is
  pure-unit while most of its neighbours in `test:filler` need RPC secrets), but it adds a script
  and a CI step to maintain for one ~3s file, and the oversized `--testTimeout` it inherits is
  harmless for unit tests. `fx.curve-payout.test.ts` set the precedent independently: it is also
  pure-unit and was wired into `test:filler` the same way.
- _Leave it unwired._ That is exactly how 12 stale failures sat on main unnoticed.

Known gap, deliberately out of scope here: several other pure-unit files (`book-crossed`,
`confirmation-policy`, `fx.one-sided-lp`, `fx.price-guard`, `paymaster-reserve`, …) still run in
no CI script. If they are ever wired up, a `test:unit` aggregate becomes worth its keep — move
`pairs.test.ts` into it then.
