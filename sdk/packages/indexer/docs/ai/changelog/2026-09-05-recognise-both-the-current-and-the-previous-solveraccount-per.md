# 2026-09-05 — Recognise both the current and the previous SolverAccount per chain

`solverAccount` in `config-{mainnet,testnet}.json` accepts one address or a list; the generated
`SOLVER_ACCOUNT_ADDRESSES` is `Record<string, string[]>`. `YieldVaultService.isDelegatedSolver`
and the phantom price handler count an account when it delegates to any listed address, the
latter by passing the list to the sdk's `aggregatePhantomBids`, which now accepts one or several.
Mainnet lists the 2026-09-05 SolverAccount `0x7cb55539d1144F62422099c3FA3405092022c88C` first and
the replaced `0xfCd233b937D7622AAc63ced3C9A1A12F4a6B64E3` second on the six chains that had an
entry, so solvers keep counting while they re-delegate. Drop the old address once they have.

Files: `src/configs/index.ts`, `src/configs/config-mainnet.json`, `scripts/generate-chain-yamls.ts`,
`src/solver-account-addresses.ts`, `src/services/yieldVault.service.ts`,
`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`,
`src/handlers/events/substrateChains/__tests__/phantomOrder.handlers.test.ts`, `docs/ai/ChangeLog.md`,
`docs/ai/Flow.md`.
