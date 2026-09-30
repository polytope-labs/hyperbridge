# Vault save from the dashboard

`PUT /api/vault` validates the rows, then branches on whether the boot config produced a venue
(`op.vault`). With a venue: `reconfigure` re-hydrates every vault on-chain and swaps the set
atomically, the config is persisted, response `applied: true`. Without one: `vaultPreflight`
validates, the rows are persisted, response `applied: false, restartNeeded: true` — the venue is
wired into every strategy's funding list at boot and cannot be created later. `WalletTools.tsx`
shows a warning notice and toast for `restartNeeded`; `vaultConfigured` (which gates the Sweep and
Redeem buttons and the "Configured" badge) reflects the venue, not the file, until the restart.
