# 2026-10-14 — USDT0 permits, and sends that pay their own way

## USDT can pay for a solver's first delegation on Polygon and Arbitrum

A solver's first delegation on a chain can pay for itself through the SimplexPaymaster: it signs an
EIP-2612 permit for the fee token, so it needs no native gas. That only worked for USDC.

USDT on Polygon and Arbitrum is USDT0. It has a permit, but no `version()`, and Simplex detected a
permit by calling `version()`. So a solver holding USDT but no native fell back to a funded
`approve` and stalled with "send native dust". Polygon USDT0 also signs over a non-standard domain:
`name, version, verifyingContract, salt` with `salt = bytes32(chainId)` and no `chainId` field.

`resolvePermitDomain` in `services/paymaster/permit.ts` now finds the domain a token's permit
checks. It reads `DOMAIN_SEPARATOR()`, builds the standard and the salt-style domain from the
token's name and version (trying "1" and "2" when there is no `version()`), and returns the one
whose hash matches. `tokenSupportsPermit` is true only when one matches, and `signEip2612Permit`
signs over that domain. A token whose separator matches neither, or that has none, such as
Ethereum USDT, still falls back to the funded approve.

## Sends install the Permit2 approval themselves

Every sponsored op except the first delegation authorizes through Permit2. A wallet that delegated
before it held a fee token never got the Permit2 approval, so its sends needed native gas for an
`approve` first, even when the fee token had a permit.

`TokenSender.submitCalls` now checks `resolvePendingPermit2Approval` before a sponsored batch. When
the wallet has never approved Permit2 for its fee token and that token has a permit, the batch
starts with `approve(Permit2, max)` and the op asks the paymaster for PERMIT mode
(`permitBootstrap`). The op pays its gas with the permit and installs the approval; later ops use
Permit2. The native fallback sends the original calls without the approve. The BRIDGE top-up uses
the same `submitCalls`, so it gets this too.

## Validation on a Polygon fork

- A 7702-delegated solver holding only USDT0 passed the paymaster's PERMIT-mode validation in
  147,064 gas; USDC took 158,565. Both are under the 250,000 bootstrap limit.
- A wallet holding only USDT0 and no POL sent twice through the real EntryPoint `handleOps`, from
  the registered bundler. The first send ran in PERMIT mode and installed the approval (483,852
  gas); the second ran in Permit2 mode (381,539 gas). The wallet's POL stayed at zero.
