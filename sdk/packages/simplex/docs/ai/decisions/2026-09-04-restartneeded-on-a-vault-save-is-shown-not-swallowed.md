# 2026-09-04 — `restartNeeded` on a vault save is shown, not swallowed

Chosen: a persisted vault save with `restartNeeded: true` shows a warning notice in the panel and a
warning toast; `false` keeps the plain success toast. The hint copy names the restart when no venue
exists. This supersedes the 2026-09-03 "Treat a persisted vault save as UI success" decision.

Why: that decision called the restart advisory incorrect. It is not: `boot.ts` only constructs the
vault venue when the boot config has vaults, `handleVaultUpdate` only sets `restartNeeded` when
`op.vault` is absent, and in that case nothing in the process sweeps into or sources from the saved
rows. Reporting plain success turned a real restart requirement into a silent no-op — reproduced on a
running solver. What was wrong before was the presentation (an error for a successful save), so it
is a warning now, with the save still reported as saved.
