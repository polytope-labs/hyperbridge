# 2026-09-05 — Backfill vault amounts on legacy ledger rows from their receipts

Sweep and redeem rows recorded before the ledger carried amounts showed dashes. At boot (15 s
after start, once vault states have hydrated), `backfillVaultLedger` (`src/data/ledger-backfill.ts`)
lists such rows (`ActivityStore.walletTxsWithoutAmounts`), reads each transaction's receipt through
`VaultFundingPlanner.describeTransaction`, which parses ERC-4626 `Deposit`/`Withdraw` events
(added to the ABI) from configured vaults (`vaultMovementsFromLogs` in `src/funding/vault/ledger.ts`),
and writes the underlying and share amounts back (`updateWalletTx`); a receipt touching several
vaults adds a row per extra vault. Best effort like the order backfill.
Files: `src/config/abis/Erc4626.ts`, `src/funding/vault/{VaultFundingPlanner,ledger}.ts`,
`src/data/{ledger-backfill,types,memory}.ts`, `src/data/sqlite/activity.ts`, `src/core/boot.ts`,
`src/tests/ledger-backfill.test.ts`, `docs/ai/{ChangeLog,Flow}.md`.
