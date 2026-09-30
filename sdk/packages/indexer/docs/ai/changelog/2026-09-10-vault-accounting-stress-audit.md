# 2026-09-10 — Stress-test vault capital flows and harden event failure handling

Exercise real vault handlers, ABI decoding and generated models against an independent cash-flow
model: Simplex redeem/send/sweep shapes, empty and established recipients, partial/full withdrawals,
pre-delegation balances, same-block sequences, rounding, eligibility, replay, rollback and paging.
The focused suite covers 99 cases and 3,000 generated actions; all audited executable lines run.

Fix the failures identified: propagate vault decoding failures instead of silently dropping capital
events, normalize transaction-hash IDs, avoid valuation RPC for completed transfer replays, exclude
non-capital transfers before timestamp reads, and reject duplicate/invalid RPC log indexes. Other
handlers keep the shared wrapper's default behavior. Production state and historical data are untouched.

Files: `src/services/yieldVault.service.ts`, `src/services/__tests__/yieldVault.service.test.ts`,
`src/services/__tests__/yieldVault.stress.test.ts`, `src/utils/vaultAccounting.ts`,
`src/utils/__tests__/vaultAccounting.test.ts`, `src/utils/event.utils.ts`,
`src/handlers/events/yieldVault/deposit.event.handler.ts`,
`src/handlers/events/yieldVault/withdraw.event.handler.ts`,
`src/handlers/events/yieldVault/transfer.event.handler.ts`,
`docs/ai/flows/vault-accounting-stress-audit.md`,
`docs/ai/decisions/2026-09-10-vault-transfer-principal.md`.
