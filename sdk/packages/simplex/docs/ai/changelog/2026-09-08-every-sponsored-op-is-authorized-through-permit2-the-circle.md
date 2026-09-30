# 2026-09-08 — Every sponsored op is authorized through Permit2; the Circle paymaster is gone

Simplex no longer signs EIP-2612 permits. The Circle paymaster provider, the `permit.ts` signer
and the `EIP2612` ABI are deleted, and `buildPaymasterAndData` has one candidate left: the Simplex
paymaster in `PERMIT2` mode (`0x02`). The `tokenSupportsPermit` probe and `buildPermitMode` went
with them, so a permit-capable token like USDC now takes the same path as a BSC pegged stable.

The reason is nonce shape. EIP-2612 keeps one sequential counter per owner, so two permits signed
back to back carry the same value and only one can land — a hard serialization point on the
solver's fee token, made worse by the fact that `buildPermitMode` signed a fresh permit for every
op (the standing-allowance mode `0x01` was retired from the contract, so nothing reused an
allowance). Permit2's `SignatureTransfer` nonces are an unordered bitmap: `randomPermit2Nonce`
draws 256 random bits, `_useUnorderedNonce` flips that one slot, and concurrent ops never collide.
That is the property worth having as soon as ops stop running single-file.

Dropping Circle follows from the same decision — it is an EIP-2612-only paymaster, so there was
nothing left for it to do. Its verification/postOp constants and the `paymasterVerificationGasLimit`
override existed solely to tune it, and are removed from `PaymasterOptions`,
`SponsoredUserOpRequest` and the three call sites that passed a value. `PaymasterDataResult.type`
narrows to `"simplex" | "none"`, and `depositShortfall` loses its candidate label. The deposit gate
now prices against `VERIFICATION_GAS_LIMIT_PERMIT2` (200k) instead of the retired
`VERIFICATION_GAS_LIMIT_PERMIT` (250k), so a near-empty deposit sponsors slightly more often.

Two consequences worth stating plainly. A solver on a chain it has never used needs native dust
once per fee token for `approve(Permit2, max)` — the txless 2612 path that used to cover a
zero-native bootstrap is gone; `resolvePendingPermit2Approval` no longer skips permit-capable
tokens, so the delegation tx batches that approve in for every token, which is the cheapest way to
pay it. And Optimism has a `CirclePaymaster` but no `SimplexPaymaster` in the SDK chain registry,
so it now falls back to native gas / the EntryPoint deposit until a Simplex paymaster is deployed
there.

`evm/src/utils/SimplexPaymaster.sol` is untouched: mode `0x00` stays on the permissionless
contract for other integrators, simplex just never sends it.

Files: `src/services/paymaster/{index,types}.ts`, `src/services/paymaster/provider/simplex.ts`,
deleted `src/services/paymaster/provider/circle.ts`, `src/services/paymaster/permit.ts`,
`src/config/abis/EIP2612.ts`; `src/services/{DelegationService,UserOpSender,TokenSender,FillerConfigService,ContractInteractionService}.ts`,
`src/funding/vault/VaultFundingPlanner.ts`, `src/core/{boot,filler}.ts`, `src/cli/init/help-text.ts`.
Tests: deleted `src/tests/services/CirclePaymaster.test.ts`; rewrote
`src/tests/services/PaymasterSelection.test.ts`; `src/tests/services/{SimplexPaymaster,SimplexPaymasterPermit2.probe,paymaster-reserve,DelegationService}.test.ts`,
`src/tests/pairs.test.ts`, `src/tests/strategies/fx.curve-payout.test.ts`.
Docs: `docs/ai/{ChangeLog,Decisions,Flow}.md`.
