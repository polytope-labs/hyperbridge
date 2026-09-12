# 2026-09-08 — The Permit2 bootstrap keys on the allowance, not on the delegation

Chosen: `setupDelegation` calls `ensurePermit2Allowance(chain)` before its already-delegated
early return, so the permit-funded `approve(Permit2, max)` op runs whenever the allowance is
missing — not only on the boot where the account happens to be undelegated.

Alternative considered: leave the bootstrap folded into the first-time delegation op only, and
tell operators to fund native dust once per chain on upgrade.

Why: that alternative is the workaround the 2026-09-02 `skipPermit` entry already considered and
rejected, and this time it would be worse. It is not "once per fresh solver" — an account
delegated by 0.15 has no Permit2 allowance on any chain, because that release short-circuited to
PERMIT mode before reading one. So every existing deployment would need native everywhere, and
the zero-native solver the paymaster exists to serve would lose sponsorship entirely, silently,
on its first order after the upgrade.

The deeper point is that delegation and bootstrap were conflated. Being delegated says the EOA
has SolverAccount code; it says nothing about whether the fee token is approved to Permit2. Two
independent facts were being read off one check. Keying the bootstrap on the allowance —
`resolvePendingPermit2Approval`, which reads it — makes the op idempotent and self-healing: it
runs on any boot where the allowance is missing and skips otherwise, whatever the delegation
state.

Kept best-effort rather than fatal: a bundler outage during the bootstrap must not turn a
correctly delegated account into a setup failure, and the `sendFundedApprove` fallback still
serves any solver holding native.
