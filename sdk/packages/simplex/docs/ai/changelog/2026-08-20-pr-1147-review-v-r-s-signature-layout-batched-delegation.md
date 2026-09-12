# 2026-08-20 — PR #1147 review: v,r,s signature layout + batched delegation approve (#1071)

Two changes from Seun's review of the PERMIT2 PR.

**Signature layout.** Mode 0x02's `paymasterData` now carries the Permit2 signature as explicit `uint8(v), bytes32(r), bytes32(s)` fields instead of a 65-byte `bytes` blob, mirroring the EIP-2612 mode 0x00 layout. Same 182-byte length; the contract reconstructs `abi.encodePacked(r, s, v)` for the Permit2 call. Client (`permit2.ts` / `provider/simplex.ts`) splits the signature before packing; fork/unit tests updated. Note: this keeps mode 0x02 65-byte-ECDSA-only, which the fixed length already enforced — no ERC-1271-length flexibility is lost.

**Batched delegation approve.** When Simplex falls back to a native EIP-7702 delegation (bundler/sponsored path unavailable), it now folds the one-time `approve(Permit2, max)` into that same set-code tx on no-permit chains, so the bootstrap costs one native tx (delegate + approve) instead of two. New `resolvePendingPermit2Approval` in `provider/simplex.ts` decides the token (null when a permit token, an approval already in place, no Permit2/PERMIT2-mode, or no fee-token balance yet); `DelegationService.sendDelegationTransaction` takes an optional approval and sets `to = token, data = approve(...)` with the authorization list attached (the delegation lands regardless of `to`). Best-effort: any resolver failure falls back to the plain self-call delegation. Scope is the native fallback only (per review).

Files: `evm/src/utils/SimplexPaymaster.sol`, `evm/tests/foundry/SimplexPaymasterPermit2ForkTest.t.sol`, `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/permit2.ts`, `src/services/DelegationService.ts`, `src/tests/services/SimplexPaymaster.test.ts`.
