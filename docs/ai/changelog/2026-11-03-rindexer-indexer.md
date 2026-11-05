# 2026-11-03 — omniplex: rindexer handlers for every SubQuery handler

Omniplex (`omniplex/`) is a Rust indexer built on rindexer v0.43.4. It replaces the SubQuery
handlers in `sdk/packages/indexer` (polytope-labs/hyperbridge#1439). It writes the same Postgres
`app` tables, and GraphQL stays on omnihedron, so the SDK and simplex queries are unchanged.
Substrate chains are read through `modules/utils/substrate-eth-rpc`.

## Running it

```
omniplex --config omniplex/config/mainnet.toml --chains bsc-mainnet
```

Omniplex is a member of the root Cargo workspace, so build it from the repo root with
`cargo build -p omniplex`. The workspace uses alloy 2.0, which rindexer requires.
The workspace builds with Rust 1.99.1. The relayer's fee database uses prisma-client-rust, which
comes from our fork, polytope-labs/prisma-client-rust: the archived upstream doesn't compile on
Rust 1.98 and later.

- Run one process per chain, all pointed at one database. rindexer finishes every chain's
  history before any chain goes live, so in one process, a slow or stuck chain would hold back
  the rest. `--chains` (or `INDEXER_CHAINS`) takes a comma-separated list, and indexes every
  chain in the config when unset. The solver watchlist poller runs in the process that indexes
  Hyperbridge.
- `DATABASE_URL` is the Postgres database. `DATABASE_POOL_SIZE` defaults to twice the number of
  chains the process indexes plus 4, and at least 10. At startup, processes take turns to apply
  the migrations and create rindexer's tables, so they can all start at once on a fresh
  database. `indexer.*` holds the cursors, and `app.*` holds the served tables.
- Each chain's RPC URL comes from an env var named after the chain: `bsc-mainnet` reads
  `BSC_MAINNET`. For a Substrate chain, that is its substrate-eth-rpc URL.
- A Substrate chain also needs `<CHAIN>_SUBSTRATE`, the node's own RPC. For the facade inside
  the Hyperbridge node, it is the same URL. A standalone facade serves only `eth_*`, so there is
  no default.
- Each of these variables can list several comma-separated URLs, as SubQuery's did. A request
  goes to the RPC that answered last, and moves on to the next one when that one doesn't
  answer, answers with an HTTP error, or answers that it can't serve the request (a rate
  limit, or a block it doesn't have yet). There is no failing back: the RPC that answers
  stays first until it fails.
- At startup, rindexer's client for each `<CHAIN>` URL checks the node's chain id. A URL for
  another chain stops the start. With several URLs, a node that fails the check or doesn't
  answer within 30 s is left out of rindexer's own calls until the next start, though handler
  reads still use it; the start fails only if none answers. With one URL, the start waits.
- `HYPERFX_ORDERBOOK_URL` prices tokens and serves the solver watchlist.
- Every RPC must serve historic state at the event's block. On Ethereum and Arbitrum, that means
  an archive endpoint. It must also serve every receipt and transaction of the blocks it
  indexes: a null answer is retried until the RPC has caught up.
- Index Substrate chains from our own archive node. The public Nexus and Gargantua endpoints
  rate-limit hard: through them, the facade indexes about 100 blocks a minute.
- `config/{mainnet,testnet}.toml` lists the chains. Soneium, Bifrost, Cere, Argon, Hydration and
  zkVerify are left out. Each chain sets its reorg-safe distance: Ethereum 3, Arbitrum and Base
  0, BSC and Polygon 4, Polkadot Hub 24, and Substrate chains 0, since the facade serves only
  finalized blocks.

## Starting from SubQuery's data

`omniplex --config <config> import-subquery` turns a copy of SubQuery's database, at
`DATABASE_URL`, into omniplex's. Each chain then continues after the last block SubQuery
indexed. A Substrate chain needs `<CHAIN>_SUBSTRATE` too. `omniplex/README.md` has the cutover
steps, including the parity check against SubQuery.

The import runs in one transaction, so a failure leaves the copy as it was.

- Each table keeps each row's current version. `solver_inventories` keeps every version.
- SubQuery's own `_global` and `_metadata_0x…` tables are dropped.
- Each chain starts at the `lastProcessedHeight` in SubQuery's metadata, which names a chain by
  its EVM chain id or its node's `system_chain`. The chain's cursor skips that block's logs.
- The import reads the heights and the rows in one snapshot, so they agree.
- The import refuses a database other clients are connected to, so it refuses SubQuery's live
  database while SubQuery runs. It also refuses a copy taken mid-rewind, a table or column only
  one side has, and a chain SubQuery has no height for. And it refuses when an object outside
  SubQuery's schema, such as a view, depends on it, since dropping the schema would drop that
  too.
- A chain SubQuery never indexed, such as the Polkadex testnet, goes in `--new-chains`. It starts
  at its start block.

## How it indexes

- **One stream per chain.** Each chain is one rindexer stream over every address its handlers
  watch. `AllTopicsProvider` widens that stream's `eth_getLogs` to every subscribed topic, so
  handlers see a chain's logs in (block, log index) order, as SubQuery delivered them.
  rindexer's per-event streams would not keep that order.
- **Batches.** Each block of a batch commits in one transaction together with the chain's
  cursor, as SubQuery flushed each block. So rows that several chains' processes write, such
  as a relayer's, stay locked for one block's work. A handler error fails the batch, and
  rindexer retries it. The replay skips the blocks that already committed.
- **RPC errors.** Handler reads time out after 30 s, and rate limits are retried in place.
  `eth_getLogs` goes through rindexer's own client. rindexer's live loop stops for good after
  an error on a range as short as a live window, so `AllTopicsProvider` retries those errors
  itself. A short range an RPC says spans too many blocks is fetched in halves instead.
- **Tip block.** When a window ends at the tip block, an answer with no log there is re-asked
  twice, after 250 ms and then 500 ms, if the block's logs bloom says it can have our logs.
  rindexer's own check, which the stream's placeholder topic rules out, re-asks four times over
  about 2 s. A busy chain's bloom matches nearly anything, so Base would pay that on nearly every
  block.
- **Block hooks.** SubQuery's block handlers are `BlockHook`s. They run after each batch at the
  last block it applied. On a chain with no logs, they run every 15 s at the block rindexer has
  synced.
- **Metadata.** Each chain has its own `app._metadata_0x…` table, which omnihedron serves as one
  `_metadatas` node, as SubQuery's multichain tables were. The explorer finds a chain by `chain`
  and reads `lastProcessedHeight`. The SDK's CI waits until every chain is close to its
  `targetHeight`.
  - `chain` is the EVM chain id, or the Substrate node's `system_chain`, such as
    "Hyperbridge (Nexus)". The table is named after it, so a chain has one node.
  - A Substrate chain's table appears once its node answers `system_chain`. Until then, the
    chain indexes as usual, but has no metadata.
  - Every 15 s, `lastProcessedHeight` is set to the block rindexer has synced, so it moves on
    chains with no logs too.
  - `targetHeight` is the head less the reorg-safe distance, and never below
    `lastProcessedHeight`.
- **Pending statuses.** A status whose parent another chain hasn't indexed yet waits in
  `pending_status_metadata`. Each process's flusher moves it once the parent exists.
- **Schema.** `migrations/0001_app_schema.sql` is the DDL SubQuery generates for `schema.graphql`,
  with relations declared as `@foreignKey` table comments. Production SubQuery keeps history, so
  it never had real foreign keys; with them, an event that arrived before its parent would stall
  its chain. `_block_range` is kept only on `solver_inventories`, which simplex reads back in
  time. omnihedron serves the same GraphQL schema as SubQuery's database, except that the
  internal `_global` table is gone and only `solverInventories` takes `blockHeight`.
- **Parity.** USD math emulates decimal.js: 20 significant digits, rounded half up.

## Parity with SubQuery

Handlers reproduce SubQuery's output, so the two indexers' tables can be compared. Bugs that
lose data are fixed instead. The deliberate differences:

- Reads SubQuery made at the chain head are made at the event's block: state commitments,
  request fees, treasury balance, fee decimals, bandwidth allowance and token supply.
  `ismp_queryRequests` has no block parameter and still reads the head. So
  `treasuries.total_balance` and `bridge_token_supplies.circulating_supply` differ from
  SubQuery's by the few blocks SubQuery trailed the head.
- RPC and database errors SubQuery swallowed now fail the batch and are retried. Reverts, other
  failed executions and malformed data are still logged and skipped. A token's `symbol()` and
  `decimals()` run code anyone can deploy, so any error answer from them is final, unless the
  node couldn't serve the call. Those calls get 500k gas, so a timeout is the node's. Four
  reads stay best-effort and only log a failure:
  - the `debug_traceTransaction` fallback for an order's calldata, which many RPCs don't serve;
  - HyperFX orderbook prices;
  - yield vault reads, which a later block retries until the day's snapshot completes;
  - the bridge token supply, which the next 3600-block run reads again.
- The periodic pending flush only picks rows whose parent exists, so a row whose parent never
  arrives no longer blocks the rest.
- Chain metadata holds only `chain`, `lastProcessedHeight` and `targetHeight`. SubQuery also wrote
  keys such as `genesisHash`, `startHeight` and `indexerHealthy`, which nothing we run reads.
- A token that doesn't answer `symbol()` and `decimals()` is worth $0 in order and fill values.
  SubQuery failed the block, and under rindexer's retries that would stop the chain.
- Dust isn't valued. The dust rows keep each token's symbol and raw amount, and the per-chain
  dust USD totals stay empty. SubQuery priced dust with Uniswap quotes, which run in pools
  anyone can create. hyperfx-app, the one reader of dust, prices the raw amounts itself.
- Some event data SubQuery failed the block on is stored instead, since a retry would fail the
  same way:
  - Strings lose any NUL, which Postgres text can't hold, and token symbols are cut to 256
    bytes, so their indexes can hold them.
  - A Hyperbridge `PostRequestHandled` whose relayer doesn't decode keeps its status, without
    relayer stats.
  - Delivery calldata unwraps `batchCall`s 8 deep.
- An order destination that starts with `0x` but isn't hex is hashed as text, the bytes the
  gateway hashed. viem hashed its own hex encoding of the order instead, so SubQuery's
  commitment and row id for such an order matched no order on chain.
- A Hyperbridge request that has since timed out is read from the ISMP events of the block it
  was dispatched in. `ismp_queryRequests` reads the head, where that request is gone.
- A Hyperbridge delivery or timeout whose request no chain has indexed yet waits in
  `pending_status_metadata`, and the delivery counts toward the relayer's stats. SubQuery
  dropped both. A waiting `HYPERBRIDGE_DELIVERED` becomes `DESTINATION` if the request turns
  out to be for Hyperbridge.
- EVM state commitment timestamps come from the host's `stateMachineCommitment` view at the
  event's block, in seconds. SubQuery read a storage slot from an older host layout, so every
  EVM-side row it wrote holds 0. The SDK's timeout flow for a request that never reached
  Hyperbridge waits for one of these rows, so against SubQuery it never finishes.
- `AccumulateFees` keeps relayers named by an EVM address, in lowercase hex as `Withdraw` names
  them. SubQuery dropped their fees.
- When one extrinsic rewards a relayer several times, the last reward keeps SubQuery's
  `relayer_reward_transactions` id, and the earlier ones move to `<id>-1`, `<id>-2` and so on.
  SubQuery overwrote them, so its row holds the last reward.
- A fill indexed before its order, because the destination ran ahead, adds to the order's
  `filled` and pays its points when the order arrives. SubQuery's `--historical=timestamp`
  reads hid fills from blocks later than the placement's, which is nearly every fill, so its
  backfill never ran. The points use the destination chain's valuation, kept in
  `indexer.deferred_fill_points` until the order arrives. A fill SubQuery indexed before the
  import, for an order that arrives after it, gets its `filled` but no points, as in SubQuery:
  SubQuery kept no valuation for it.
- A chain still in its history doesn't consume a solver watchlist version until its blocks
  reach the time of the poll that wrote it, as SubQuery's time-bounded reads did.

Bugs kept for parity, to decide on separately:

- A batched delivery counts the whole transaction's gas once per event. Every transfer out of a
  host counts as relayer fees.
- Bandwidth eviction looks at the first 100 rows only. Active counts include expired
  subscriptions.
- The intent gateway: a zero referrer counts as a referrer, early fills leave the order `PLACED`,
  and statuses can move backwards.
- Comments in the handlers mark where a bug is kept for parity.

## Not included yet

- **Parity check.** It is done by hand at cutover, as `omniplex/README.md` describes. So far,
  short ranges on Nexus and Gargantua match SubQuery for the ISMP, filler bid, reward, treasury
  and bandwidth tables, apart from the two columns above. On EVM, only Base has been compared, on
  one order.
- **Deployment and retirement:** a Docker image, compose and CI, and retiring SubQuery.
