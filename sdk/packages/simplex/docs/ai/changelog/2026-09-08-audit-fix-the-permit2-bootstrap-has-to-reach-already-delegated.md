# 2026-09-08 — Audit fix: the Permit2 bootstrap has to reach already-delegated accounts

An adversarial audit of the two entries below found the bootstrap unreachable exactly where it
was most needed. `permitBootstrap` was set in one place — the first-time delegation op — and
`setupDelegation` returns the moment `isDelegated(chain)` holds. Delegation and bootstrap are
separate facts: an account delegated by 0.15 has NO Permit2 allowance, because that release
took the 2612 branch before it ever read one and its `resolvePendingPermit2Approval` explicitly
skipped permit-capable tokens. So every already-running solver would upgrade into the state the
bootstrap exists to prevent — and with zero native, every fill, bid and sweep on that chain
would lose sponsorship with `send native dust`. That is the same trap the 2026-09-02
`skipPermit` entry in Decisions.md records hitting in production on Base and Arbitrum, which
explicitly rejected "fund native dust on every chain" as the remedy.

`ensurePermit2Allowance(chain)` now runs before that early return. It sends the same
permit-funded ERC-7821 `approve(Permit2, max)` op, without the authorization an already-delegated
account does not need, and both carriers share one `bootstrapCallData` encoder so they cannot
drift. It is best-effort: a failure logs a warn and leaves the first sponsored op to
`sendFundedApprove`, so a solver holding native is unaffected.

Two more audit findings fixed. `sendFundedApprove` is now wrapped in `ensureFundedApprove`,
which dedups in-flight approvals by (chain, token, owner) — the bid path and the vault /
token-send path are scheduled independently, and routing every chain through Permit2 made the
missing-allowance state reachable on all of them, so two concurrent `writeContract` calls could
resolve the same pending nonce and drop one. And the bootstrap callData test asserted only that
two addresses appeared somewhere in the payload; it passed with the approve amount mutated to
1 wei, which would have stranded the chain permanently (`resolvePendingPermit2Approval` refuses
any non-zero allowance). It now asserts the exact bytes, and was checked against that mutation.

The completeness critic also flagged that `signEip2612Permit` returned the signer's bytes raw
while `signPermit2Transfer` ran through `normalizeSignature65`, even though `buildPermitMode`
splits v straight out of the hex for a contract expecting v in {27,28}. Pre-existing — identical
at `ebe157c70` — but this change makes that path the sole bootstrap for every solver, so it is
normalized now too. A correct 65-byte signature passes through unchanged.

Audit findings accepted without a code change: Optimism has a `CirclePaymaster` and no
`SimplexPaymaster`, so it degrades to native gas and an EntryPoint deposit — the intended
consequence of dropping Circle, already recorded below. A stale non-zero Permit2 allowance below
$5 still has no bootstrap route. `THRESHOLD_USD` is now dead. The `PERMIT2_DEADLINE_SECONDS`
(3600) bid expiry is not reconciled with the operator-configurable bid tenor.

Files: `src/services/DelegationService.ts`, `src/services/paymaster/{permit,provider/simplex}.ts`.
Tests: `src/tests/services/DelegationService.ordering.test.ts` (4 new cases; the regression guard
was verified to fail against the pre-fix early return). Docs: `docs/ai/{ChangeLog,Flow}.md`.
