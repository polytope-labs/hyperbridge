# 2026-09-04 — `maxDeposit` is read on every vault refresh

Chosen: `VaultLiquidityState.refresh` reads `maxDeposit(solver)` next to `previewRedeem` and
`maxWithdraw`, and the balance snapshot exposes it as `acceptsDeposits`.

Alternative rejected: reading it only inside the sweep. That leaves the overview unable to explain
an idle sweep without the operator pressing Sweep now. One extra view call per vault per refresh
(once a minute, and per fill plan) is cheap next to the two already made.
