# 2026-09-04 — A sweep pass reports its outcome instead of resolving to void

Chosen: `sweepExcessToVault` returns `VaultSweepResult` — submissions plus a `skipped` entry per
vault with a reason and the numbers behind it — and the UI endpoint forwards it formatted. The
result is the contract; logging is a side channel for the periodic timer.

Alternatives rejected: logging alone (the dashboard's Sweep now button would still say "executed"
for a no-op, and the library consumer has nothing to branch on); a boolean "did anything" (cannot
distinguish a wallet under its trigger from a vault refusing deposits, which is the whole diagnosis).
