# 2026-09-01 — Refresh a pool's LP balances on every order fill

A pool's published depth is the sum of its bidders' output-token inventory, measured when the last phantom bid
window closed. A fill spends some of that inventory, so between windows the depth advertises capacity the
solvers no longer hold — and the depth is what a taker sizes an order against.

`OrderFilled` now re-reads the balances of every LP recorded as backing the pools the fill traded through and
republishes the depths from them: `PoolBidder.liquidity`, `PoolChainLiquidity.depth`/`bidCount`/unrestricted
slice, `PoolRoute.depth`/`bidCount`, and the pool's own `sellDepth`/`buyDepth`/bid counts. A bidder left holding
nothing loses its row, so the "every row is a bidder with capacity" invariant survives, and the routes it alone
declared go with it. Rates are untouched — nothing here observes a quote — though a pool's merged rate can still
move, because the chains are weighted by the depths that just changed.

`PartialFill` refreshes on the same terms, through the same service method: it spends inventory identically, and
being the same-chain path (`IntrinsicIntents`) its source and destination are one chain. Cross-chain fills are
all-or-nothing, so between the two handlers every fill that moves inventory now triggers a refresh.

The pools are resolved from the order's own two sides: the input symbols on its source chain paired with the
output symbols on the destination chain, via the same token registry the snapshot path uses. An order in
untracked assets resolves to no pool and costs nothing, which is most of them.

Two guards keep the cost bounded and the data honest. A pool whose last snapshot postdates the fill is skipped
entirely — that snapshot already read balances this fill had moved, and during a resync it is every pool, so
replaying history triggers no RPC at all. And a chain whose balances cannot be read in full is left exactly as
indexed rather than partially republished: a failed read is indistinguishable from a zero balance, so half a
chain would report the unread bidders as departed.

The refresh also extends `LiquidityProviderBalanceV2` with what it read, so a provider's latest balance row does
not keep reporting inventory the pool rows already know is spent. That series is keyed by Hyperbridge block and a
fill has none, so it borrows Hyperbridge's head via `chain_getHeader` on the configured node — one read per
indexed block, shared by every fill in it. Two readings on one key resolve to the larger, the same rule the sweep
follows, because a refresh can never see a solver's Uniswap V4 positions and the smaller reading is the
incomplete one. An unreachable Hyperbridge node costs the data point, not the refresh.

`lastUpdatedBlock`/`lastUpdatedAt` are deliberately not moved anywhere. They date the snapshot that priced the
pool, in Hyperbridge blocks; a fill carries an EVM block number of another chain, which is not comparable with
them and would wreck the `MAX_SAMPLE_AGE_BLOCKS` staleness filter.

The balance-reader wiring (`setAggregationFetch(safeFetch)`, the per-chain RPC map, the per-block memo) moved out
of `handlePhantomOrderPrices.handler.ts` into `src/utils/solverBalance.ts` so both paths read balances the same
way; the memo key is now a string so it can identify a block of either chain. The pool-level merge and the
unrestricted-slice split were extracted into `mergeChainRowsIntoPool` and `bidderDepths`, shared by the snapshot
writer and the refresh — the two must publish the same numbers from the same bidders.

Unrelated but in the way: `phantomOrder.handlers.test.ts` was red before this change — all 14 of its
`handlePhantomOrderPrices` cases threw `sortPoolSymbols is not a function`. Its `@hyperbridge/sdk/intents-helpers`
mock lists only the three entry points the handlers call, and the pool-token registry re-exports its symbol
ordering from that same module, so mocking it dropped `poolSlug`/`sortPoolSymbols` and every pool attribution
threw. The mock now spreads `jest.requireActual` and overrides only those three; importing the SDK under jest has
worked since `transformIgnorePatterns` was added. That suite covers the handler wiring this change moved, so it
had to run to verify the move — all 16 cases pass.

Files: `src/services/liquidityPool.service.ts`, `src/services/intentGatewayV3.service.ts`,
`src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`,
`src/handlers/events/intentGatewayV3/partialFilledV3.event.handler.ts`,
`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`, `src/utils/solverBalance.ts` (new),
`src/configs/schema.graphql` (descriptions only),
`src/services/__tests__/liquidityPoolRefresh.service.test.ts` (new),
`src/handlers/events/substrateChains/__tests__/phantomOrder.handlers.test.ts`.
