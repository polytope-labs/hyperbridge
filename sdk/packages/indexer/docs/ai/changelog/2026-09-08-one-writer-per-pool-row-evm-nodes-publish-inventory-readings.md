# 2026-09-08 — One writer per pool row: EVM nodes publish inventory readings, Hyperbridge folds them (#1214)

The pool family (`LiquidityPool`, `PoolChainLiquidity`, `PoolBidder`, `PoolRoute`, `LiquidityProviderBalanceV2`) is
now written by the Hyperbridge node only. An EVM event that moves a solver's inventory (order fill, partial fill,
escrow release, vault deposit or withdrawal) re-reads the solver's balance on its own chain, pinned to the event's
block, and writes a `SolverInventoryReading` row that only that chain's node writes. A new block handler on the
Hyperbridge node, `handleInventoryFold`, folds those readings into the pool rows every block; a reading applies to
a bidder row only if it postdates the row's snapshot time and the new nullable `PoolBidder.refreshedAt`, so the
fold is idempotent. This removes the concurrent read-modify-write of the same pool rows by several node
processes, each serving `get` from a process-local cache that no other node's write invalidates. The
`SolverV4Positions` read on the EVM side goes through a field query for the same reason. The EVM-side read of
Hyperbridge's head block is gone: the fold stamps balance-series rows with its own block.
Files: src/configs/schema.graphql, src/services/inventoryReading.service.ts (new), src/services/liquidityPool.service.ts,
src/services/solverPositions.service.ts, src/services/intentGatewayV3.service.ts, src/services/yieldVault.service.ts,
src/utils/solverBalance.ts, src/handlers/events/liquidity/inventoryFold.block.handler.ts (new),
src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts, partialFilledV3.event.handler.ts,
escrowReleasedV3.event.handler.ts, src/mappings/mappingHandlers.ts, scripts/templates/substrate-chain.yaml.hbs,
src/services/__tests__/inventoryReading.service.test.ts (new), src/services/__tests__/liquidityPoolFold.service.test.ts (new),
src/services/__tests__/liquidityPoolRefresh.service.test.ts (removed), docs/ai/ChangeLog.md, docs/ai/Decisions.md, docs/ai/Flow.md.
