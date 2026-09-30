# 2026-09-04 — Say why a vault sweep did nothing, and restore the restart notice for vault saves

Diagnosed on a running solver: the periodic sweep ran every five minutes against a wallet holding
165k cNGN over a threshold of 10, and never deposited. The ycNGN vault is a `StreamingYieldVault`,
whose `maxDeposit` returns 0 while a tranche vests (22h of every 24h cycle); the sweep clamped the
deposit to 0 and `continue`d with no log line, and the dashboard's Sweep now button reported
"Sweep executed" on the same silent path.

`VaultFundingPlanner.sweepExcessToVault` now returns a `VaultSweepResult`: the batches it submitted
(with per-vault deposit amounts) and every vault it skipped with a reason — `sweeping-disabled`,
`below-threshold`, or `deposits-closed` with the wallet balance, threshold and `maxDeposit` it saw.
A `deposits-closed` skip logs a warning the first time per closure and debug on every repeat until
a deposit goes through or the venue is reconfigured. `VaultLiquidityState.refresh` reads
`maxDeposit(solver)` alongside `maxWithdraw`, so the balance snapshot carries `acceptsDeposits` per
vault and the overview shows "Deposits closed" under the asset's In vault figure.

`POST /api/vault/sweep` returns the pass as `VaultSweepDto` with amounts formatted in token units.
The vault panel turns that into one sentence — what was deposited, or which vault refused and how
far over its threshold the wallet is, or that balances are simply below their triggers.

Vault saves: the 2026-09-03 change that dropped the `restartNeeded` handling is reverted in
substance. The server sends `restartNeeded: true` only when the filler booted without a vault venue;
the rows are persisted but nothing in the process uses them until a restart, so the panel now shows a
warning notice and toast saying so, and the hint copy once again says edits re-hydrate the running
venue "after a restart" when no venue exists.

Also fixed the branch's declaration build: `InitChainMeta` gained the optional `note` the setup API
and terminal wizard were already reading.

Files: `src/funding/types.ts`, `src/funding/vault/{VaultFundingPlanner,VaultLiquidityState}.ts`,
`src/services/BalanceProvider.ts`, `src/services/server/{UiServer,dto}.ts`, `src/simplex.ts`,
`src/index.ts`, `src/cli/init/chains.ts`, `ui/src/types.ts`,
`ui/src/operator/{Operations,OperatorOverview}.tsx`, `ui/src/styles/{controls,operator}.css`.
Tests: `src/tests/funding/vault.test.ts`, `src/tests/ui-server.test.ts`. Docs:
`docs/content/developers/sdk/{simplex,api/simplex}.mdx`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
