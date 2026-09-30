# Substrate schema migrations (verified 2026-09-04, against the forked node-core)

How a change to `src/configs/schema.graphql` reaches the database. All nodes share one `schema.graphql` and one
`--db-schema=app`; the substrate node runs the forked `polytopelabs/subql-node-substrate` image with
`--allow-schema-migration`, the EVM nodes run the stock `subql-node-ethereum` image.

1. On boot each node's main thread runs `StoreService.init` (node-core). With the flag off (the EVM nodes) it calls the
   schema migration with a `null` baseline, which only emits `CREATE TABLE IF NOT EXISTS` — existing tables and their
   columns are left untouched.
2. With the flag on (the substrate node), `init` reads the previously applied schema from the `appliedSchemaSDL`
   metadata key, rebuilds it into a baseline `GraphQLSchema`, and diffs it against the current one. Additive changes
   (add nullable column, table, index, enum value, relation) are ALTERed in place; the new schema is written back to
   `appliedSchemaSDL` in the same transaction as the DDL.
3. A destructive diff (a removed or retyped field, a removed entity — a retype surfaces as remove+add) is refused with a
   fatal error unless `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true`, because it would drop the column and its data.
4. Compose gates every EVM node behind the substrate node's `/ready` healthcheck, and `init` (including the migration)
   completes before `/ready`. So the substrate leader applies the whole shared schema's DDL before any EVM node starts;
   the EVM nodes' `CREATE TABLE IF NOT EXISTS` is then a no-op. There is no multi-writer race.

The engine that performs the ALTERs (`SchemaMigrationService`) already existed; the change was giving it a real baseline
on restart instead of `null`. Adding a field is therefore: edit `schema.graphql`, `pnpm build`, restart. No rename, no
data loss, no reindex.
