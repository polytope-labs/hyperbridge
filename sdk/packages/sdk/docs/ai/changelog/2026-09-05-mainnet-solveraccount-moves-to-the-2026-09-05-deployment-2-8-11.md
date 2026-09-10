# 2026-09-05 — Mainnet SolverAccount moves to the 2026-09-05 deployment; 2.8.11

Every mainnet chain config points `SolverAccount` at `0x7cb55539d1144F62422099c3FA3405092022c88C`,
the account deployed alongside the new IntentGatewayV2 implementation on 2026-09-05 (PR #1207),
replacing `0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3`. Solvers delegating through the SDK pick
up the new account on upgrade. The bid-verification fixture in `phantomAggregation.test.ts` uses
the same address. `aggregatePhantomBids` accepts `solverAccount` as one address or a list and
counts a bid whose sender delegates to any of them, so an indexer can keep the replaced account
listed while solvers re-delegate; the delegation reader reads the code once and checks membership.
Version 2.8.11; simplex 0.13.1 goes with it.

Files: `src/configs/chain.ts`, `src/protocols/intents/phantom-aggregation.ts`,
`src/tests/phantomAggregation.test.ts`, `package.json`, `docs/ai/ChangeLog.md`.
