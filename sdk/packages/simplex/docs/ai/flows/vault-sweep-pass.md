# Vault sweep pass

Verified 2026-09-04 against a running solver and `src/tests/funding/vault.test.ts`.

```
startSweeping()                       boot.ts after venue initialise; reconfigure() restarts it
  30s one-shot, then every sweepIntervalMs (default 5m)
    -> sweepExcessToVault()            also POST /api/vault/sweep and simplex.vaults.sweepNow()
         per chain, under sweepMutexByChain:
           thresholdScaled null        -> skip "sweeping-disabled" (no RPC)
           asset.balanceOf(solver)
           balance < threshold         -> skip "below-threshold"
           excess = balance - minBalance
           vault.maxDeposit(solver)
           min(excess, maxDeposit) <= 0 -> skip "deposits-closed"; warn once per closure, then debug
           else approve + deposit calls -> submitBatch (sponsored UserOp or native tx)
         returns { submitted?, skipped[] }; the pass merges chains into VaultSweepResult
```

`UiServer` formats the result into `VaultSweepDto` (token units) and `WalletTools.tsx` renders one
sentence from it; an empty pass is a warning only when a vault refused a due deposit. Independently,
`VaultLiquidityState.refresh` (balance snapshot, fill planning) reads `maxDeposit` and the overview
shows "Deposits closed" under In vault when it is zero.

Why a `StreamingYieldVault` refuses: `maxDeposit` is 0 while a tranche vests (`VEST`, 22h) and
opens only between `vestedAt` and the next `addYield` (at least `MIN_WINDOW`, 2h). A sweep tick
inside that window deposits; the rest silently skipped before this change.

`submitBatch` prefers a sponsored UserOp and only falls back to a native `executeBatch` on the
solver account. If that account is not 7702-delegated the fallback is refused (the call would
succeed and run nothing), so the pass rejects with an error and records no `onTx`; the periodic
timer's `.catch` logs it, as for any other batch failure. The same `submitBatch` serves the
shutdown `redeemAll`, which redeems `min(balanceOf, maxRedeem(solver))` per vault and skips a
zero-cap vault, so one liquidity-capped vault cannot revert the batch for the others.
