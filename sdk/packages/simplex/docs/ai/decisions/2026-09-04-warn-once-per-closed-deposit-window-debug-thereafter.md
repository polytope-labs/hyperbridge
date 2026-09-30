# 2026-09-04 — Warn once per closed-deposit window, debug thereafter

Chosen: the first `deposits-closed` skip for a `chain:vault` logs at warn with wallet balance,
threshold, floor and `maxDeposit`; repeats log at debug until a deposit succeeds or the venue is
reconfigured, which clears the memo.

Alternative rejected: warn on every tick. The sweep runs every five minutes and a
`StreamingYieldVault` is closed ~22 hours a day, so that is ~260 identical warnings per vault per
day; the dashboard's "Deposits closed" flag and the sweep result cover "is it still closed".
Silence (the previous behaviour) was the bug.
