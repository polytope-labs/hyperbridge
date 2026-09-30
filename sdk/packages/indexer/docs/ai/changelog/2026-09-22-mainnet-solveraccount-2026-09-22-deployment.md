# 2026-09-22 — Mainnet recognises only the 2026-09-22 SolverAccount

Every mainnet chain in `config-mainnet.json`, and Base in `config-solver-ci.json`, lists a single
`solverAccount`: `0xd5535d4DeB17F050e52B6efda2fDe00435f39279`, deployed with the new IntentGatewayV2
implementation (PR #1317). The previous accounts `0x7cb55539d1144F62422099c3FA3405092022c88C` and
`0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3` are removed, so `SOLVER_ACCOUNT_ADDRESSES` no longer
contains them. A solver EOA still delegated to either is recorded as delegated to an unknown
contract: it is not tracked for solver inventory, and its yield-vault positions are not attributed
to a solver, until it re-delegates to the new account.

The E2E seed fixture `scripts/tests/solver-fixtures.cjs` delegates to the new address.
