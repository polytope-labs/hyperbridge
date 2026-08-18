# Flow

AI-maintained map of how code paths in `sdk/packages/simplex` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## Paymaster selection for a sponsored UserOp

Entry points: `UserOpSender.trySendSponsored` (delegation, vault sweeps/redeems, token sends) and `ContractInteractionService.prepareBidUserOp` (bids). Both call `buildPaymasterAndData` in `src/services/paymaster/index.ts`.

1. `buildPaymasterAndData` tries the Circle paymaster first when it is configured and the solver holds at least 1 USDC (`provider/circle.ts`), then the Simplex paymaster (`provider/simplex.ts`), else returns `type: "none"` and the caller falls back to native / the EntryPoint deposit.

2. `buildSimplexPaymasterData` picks the first configured stablecoin (USDC, then USDT) with a balance of at least one whole token, then the authorization mode:
   - `tokenSupportsPermit` (probes `version()`) and not `skipPermit`: `buildPermitMode` — if the allowance to the paymaster already covers the $5 permit amount, APPROVE mode; else sign an EIP-2612 permit (`permit.ts`, `deadline = maxUint256`) and pack mode `0x00` (150 bytes).
   - Otherwise resolve `permit2 = configService.getPermit2Address(chain)` and require `paymasterSupportsPermit2` (a cached `PERMIT2()` read on the paymaster). Read the solver's allowances to the paymaster and to Permit2 in parallel, then:
     - Permit2 allowance at or above $5: `buildPermit2Mode` — random nonce, `deadline = now + PERMIT2_DEADLINE_SECONDS`, `signPermit2Transfer` (`permit2.ts`, canonical v4 typed data, 65-byte signature) and pack mode `0x02` (182 bytes) with `VERIFICATION_GAS_LIMIT_PERMIT2`.
     - Paymaster allowance at or above $2: APPROVE mode `0x01`, no tx.
     - Else bootstrap with `sendFundedApprove` (native balance pre-check, then `approve` from the solver EOA, wait one confirmation): `approve(Permit2, max)` then PERMIT2 when Permit2 is usable, else `approve(paymaster, $5)` then APPROVE.

3. Back in `UserOpSender.trySendSponsored`, the EIP-7702 authorization thunk is resolved only after paymaster data is built, because the bootstrap approve is a tx from the same EOA and an authorization signed earlier would carry a stale nonce (bundler reject). Then bundler gas price, `EntryPoint.getNonce`, estimation (or the caller's fixed limits), signing of the packed userOp typed data, `eth_sendUserOperation`, and receipt polling.

4. On chain (`evm/src/utils/SimplexPaymaster.sol`): `_fetchDetails` validates the mode byte and token registry and, for mode 2, returns the permit deadline as `validUntil`; `_prefund` pulls the prefund through `Permit2.permitTransferFrom` (owner = `userOp.sender`, to = paymaster, requestedAmount = oracle-derived prefund, capped by the signed amount) and postOp refunds the unused part to the sender.
