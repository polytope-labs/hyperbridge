# 2026-11-03 — indexer: rindexer handlers for every SubQuery handler

`indexer/` is a Rust indexer built on rindexer v0.43.4. It replaces the SubQuery handlers in
`sdk/packages/indexer` (polytope-labs/hyperbridge#1439). It writes the same Postgres `app`
tables, and GraphQL stays on omnihedron, so the SDK and simplex queries are unchanged.
Substrate chains are read through `modules/utils/substrate-eth-rpc`.

## Running it

```
hyperbridge-indexer --config indexer/config/mainnet.toml --chains bsc-mainnet
```

- Run one process per chain, all pointed at one database. rindexer finishes every chain's
  history before any chain goes live, so in one process, a slow or stuck chain would hold back
  the rest. `--chains` (or `INDEXER_CHAINS`) takes a comma-separated list, and indexes every
  chain in the config when unset. The solver watchlist poller runs in the process that indexes
  Hyperbridge.
- `DATABASE_URL` is the Postgres database. `DATABASE_POOL_SIZE` defaults to twice the number of
  chains the process indexes, plus 4. Migrations run at startup: `indexer.*` holds the
  cursors, and `app.*` holds the served tables.
- Each chain's RPC URL comes from an env var named after the chain: `bsc-mainnet` reads
  `BSC_MAINNET`. For a Substrate chain, that is its substrate-eth-rpc URL. `<CHAIN>_SUBSTRATE`
  is the node's own RPC, and defaults to the same URL, which is right for the in-node facade.
- `HYPERFX_ORDERBOOK_URL` prices tokens and serves the solver watchlist.
- Every RPC must serve historic state at the event's block. On Ethereum and Arbitrum, that means
  an archive endpoint.
- `config/{mainnet,testnet}.toml` lists the chains. Soneium is left out. Each chain sets its
  reorg-safe distance: Ethereum 3, Arbitrum and Base 0, BSC and Polygon 4, Polkadot Hub 24,
  and Substrate chains 0, since the facade serves only finalized blocks.

## How it indexes

- **One stream per chain.** Each chain is one rindexer stream over every address its handlers
  watch. `AllTopicsProvider` widens that stream's `eth_getLogs` to every subscribed topic, so
  handlers see a chain's logs in (block, log index) order, as SubQuery delivered them.
  rindexer's per-event streams would not keep that order.
- **Batches.** A batch runs in one transaction together with the chain's cursor. A batch that
  rindexer replays after a crash skips logs that already committed. A handler error fails the
  batch, and rindexer retries it.
- **Block hooks.** SubQuery's block handlers are `BlockHook`s. They run after each batch at the
  last block it applied. On a chain with no logs, they run every 15 s at the block rindexer has
  synced.
- **Pending statuses.** A status whose parent another chain hasn't indexed yet waits in
  `pending_status_metadata`. One flusher moves it once the parent exists.
- **Schema.** `migrations/0001_app_schema.sql` is the DDL SubQuery generates for `schema.graphql`,
  with relations declared as `@foreignKey` table comments. Production SubQuery keeps history, so
  it never had real foreign keys; with them, an event that arrived before its parent would stall
  its chain. `_block_range` is kept only on `solver_inventories`, which simplex reads back in
  time. omnihedron serves the same GraphQL schema as SubQuery's database, except that the
  internal `_global` table is gone and only `solverInventories` takes `blockHeight`.
- **Parity.** USD math emulates decimal.js: 20 significant digits, rounded half up.

## Parity with SubQuery

Handlers reproduce SubQuery's output, including its bugs, so the two indexers' tables can be
compared. The deliberate differences:

- Reads SubQuery made at the chain head are made at the event's block: state commitments,
  request fees, treasury balance, fee decimals, bandwidth allowance and token supply.
  `ismp_queryRequests` has no block parameter and still reads the head.
- RPC and database errors SubQuery swallowed now fail the batch and are retried. Reverts and
  malformed data are still logged and skipped.
- The periodic pending flush only picks rows whose parent exists, so a row whose parent never
  arrives no longer blocks the rest.
- A token that doesn't answer `symbol()` and `decimals()` is worth $0 in order and fill values.
  SubQuery failed the block, and under rindexer's retries that would stop the chain.
- Points for a fill indexed before its order use the destination chain's valuation, kept in
  `indexer.deferred_fill_points` until the order arrives. SubQuery re-read the destination's
  tokens on the source chain, which failed the block when a token had no code there.

Bugs kept for parity, to decide on separately:

- EVM state commitments read storage slot 5, not 3, and divide seconds by 1000. Every EVM
  `commitment_timestamp` is 0.
- Hyperbridge-side handled and timeout statuses are dropped when the request isn't indexed yet.
- A batched delivery counts the whole transaction's gas once per event. Every transfer out of a
  host counts as relayer fees.
- `AccumulateFees` drops 20-byte relayers. Several `RelayerRewarded` events in one extrinsic share
  one id.
- Bandwidth eviction looks at the first 100 rows only. Active counts include expired
  subscriptions.
- The intent gateway: a zero referrer counts as a referrer, early fills leave the order `PLACED`,
  and statuses can move backwards. Dust USD from Uniswap V2 and V4 ignores token decimals.
- Comments in the handlers mark where a bug is kept for parity.

## Not included yet

- **Live Substrate runs.** Nothing has run against a Hyperbridge node or facade. On EVM, Base was
  indexed live and one order matched SubQuery field for field. BSC and Polkadot Hub ran cleanly;
  Ethereum and Arbitrum need an archive RPC.
- **Side-by-side comparison** of every table against SubQuery over an agreed block range.
- **Deployment and retirement:** a Docker image, compose and CI, and retiring SubQuery.
