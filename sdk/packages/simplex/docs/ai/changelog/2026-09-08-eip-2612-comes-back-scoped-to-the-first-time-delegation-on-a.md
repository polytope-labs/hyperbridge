# 2026-09-08 — EIP-2612 comes back, scoped to the first-time delegation on a chain

The Permit2-only change above left a hole: with the 2612 path gone, a solver holding
stablecoins and zero native could no longer bootstrap a chain at all. Permit2 prefunding
needs a standing `approve(Permit2, max)`, and installing it took a native tx. The permit
mode was the only thing that had ever covered that case.

It is back, behind a `permitBootstrap` flag that exactly one caller sets. When
`DelegationService` finds no Permit2 allowance for the chain's fee token and that token
implements 2612, the delegation UserOp now does two things at once: it asks the paymaster
for PERMIT mode (`0x00`), and it carries `approve(Permit2, max)` in its own callData as an
ERC-7821 batch. The permit pays for the very op that installs the allowance every later op
authorizes against. One sponsored op, no native, and the account never signs a 2612 permit
again on that chain.

The flag is a fallback, not a preference. Even with it set, an allowance that already
covers the recommendation takes the Permit2 path — the unordered nonce is strictly better
and there is nothing to bootstrap. And without it, a permit-capable token like USDC still
bootstraps via a funded approve rather than a permit, because a fill's nonce must not be a
shared counter. That is the whole point of the scoping: the delegation is the one op per
chain that provably has no concurrent sibling, so a sequential nonce costs nothing there.

`resolvePendingPermit2Approval` now also reports `permitCapable`, which reorders
`setupDelegation`. A permit-capable token goes to the bundler first even when the EOA holds
native, since spending stablecoins beats spending native. Only a no-permit token (BNB Chain
pegged stables) still prefers the direct type-0x04 tx with the approve batched in — a native
tx is its only carrier. That probe also keeps the two approves from colliding: were the
callData approve attached on a no-permit token, the paymaster's own funded approve would land
first and turn it into the non-zero → non-zero change the USDT rule rejects.

The deposit gate prices a bootstrap op against `VERIFICATION_GAS_LIMIT_PERMIT` (250k,
restored) instead of `VERIFICATION_GAS_LIMIT_PERMIT2` (200k), since the mode is not known
until the allowance is read.

Files: `src/services/paymaster/permit.ts` and `src/config/abis/EIP2612.ts` (restored),
`src/services/paymaster/{index,types}.ts`, `src/services/paymaster/provider/simplex.ts`,
`src/services/{DelegationService,UserOpSender}.ts`, `src/cli/init/help-text.ts`.
Tests: `src/tests/services/{SimplexPaymaster,PaymasterSelection,DelegationService.ordering}.test.ts`.
Docs: `docs/ai/{ChangeLog,Decisions,Flow}.md`.
