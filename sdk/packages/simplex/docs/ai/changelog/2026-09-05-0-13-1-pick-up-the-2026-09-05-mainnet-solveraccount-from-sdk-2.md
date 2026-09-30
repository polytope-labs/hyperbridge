# 2026-09-05 — 0.13.1: pick up the 2026-09-05 mainnet SolverAccount from sdk 2.8.11

No code change in this package. The filler reads the SolverAccount per chain from the sdk chain
config, which now points every mainnet chain at `0x7cb55539d1144F62422099c3FA3405092022c88C`
(PR #1207). Bumped together with the sdk so the published simplex resolves the matching sdk.

Files: `package.json`, `docs/ai/ChangeLog.md`.
