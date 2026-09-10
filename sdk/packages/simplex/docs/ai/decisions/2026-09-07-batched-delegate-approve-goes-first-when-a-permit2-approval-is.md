# 2026-09-07 — Batched delegate+approve goes first when a Permit2 approval is pending

Chosen: `setupDelegation` tries the direct set-code tx with `approve(Permit2, max)` batched in
before the sponsored path, whenever `resolvePendingPermit2Approval` reports a pending approval
and the EOA holds native for one tx. Supersedes the 2026-08-20 "delegation-batched approve is
native-fallback only" scoping. Reason (review request): on a chain whose fee token has no permit,
the sponsored path cannot use the paymaster until the Permit2 approve has landed, so it sent a
native-funded approve and then the sponsored op, two transactions, where the batched direct tx
does both in one. The approve cannot ride inside the sponsored op itself because the paymaster
prefunds during validation, before the op's calldata runs. When native is short the order is
unchanged: the bundler path still gets its chance (Circle may sponsor), and the plain direct tx
stays the last fallback with the same deficit log.
