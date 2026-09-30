# 2026-09-05 — Wallet ledger: action icon, Amount in and Amount out with token logos

The transaction history was time / chain / "order fill" / hash. Each row now leads with an action
icon coloured by kind (fill green, vault sweep and redeem blue, send amber), then Amount in (green, "+") and Amount out (red, "−") with token logos: a fill shows
the order's input received and output paid; a sweep shows the underlying out and the vault shares
in (share tokens wear a vault badge over the underlying's logo, e.g. stataUSDC over USDC; the share
symbol names the vault, so no counterparty text); a redeem the reverse; a send the token out to
the address. The sweep icon points up (sent into the vault), the redeem icon down. Chain gets its logo, the transaction is an open-in-explorer icon (hash on hover), time is clock + date. Backing this:
`VaultMovement` carries `shares`/`shareSymbol`/`shareDecimals` (`previewDeposit` for a sweep,
`previewRedeem` for a redeem; share-token ERC-20 metadata cached per vault; `previewDeposit` added
to the ERC-4626 ABI), `WalletTx` gains `tokenIn`/`amountIn` (SQLite migration adds `token_in`,
`amount_in`), boot records one row per movement, and `/api/wallet/history` returns `in`/`out`
`LedgerLeg`s plus `label`. Rows recorded before this keep the plain action label.
Files: `src/config/abis/Erc4626.ts`, `src/funding/vault/VaultFundingPlanner.ts`, `src/core/boot.ts`,
`src/data/{types,sqlite/activity}.ts`, `src/services/server/{UiServer,dto}.ts`,
`ui/src/operator/Wallet.tsx`, `ui/src/types.ts`, `ui/src/styles/operator.css`,
`docs/ai/{ChangeLog,Flow}.md`.
