# 2026-09-12 — Cancellation writes events and metadata only (#1185)

Supersedes `2026-09-12-cancellation-status-is-guarded-locally.md`. The user chose this narrow fix after review of the SubQuery handler API.

`recordOrderCancellation` saves `IOrderV3Cancellation` and `CANCELLED` status metadata, without saving `IOrderV3`. If the parent is missing, it writes `PendingStatusMetadata` directly; the established flush later materializes metadata without changing the parent's status. A parent appearing between reads must not cause a fallback through `updateOrderStatus`.

Rejected: a local `PLACED` guard, a second read, or a process-local mutex. Independent chain indexers have independent caches. SubQuery's cached store flushes complete entity snapshots; these approaches cannot condition a write on the database's current status and can overwrite a source-chain refund. The runtime's field-limited `bulkUpdate` is unsupported. Implementing conditional persistence would require a broader runtime change.

`EscrowRefunded` still owns the `REFUNDED` transition. Cancellation metadata is retained even when a terminal state is already indexed. Consumers must treat cancellation as initiation and prefer settled statuses over cancellation metadata. Repeated cancellation logs remain separate event records; the per-status metadata ID retains the existing convention.

## Runtime regression test

`scripts/tests/verify-cancellation-concurrency.cjs` uses two independent instances of SubQuery's actual `CachedModel`, separate database connections, and the actual bundled gateway service. It uses timestamp history, as in the multichain configuration. It first reproduces the stale whole-row regression, then verifies both refund/cancellation flush orders and the missing-parent metadata path. Each run creates and drops a disposable schema. Use a disposable Postgres database with permission to create schemas and the `btree_gist` extension.

From the repository root, with workspace dependencies installed:

```sh
node <<'NODE'
const esbuild = require(require.resolve('esbuild', {
  paths: [require.resolve('./sdk/packages/sdk/node_modules/tsup')],
}));
esbuild.buildSync({
  entryPoints: ['sdk/packages/indexer/src/services/intentGatewayV3.service.ts'],
  outfile: '/tmp/pr1185-service.cjs',
  bundle: true, platform: 'node', format: 'cjs', target: 'node22',
  tsconfig: 'sdk/packages/indexer/tsconfig.json',
});
NODE

# Set MIGRATION_DATABASE_URL to a disposable database reachable from this container.
docker run --rm --entrypoint node \
  -e MIGRATION_DATABASE_URL \
  -v /tmp/pr1185-service.cjs:/service.cjs:ro \
  -v "$PWD/sdk/packages/indexer/scripts/tests/verify-cancellation-concurrency.cjs:/verify.cjs:ro" \
  -v "$PWD/sdk/packages/indexer/src/configs/schema.graphql:/schema.graphql:ro" \
  polytopelabs/subql-node-substrate:v6.4.8-0 /verify.cjs
```
