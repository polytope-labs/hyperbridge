# @hyperbridge/indexer

A SubQuery-based indexer service for tracking and indexing cross-chain messages within the Hyperbridge ecosystem.

## Features

- Index post and get requests, transfers, relayer activities and state machine updates across different chains
- Track request statuses and timeouts
- Support for EVM and Substrate chains
- Real-time data processing

## Installation

This package is primarily for internal use, but you can install and run it locally:

```bash
pnpm install
pnpm build
```

## Running the Indexer

Start in development mode
```bash
pnpm local
```

## Schema migrations

Entities live in `src/configs/schema.graphql`. Additive changes to an existing entity are applied in place on restart,
so you keep the data you have already indexed:

1. Edit `schema.graphql` — add a field (it must be nullable), a new entity, an index, an enum value, or a relation.
2. `pnpm build` then restart the stack (`pnpm start:local`).

On boot the substrate node diffs the schema it last applied against the new one and runs the matching `ALTER`s. It is
the only node that needs the `--allow-schema-migration` flag (already set in the compose files): every EVM node waits on
its healthcheck, and the shared schema covers all entities, so the substrate node applies the DDL for everyone before
the others start. No renaming an entity, no reindex.

Destructive changes — dropping a field, changing a field's type (a retype drops and re-adds the column), or removing an
entity — drop the column and its data, so they are refused by default with a fatal error naming what would be lost. If
that loss is intended, set `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true` on the substrate node before restarting.

Note: the first restart after this feature is enabled only records the current schema as the baseline. Migration happens
on the restarts after that, when there is a baseline to diff against.
