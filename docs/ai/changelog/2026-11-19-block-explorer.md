# Block explorer on omniplex

Hyperbridge's block explorer keeps the Statescan site and replaces Statescan's MongoDB scanners
with omniplex, so one process reads each chain's history instead of five scanners.

## substrate-eth-rpc

The `explorer` key adds three logs to every block: `Block` once, `Extrinsic` per extrinsic and
`Event` per event, with calls and events decoded against the block's own metadata and their
arguments rendered as JSON. They come from the same decode and block cache as the pallet logs,
take log indices after the block's events, and never fail a block when a call doesn't decode.
The facade README lists the signatures.

## Indexing

A config with `explorer = true` runs only the explorer's handlers, with no SubQuery metadata and
no background jobs, so its chain keeps its own cursor beside the ismp indexer of the same chain.
Its chains are named `gargantua-explorer` and `nexus-explorer`, which read `GARGANTUA_EXPLORER`
and `NEXUS_EXPLORER` (and the `_SUBSTRATE` variables) for their RPCs, and start at block 1.

The handlers write the `explorer` schema (migration `0003_explorer`):

- `blocks`, `extrinsics`, `events` and `transfers`, with times in milliseconds and addresses
  in SS58 with prefix 0. Transfers come from the typed `balances` `Transfer` log.
- `accounts`, refreshed after each batch from `System.Account` for every account a transfer
  or a signed extrinsic touched. An account the chain reaped is removed.
- `call_names`, every section and method seen under each runtime, for the site's filters.
- `status`, the total issuance read after each batch.

Arguments larger than 8 KiB are stored as `NULL`. On Hyperbridge those are relay chain proofs,
consensus proofs and runtime code, which no page shows and which would otherwise be most of
the database.

## API

`omniplex explorer-api` (`EXPLORER_API_LISTEN`, default `0.0.0.0:7011`) serves the REST routes
the site calls for Hyperbridge, in the shapes Statescan's server gave them, and its block,
extrinsic, event and account GraphQL queries at `/graphql`. It reads only Postgres: Statescan
answered the GraphQL queries from the chain on every page view.

Differences from Statescan:

- Lists leave out the `timestamp.set` and `parachainSystem.setValidationData` inherents and
  each extrinsic's `ExtrinsicSuccess` or `ExtrinsicFailed` event, as Statescan did, and every
  block is kept, where Statescan's simple mode kept only the last hundred.
- Block authors, inner calls, nonces, tips, signatures and event docs are not indexed and
  read as null.
- The filters list the calls and events that occurred up to each runtime, not every one its
  metadata declares.
