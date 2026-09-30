# 2026-09-05 — Reword the vault "Minimum wallet balance" tooltip

The tooltip claimed the wallet float keeps liquidity available for fills, but fills draw from the
vault position atomically; the float is simply what Simplex never sweeps into the vault. Rewrote it
to say that, and added a USDC-only sentence noting USDC also pays paymaster gas, since that is the
real reason to hold USDC back. Requested by the maintainer while reviewing the redesign.
Files: `ui/src/components/VaultRowsEditor.tsx`, `docs/ai/ChangeLog.md`.
