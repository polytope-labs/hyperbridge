# 2026-10-16 — Limit-order budgets and vault liquidity come from the chain

Decided: simplex keeps no local holds against limit orders and no local reservations against vault
liquidity. The chain is the only record of what an order may still pay out and of what a vault will
still release.

Both limits are already enforced on chain, in the same transaction as the fill:

- `debitOrder` runs in the solver's ERC-7821 batch after `fillOrder` and reverts a fill that would
  take the limit order's tally past its cap (`LimitOrderExceeded`).
- A vault `withdraw` past `maxWithdraw(solver)` reverts, and with it the whole fill batch.

Bid sizing already reads both on every evaluation: `FXFiller` sizes a bid from the order's on-chain
room (`limitOrderSpent` → `budgetRoom`), and `VaultFundingPlanner.planWithdrawalForToken` refreshes
`maxWithdraw` under its per-chain mutex before every plan. `remaining` is brought down to the tally
after each fill and by `reconcileTallies`.

A local hold could only add a second, drifting copy of those figures. It had to be taken, released
on every loss path, converted on every win, made idempotent against settling twice and kept in step
across restarts, and each of those was a way to refuse good fills or misstate an order.

One evaluation never double-plans: the bids it builds for one swap share running totals of the
wallet pool, each limit order and each vault, which are dropped when it returns. These are sizing
within one evaluation, not holds.

The accepted cost: two bids from separate evaluations can each offer the whole room an order or a
vault has left. If both are selected, the second fill reverts on chain and its gas is spent. The
order never pays out more than its cap and the vault never releases more than it holds.

What still lives locally is provenance, not inventory: each bid row's `pricedBy` names the limit
order that priced it, for attributing a fill and for `byLimitOrder`. It is written once and never
claimed, cleared or released.
