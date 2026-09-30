# 2026-09-03 — Send funds reuses the canonical balance snapshot

Chosen: pass the dashboard's already-polled `BalanceSnapshot` into Operations and derive the selected
asset display by chain plus token address. ERC-20 sends show `available`, which accounts for the wallet
reserve and currently withdrawable vault assets; native sends show the chain's native wallet amount.
Unknown custom tokens and failed reads show Unavailable instead of an inferred zero. Refresh the shared
snapshot after a successful send rather than introducing a second polling loop in the drawer.

Alternative rejected: fetching balances independently in `SendCard` would duplicate polling and allow
the drawer to disagree with the dashboard; displaying `total` would include vault ownership that may
not currently be withdrawable and would overstate operational liquidity.
