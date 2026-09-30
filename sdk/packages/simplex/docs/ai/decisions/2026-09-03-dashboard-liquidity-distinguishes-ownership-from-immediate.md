# 2026-09-03 — Dashboard liquidity distinguishes ownership from immediate availability

Chosen: expose the funding planner's hydrated ERC-4626 state through a narrow read-only snapshot and
make the balance API's per-asset projection canonical. `total` is wallet balance plus the owned vault
position; `available` is wallet balance above the configured reserve plus the vault's current
`remaining`, which already accounts for `maxWithdraw` and pending-fill reservations. The UI shows
wallet, vault, and available values separately and uses available USDC/USDT for its headline metric.

The vault snapshot refreshes under the same per-chain mutex used by withdrawal planning, so the
operator view cannot race reservation accounting. RPC and vault-source failures produce explicit
partial/unavailable states and null aggregates; missing values are never guessed or silently folded
into a smaller total. The existing `usdc`, `usdt`, and `exotics` wallet fields remain as a
backward-compatible projection for SDK consumers.

Alternatives rejected: adding vault positions directly to the old wallet fields would make owned
funds look immediately spendable; calculating vault balances independently in the dashboard would
duplicate funding-domain rules and race live reservations; treating failed reads as zero would turn
an outage into a false low-balance alarm.
