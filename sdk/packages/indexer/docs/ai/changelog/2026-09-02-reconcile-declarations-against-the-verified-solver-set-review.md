# 2026-09-02 — Reconcile declarations against the verified solver set (review fixes on #1194)

Review found the declaration reconciliation keyed off the wrong set. `recordDeclaredPositions` was passed the
solvers appearing in `lpBalances` or `positions`, and both are filtered: the sweep skips tokens a solver does not
hold, and `positions` only lists what was declared. A solver that bid while holding nothing anywhere and
declaring nothing was in neither, so its previous declaration was never emptied and the refresh kept valuing
positions it had stopped offering — and a V4-funded solver with a near-empty wallet is exactly that profile.

`aggregatePhantomBids` now returns `solvers`, the verified bidders it already tracked internally to stop one
solver's bid counting once per funded filler, and the handler reconciles against that.

Also drops `IntentGatewayV3Service.refreshLiquidityAfterVaultEvent`, which was never called: `recordLedger`
reaches `refreshProviderLiquidity` through `liquidityRefreshContext` directly, which is the right seam — a vault
event has nothing to do with the intent gateway.

Files: `src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`,
`src/services/solverPositions.service.ts`, `src/services/intentGatewayV3.service.ts`,
`src/handlers/events/substrateChains/__tests__/phantomOrder.handlers.test.ts`.
