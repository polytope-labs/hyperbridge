# 2026-09-14 — Event-sourced solver inventory and delegation, with watchlist discovery (#1264)

Solver balances and delegation no longer depend on phantom-order indexing. Each EVM node now tracks its own
chain's solvers:
- **Discovery.** A solver is discovered by its first IntentGateway fill or partial fill, or by a request the
  Hyperbridge node queues from the HyperFX orderbook's `GET /solvers` watchlist. The watchlist is polled every
  block while live, one request for every chain, with an ETag and a timeout.
- **Genesis.** A newly discovered solver gets one storage read pinned to a block, in the EVM block handler: every
  supported token's balance, every supported vault's shares, and its code.
- **Events.** From then on, wallet balances follow the supported tokens' ERC-20 Transfers, and vault shares
  follow every vault share Transfer.
- **Refresh.** Vault shares are revalued hourly. A daily reconciliation re-reads everything, records drift and
  re-checks delegation.
- **Head.** A per-chain head, advanced every 30 s of block time, tells consumers how current the unchanged rows
  are.

New entities:
- `TrackedSolver`
- `SolverInventory`
- `SolverVaultShares`
- `SolverDelegation`
- `SolverInventoryHead`
- `SolverWatchlistCursor`
- `SolverDiscoveryRequest`, written by the Hyperbridge node
- `SolverWatchlist`, written by the Hyperbridge node

The EVM manifest gains one ERC-20 datasource per supported token and the block handler. The substrate manifest
gains the poll. `HYPERFX_WATCHLIST_URL` is a new tracked environment variable. `safeFetch` gains a timeout, 30 s
by default.

`SolverDelegation.delegated` is deliberately not indexed. In historical mode SubQuery rebuilds every
index as GIST over (fields…, `_block_range`), and `btree_gist` has no operator class for boolean, so
an index there makes the whole schema fail to create. Filtering on it still works, within what the
`(chain, solver)` index has already narrowed. Enums are unaffected — `anyenum` is covered — so
`TrackedSolver`'s `["chain", "status"]` is fine.

Files: `src/configs/schema.graphql`, `src/configs/abis/Erc20.abi.json` (new),
`src/services/solverInventory.service.ts` (new), `src/services/solverWatchlist.service.ts` (new),
`src/handlers/events/solverInventory/tokenTransfer.event.handler.ts` (new),
`src/handlers/events/solverInventory/inventory.block.handler.ts` (new),
`src/handlers/events/solverInventory/watchlistPoll.block.handler.ts` (new),
`src/handlers/events/intentGatewayV3/orderFilledV3.event.handler.ts`,
`src/handlers/events/intentGatewayV3/partialFilledV3.event.handler.ts`,
`src/handlers/events/yieldVault/transfer.event.handler.ts`, `src/mappings/mappingHandlers.ts`,
`src/utils/safeFetch.ts`, `scripts/generate-chain-yamls.ts`, `scripts/templates/evm-chain.yaml.hbs`,
`scripts/templates/substrate-chain.yaml.hbs`, `src/services/__tests__/solverInventory.service.test.ts` (new),
`src/services/__tests__/solverWatchlist.service.test.ts` (new), `src/utils/__tests__/safeFetch.test.ts` (new),
`docs/ai/decisions/2026-09-14-solver-inventory-is-event-sourced-per-chain-with-a-discovery-queue.md`,
`docs/ai/flows/solver-inventory-discovery-genesis-transfers.md`.
