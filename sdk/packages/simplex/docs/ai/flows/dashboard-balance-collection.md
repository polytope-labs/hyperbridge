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
marked `partial` or `unavailable` with contextual issues. `OperatorOverview` therefore displays an
unavailable state instead of summing known values into a misleading total. Its headline stablecoin
metric sums only non-null available USDC/USDT values.

Within each network section, `OperatorOverview` sends every asset through `AssetBalanceCard`. The
card resolves its artwork through the shared `TokenIcon` mapping, presents total ownership first,
then wallet and vault sources, and isolates `available` as the operational amount available to fill.
The network's native balance stays in the section header. Partial and unavailable asset states keep
their null values explicit and use the warning accent instead of inheriting the token colour.

`Operator` passes the same snapshot to the Wallet page (`WalletTools`). `SendCard` matches the selected chain
and token address, displays the asset's canonical `available` value beside Amount, and uses the native
wallet row for native gas. After `/api/send` succeeds, it invokes the parent load path so the shared
balance snapshot, strategies, and configuration refresh immediately.
