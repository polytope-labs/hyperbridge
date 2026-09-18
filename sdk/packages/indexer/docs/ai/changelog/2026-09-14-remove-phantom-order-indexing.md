# 2026-09-14 — Remove phantom-order indexing

The HyperFX orderbook replaces phantom orders as the source of rates and depth, so the indexer no longer indexes
them. Removed:
- The `PhantomOrderRegistered` and `PhantomBidWindowExhausted` handlers, and bid-calldata decoding
  (`phantom-decode.ts`).
- The pool family they produced: `LiquidityPool`, `PoolChainLiquidity`, `PoolBidder`, `PoolRoute`,
  `LiquidityProviderBalanceV2` and `LiquidityProvider`.
- The EVM-side inventory readings and the Hyperbridge fold that kept pool depth fresh: `SolverInventoryReading`,
  `inventoryReading.service.ts`, `solverBalance.ts` and `handleInventoryFold`.
- The declared Uniswap V4 positions (`SolverV4Positions`) and their addresses.
- The pool-token registry and its codegen script.
- The phantom entities `PhantomOrderV2`, `PhantomOrderLeg` and `PhantomOrderPriceSnapshotV2`.

Fills, partial fills, escrow releases and vault ledger events no longer publish inventory readings.

Two consumers of that data change:
- **FX volume pricing.** Intent-gateway USD volume priced FX tokens from `LiquidityPool` rates. It now asks
  `fetchOrderbookUsdPrice`, which is mocked to return no price until the orderbook query is wired in. Until then
  those tokens' USD rollup is skipped and their raw amounts are kept.
- **Yield ledger gate.** It treated a `LiquidityProvider` row as proof that an LP is one of our solvers. It now
  relies only on an existing position or on-chain delegation.

`FillerBid` and `handleBidPlaced` stay, since they record coprocessor bids for any order. The substrate manifest's
`enableLiquidityIndexing` flag is now `enableSolverDiscovery`, and gates only the watchlist poll.

Removing entities is a destructive migration. The substrate schema leader needs
`SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true` for the restart that applies it, and the dropped tables' data is lost.

Merging `main` removed the rest of the same machinery it had grown meanwhile:
`IntentGatewayV3Service.publishInventoryAfterFill`, `publishInventoryAfterEscrowRelease`, and
`filledBeneficiary`, whose only caller was the escrow-release publication. Both escrow-release
handlers still record the release; `recordEscrowRelease` decides REDEEMED versus a non-finalizing
partial redeem on its own.

Files: `src/configs/schema.graphql`, `src/mappings/mappingHandlers.ts`, `scripts/generate-chain-yamls.ts`,
`scripts/templates/substrate-chain.yaml.hbs`, `package.json`, `src/services/intentGatewayV3.service.ts`,
`src/services/orderbookRates.service.ts` (new), `src/services/yieldVault.service.ts`,
`src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`,
`src/handlers/events/intentGatewayV3/partialFilledV3.event.handler.ts`,
`src/handlers/events/intentGatewayV3/escrowReleasedV3.event.handler.ts`,
`src/services/__tests__/yieldVault.service.test.ts`, `src/services/__tests__/yieldVault.stress.test.ts`;
deleted `src/handlers/events/substrateChains/handlePhantomOrderRegistered.handler.ts`,
`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`,
`src/handlers/events/substrateChains/__tests__/phantomOrder.handlers.test.ts`,
`src/handlers/events/liquidity/inventoryFold.block.handler.ts`, `src/services/liquidityPool.service.ts`,
`src/services/inventoryReading.service.ts`, `src/services/solverPositions.service.ts`, `src/utils/solverBalance.ts`,
`src/utils/phantom-decode.ts`, `src/addresses/uniswap-v4.addresses.ts`, `src/addresses/pool-tokens.addresses.ts`,
`src/addresses/pool-tokens.generated.ts`, `scripts/generate-pool-tokens.ts`, their tests, and the flows
`phantom-price-snapshot-to-pool-rates-phantombidwindowexhausted.md`,
`phantom-bid-calldata-decoding-extractfilldatavm2.md` and
`pool-liquidity-refresh-orderfilled-partialfill-escrowreleased.md`;
`docs/ai/flows/intent-gateway-volume-indexing-orderfilled.md`,
`docs/ai/decisions/2026-09-14-phantom-order-indexing-is-removed-and-fx-volume-prices-come-from-the-orderbook.md`.
