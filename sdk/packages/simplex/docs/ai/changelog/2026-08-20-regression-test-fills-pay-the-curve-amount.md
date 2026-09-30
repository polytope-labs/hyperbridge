# 2026-08-20 — Regression test: fills pay the curve amount

The payout fix below restored `targetOutput = policyMaxOutput`, but nothing asserted the payout —
the original regression landed silently precisely because no test pinned `calculateProfitability`'s
cached outputs (the figure `prepareBidUserOp` signs into the bid) against the curve.
`fx.curve-payout.test.ts` drives `calculateProfitability` with mocked chain access and pins the
three sizing outcomes: an uncapped leg pays the curve amount, not the user's requested amount; a
capped leg pays the capped slice's worth at the curve, not the user's pro-rata ask; and a
balance-limited leg pays what the wallet covers — a full fill when that still clears the ask.
Verified to fail on the pre-fix clamp: reintroducing `min(policyMaxOutput, desiredOutput)` fails
all three cases with exactly the clamped amounts.

The file is added to `test:filler`, the script CI actually runs — a payout test the CI never
executes would repeat the original failure mode. (`pnpm test` runs the full suite, but CI does not;
`pairs.test.ts`, which exercises the profit gates, is in no CI script at all and currently fails 12
of its cases on main — 9 predating #1154 and 3 from #1154 unrationing phantom probes without
updating the cap expectations. Repairing that suite and wiring it into CI is a separate task.)

Files: src/tests/strategies/fx.curve-payout.test.ts (new), package.json, docs/ai/ChangeLog.md.
