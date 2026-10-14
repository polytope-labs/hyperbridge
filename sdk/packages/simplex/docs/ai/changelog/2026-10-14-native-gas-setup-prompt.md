# 2026-10-14 — Prompt for the native gas a chain needs to finish setup

The paymaster pays every sponsored transaction's gas through Permit2. That needs a one-time
Permit2 approval of the fee token. A token with an EIP-2612 permit pays for that approval
itself. BNB Chain's USDC and USDT have no permit. Their approval is an ordinary transaction,
paid in BNB. A wallet funded with stables alone stays unsponsored on BNB Chain, and nothing said
why.

## Balance rows carry `nativeSetup`

`nativeSetupNeed` in `src/services/native-setup.ts` returns what a chain still needs, or null.
`BalanceProvider` calls it on each refresh, after reading the native balance. The result is set
on the chain's `ChainBalanceRow` as `nativeSetup`:

- `required`: native to send, in whole units. It is twice the current cost, so a gas-price rise
  does not leave it short.
- `feeToken`: `"USDC"` or `"USDT"`, the token whose approval is pending.

The cost is the delegation transaction with the approve folded in
(`DELEGATION_TX_GAS_FLOOR + BATCHED_APPROVE_GAS`) for a wallet that has not delegated yet. It is
`APPROVE_TX_GAS` for one that has. A chain needs nothing when the wallet already holds that much
native, when it holds no fee token, when the approval is in place, or when the fee token has a
permit. A wallet that holds the costlier amount skips the approval and delegation reads. The
check is best-effort: a failure leaves `nativeSetup` unset for that refresh.

## Where it shows

- Dashboard overview: a warning per chain with `nativeSetup`, naming the amount and the native
  symbol, with the filler wallet address to copy. It clears on the next refresh once the wallet
  holds enough. The approval itself still goes out as before: batched into the next delegation
  attempt, or as the funded approve ahead of the first sponsored transaction on that chain.
- Setup wizard, Review step: when BNB Chain is enabled, the funding line adds that BNB Chain also
  needs a little BNB for a one-time approval.
