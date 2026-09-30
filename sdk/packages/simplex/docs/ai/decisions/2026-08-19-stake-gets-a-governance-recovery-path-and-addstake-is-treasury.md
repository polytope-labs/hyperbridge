# 2026-08-19 — Stake gets a governance recovery path, and `addStake` is treasury-only (#1071)

Chosen: two new empty-payload request kinds (`UnlockStake` = 5, `WithdrawStake` = 6, always paying out to the treasury) and an `addStake` override gated to the treasury.

The alternative — leave stake unrecoverable and simply never stake — is not available: bundlers require a staked paymaster for the storage access this contract performs, so staking is effectively mandatory and was already done on three chains. Two kinds rather than one because the EntryPoint requires `unlockStake()` and then a wait of `unstakeDelaySec` before `withdrawStake()` will succeed; a single request could not span that delay.

Gating `addStake` is the half that cannot be deferred. The EntryPoint only ever lets `unstakeDelaySec` grow and resets any pending unlock on every `addStake`, so while the function is open an unprivileged caller can push the delay to 136 years and cancel unlocks indefinitely — which would defeat the recovery path being added here.
