# 2026-09-01 — Refresh liquidity on escrow releases and vault events, and value declared V4 positions (#1159)

Follow-up to #1192, which refreshed a pool's LPs on `OrderFilled`/`PartialFill`. Four things were still missing.

**Uniswap V4 positions are recorded and re-read.** A bid's `paymasterAndData` is the only place a position is
ever named, so a balance re-read could not see that inventory at all — and carrying the last sweep's VALUE
forward would be worse, because simplex funds fills out of these positions: such a fill drains the position
inside the fill transaction while wallet and vault balances barely move. The aggregation now reports the
tokenIds it verified, and `handlePhantomOrderPrices` records them in a new `SolverV4Positions` entity: one row
per solver, keyed by its address, replaced wholesale each time that solver bids. The refresh reads that one row
and re-values each position at the event's block. A solver that bids without declaring has its row emptied; one
that skips a window keeps its row, and a position it has since burned or sold reads back as no longer owned,
which is the check every consumer applies before valuing one.

**Reads are pinned to the event's block.** `getTotalSolverBalance` is exported from the SDK with a block tag and
`memoizedSolverBalance` takes a per-chain map of them, so an event's re-read returns the same value on a replay
as it did live. Only the event's own chain is pinned: a refresh reaches across every chain the pool is quoted on
and a block number means nothing on any other.

**Escrow releases refresh too.** `EscrowReleased` on the source chain pays the filler the order's inputs back, so
its inventory there rose and every pool it backs in those tokens is understating depth. The event names no
filler, so the handler reads the gateway's `_filled(commitment)` mapping at that block — `_withdraw` writes the
beneficiary in the same call that emits the event, so it never depends on the destination chain's node having
indexed the fill first.

**Vault deposits and withdrawals refresh too.** `YieldVaultService.recordLedger` ends with a refresh for
(chain, lp, underlying token). An LP moving its own principal only shifts inventory between the raw and vault
halves of one total, which the re-read confirms rather than changes, but the total does move when the
counterparty is someone else — a treasury funding the solver, or inventory leaving it — and no order event
reports that at all.

Those last two name a solver and a token, never a pool, so they reach the refresh through a new
`refreshProviderLiquidity` entry point. It shares everything with the pool-scoped one, which meant one change to
the shared core: depths are now re-summed from the stored bidder rows of each (pool, chain) rather than from the
rows that were re-read, so refreshing one solver leaves the others contributing exactly what they were.

The only schema change is the new `SolverV4Positions` entity — additive, and with no `@derivedFrom` field added
to `LiquidityProvider`, so nothing about an existing entity changes. Deliberately not done, per #1159 §3, for
that same reason: the `trigger` enum, the nullable `transactionHash`, the
`{chain}-{token}-{solver}-{blockNumber}` id shape for event-triggered rows, and ordering "current liquidity" by
`snapshotTime` instead of `blockNumber` all alter a live entity. Event rows keep borrowing Hyperbridge's head
block, and the design is left as a comment on `recordProviderBalances` for whenever a migration is on the table.

Files: `src/configs/schema.graphql`, `src/services/liquidityPool.service.ts`,
`src/services/solverPositions.service.ts` (new), `src/services/intentGatewayV3.service.ts`,
`src/services/yieldVault.service.ts`, `src/utils/solverBalance.ts`,
`src/handlers/events/intentGatewayV3/escrowReleasedV3.event.handler.ts`,
`src/handlers/events/substrateChains/handlePhantomOrderPrices.handler.ts`,
`src/services/__tests__/liquidityPoolRefresh.service.test.ts`,
`src/handlers/events/substrateChains/__tests__/phantomOrder.handlers.test.ts`.
