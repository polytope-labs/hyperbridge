# ChangeLog

AI-maintained log of code changes in `sdk/packages/simplex`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog (`CHANGELOG.md` at the package root is the published release log).

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-19 — Security-review fixes: stake recovery, postOp gas band (#1071)

A threat-model audit of the paymaster (fork-executed, not just read) produced three fixes carried here.

**Stake recovery.** `PaymasterCore.addStake` was ungated while `unlockStake`/`withdrawStake` route through `_authorizeWithdraw()`, which reverts unconditionally, and no governance request kind covered stake — so staked native was permanently unrecoverable. Verified against the live deployments: 0.1 native was already stuck on each of Ethereum, BSC and Arbitrum, and any unprivileged address could call `addStake{value: 1 wei}(type(uint32).max)` to stretch the unstake delay from one day to 136 years, defeating even a future upgrade. Added `RequestKind.UnlockStake` / `RequestKind.WithdrawStake` (discriminators 5 and 6, both empty-payload, destination always the treasury) and gated `addStake` to the treasury.

**postOp gas band.** The EntryPoint penalises the unused part of `paymasterPostOpGasLimit` after fixing the cost handed to `postOp`, so that penalty is never billed to the user; it waives the penalty entirely while `gasLimit <= gasUsed + 40_000`. The SDK pinned the limit at the contract's 100k cap while postOp actually needs ~8-12k, so roughly half the per-op margin was being burned. `MAX_POST_OP_GAS_LIMIT` is now 40_000 — the largest unconditionally penalty-free value — and a `MIN_POST_OP_GAS_LIMIT` of 30_000 closes the refund-underflow window that an unbounded-below limit left open. Fork-measured margin at the cap rose from 9,053,104,589,778 to 21,354,113,277,112 wei per op, and is now identical at the cap and the floor.

The SDK's shared `POST_OP_GAS_LIMIT` split into `POST_OP_GAS_LIMIT_SIMPLEX` (40k) and `POST_OP_GAS_LIMIT_CIRCLE` (100k) — it was feeding both paymasters, and Circle's is a different contract whose postOp was never measured here.

Files: `src/services/paymaster/types.ts`, `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/provider/circle.ts`, `src/tests/services/UserOpSender.test.ts`, plus `evm/src/utils/SimplexPaymaster.sol`, `evm/tests/foundry/{SimplexPaymasterTest,SimplexPaymasterPermit2ForkTest,SimplexPaymasterGasGriefTest,SimplexPaymasterStakeLockForkTest}.t.sol` and `modules/pallets/intents-coprocessor/src/{lib,types,weights}.rs`.

## 2026-08-18 — Permit2 mode for the Simplex paymaster (#1071)

The Simplex paymaster client gained mode `0x02 PERMIT2`. On chains whose fee token has no EIP-2612 permit (BSC pegged USDC/USDT), the solver previously kept a $5 standing allowance to the paymaster and refilled it with a native-funded `approve` every time it dipped under $2. Now the bootstrap is a single funded `approve(Permit2, max)` per token, and every subsequent op carries a per-op Permit2 `PermitTransferFrom` signature (spender = paymaster, amount = the same $5 cap, random 256-bit nonce, 1 hour deadline) packed as `mode(1) + token(20) + permitAmount(32) + nonce(32) + deadline(32) + signature(65)`. Selection order in `buildSimplexPaymasterData`: EIP-2612 PERMIT when the token supports it (unless `skipPermit`), then PERMIT2 when the token is already approved to Permit2, then APPROVE while a legacy paymaster allowance is still in place, then the bootstrap approve. Mode 2 is only used when the paymaster deployment exposes `PERMIT2()` (older deployments reject it with `InvalidMode`), probed once per paymaster address and cached. `PaymasterOptions.forceApproveMode` was renamed to `skipPermit` since PERMIT2 stays allowed for delegation ops; `ensureCappedApproval` became the generic `sendFundedApprove`.

Live probe on Base Sepolia through Alchemy's bundler (deployment script `evm/script/SimplexPaymasterPermit2Probe.s.sol`, env-gated suite `src/tests/services/SimplexPaymasterPermit2.probe.test.ts`): a fresh EOA's sponsored EIP-7702 delegation and a follow-up no-op from the delegated account were both accepted in mode 2 (`Permit2Executed` on-chain), answering the ERC-7562 question for that bundler. The very first op right after the bootstrap approve was rejected `AA33` twice in a row until the approve was one more block old, so `sendFundedApprove` now waits for two confirmations.

Files: `src/services/paymaster/permit2.ts` (new), `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/types.ts`, `src/services/paymaster/index.ts`, `src/services/UserOpSender.ts`, `src/services/DelegationService.ts`, `src/config/abis/SimplexPaymaster.ts`, `src/tests/services/SimplexPaymaster.test.ts`, `src/tests/services/UserOpSender.test.ts`, `src/tests/services/SimplexPaymasterPermit2.probe.test.ts` (new).
