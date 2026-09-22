# 2026-09-22 — 0.16.5: pick up the 2026-09-22 mainnet SolverAccount from sdk 2.8.16

The filler reads the SolverAccount per chain from the sdk chain config, which now points every
mainnet chain at `0xd5535d4DeB17F050e52B6efda2fDe00435f39279` (PR #1317). Solvers re-delegate to it
through the SDK on upgrade. The posting test rig's stand-in for Base's SolverAccount uses the same
address. Simplex-desktop is bumped to 0.16.5 with it.
