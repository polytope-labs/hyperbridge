# Wallet ledger

`/api/wallet/history` merges `walletTxs` (sends from the dashboard; sweeps and redeems from
`VaultFundingPlanner.onTx`, one row per vault movement: `token`/`amount` = what left, `tokenIn`/
`amountIn` = what came back, `to` = vault) with `fills()` (observed on-chain fills by this filler).
The server maps each row to `in` and `out` `LedgerLeg`s — for fills from the order summary (input
received, output paid, raw base units + decimals); for vault rows from the decimal strings, with
`vault: true` and `icon` = the underlying's symbol for share tokens — and adds `label`, the
registry's vault name for `to`. `Wallet.tsx` renders an action icon per kind, Amount in (green)
and Amount out (red) with token logos (a vault badge on share tokens; sends show the recipient,
vault rows nothing more), the chain with its logo, the explorer link and the time.
Rows recorded before amounts existed are backfilled at boot: `backfillVaultLedger` reads each
receipt via `VaultFundingPlanner.describeTransaction` (ERC-4626 Deposit/Withdraw logs from
configured vaults, share metadata from the cached ERC-20 reads) and updates the row.
