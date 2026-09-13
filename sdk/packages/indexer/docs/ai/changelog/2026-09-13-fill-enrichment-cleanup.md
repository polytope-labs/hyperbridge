# 2026-09-13 — Fill enrichment cleanup (#1112)

Replaced sorted operation-boundary searches with single-pass selection and shared the sender check between fill and placement matching. Grouped delivery transfers once per token instead of rescanning all transfers for each output. The calldata decoder now uses generated ABI types and one token conversion helper; the unused forwarding export was removed.

The migration test now uses explicit row fixtures and expected field values instead of inferring types from field names through nested ternaries. All 56 focused tests, the release build, and the real SubQuery/Postgres migration check pass. A temporary differential check against commit `1f637786b` compared transfer amounts and both operation-hash matchers across 10,000 deterministic receipt scenarios, including shuffled log order, with identical results and no receipt mutation.

Files: `src/utils/userOp.helpers.ts`, `src/utils/fill.helpers.ts`, `src/utils/__tests__/fill.helpers.test.ts`, `scripts/tests/verify-fill-schema-migration.cjs`.
