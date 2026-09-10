# 2026-09-07 — Delegation batches the Permit2 approve into a direct tx before trying the bundler

`DelegationService.setupDelegation` now resolves the pending Permit2 approval up front and, when
one exists and the EOA can pay for a set-code tx, sends the batched delegate+approve first; the
bundler path follows only if that fails or when nothing is pending or native is short. The
native-balance check moved into `nativeCoversDirectTx`, shared by the early batched attempt and
the final plain fallback. Unit tests in `DelegationService.ordering.test.ts` pin the three
orderings.

Files: `src/services/DelegationService.ts`, `src/tests/services/DelegationService.ordering.test.ts`,
`docs/ai/Flow.md`, `docs/ai/Decisions.md`.
