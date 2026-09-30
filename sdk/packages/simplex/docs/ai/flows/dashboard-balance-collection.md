# Dashboard balance collection

`boot.ts` hydrates `VaultFundingPlanner` before constructing `BalanceProvider`, passes that planner
through the narrow `VaultBalanceSource` interface, and awaits `BalanceProvider.start()`. Startup now
performs the first refresh before boot returns; later refreshes run on the configured interval.

For each chain, `BalanceProvider` resolves configured USDC/USDT and unique exotic assets, reads the
solver's native and token wallet balances, then merges the planner's vault rows by underlying asset.
`VaultFundingPlanner.getBalanceSnapshot()` takes the same per-chain mutex as fill planning, refreshes
`VaultLiquidityState`, and returns base-unit position, remaining withdrawable assets, and wallet
reserve. The API boundary formats those once and derives:

- `total = wallet + vaultPosition`
- `available = max(wallet - walletReserve, 0) + vaultAvailable`

If a token read or the vault snapshot fails, the affected aggregate is `null` and the response is
marked `partial` or `unavailable` with contextual issues. The dashboard therefore displays an
unavailable state instead of summing known values into a misleading total. `OperatorBalances`
exports `availableStablecoins(assets, snapshotStatus)` for that rule — null if any contributing
`available` is null, `0` only when the snapshot is fresh and the asset list holds no USDC or USDT —
and both the Overview's headline stablecoin metric and the switcher's per-network figure go through
it.

`OperatorOverview` renders the metrics strip and the runtime controls, then hands the snapshot to
`OperatorBalances`, which owns the whole balances section. It shows one network at a time:

- The selected network is `picked` when it still matches a configured chain, else the first one.
  Nothing about a network's health is derived: the snapshot has no such field, and the
  section-level notice already reports failed reads.
- `tokenTotals()` walks the chains in config order and accumulates one entry per symbol: `total`
  across every network and `onSelected` for the selected one, both set to null the moment a
  contributing `available` is null. The first four entries become the totals strip (`+N more …`
  names the rest); each cell renders the total, a share bar floored at 2%, and the share caption.
- The switcher is `AppSelect` with a per-network `description` (filling or observing) and
  `trailing` (that network's available USDC+USDT, `Unavailable` when a contributing read failed).

The selected network's assets then go through `AssetBalanceCard`, unchanged: it resolves artwork
through the shared `TokenIcon` mapping, presents total ownership first, then wallet and vault
sources, and isolates `available` as the operational amount available to fill. The network's native
balance sits in the focus line above the cards. Partial and unavailable asset states keep their null
values explicit and use the warning accent instead of inheriting the token colour. The section-level
"Some balances are unavailable" notice covers the whole snapshot, including networks that are not
currently selected.

`Operator` passes the same snapshot to the Wallet page (`WalletTools`). `SendCard` matches the selected chain
and token address, displays the asset's canonical `available` value beside Amount, and uses the native
wallet row for native gas. After `/api/send` succeeds, it invokes the parent load path so the shared
balance snapshot, strategies, and configuration refresh immediately.
