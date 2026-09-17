# 2026-09-17 — Gate the E2E's query service on the node that owns the schema

The solver-inventory E2E failed with `Unknown field "trackedSolvers" on type "Query"`, repeated until its
fifteen-minute timeout. The entities were missing from the GraphQL schema entirely, not merely empty.

`polytopelabs/omnihedron` reads the database schema **once at startup** — its CLI has no watch or reload option —
so a service that boots before the indexer has created its tables serves a `Query` type without them for the rest
of the run. The compose file only made it wait for postgres, which is healthy long before any DDL has run.

Confirmed directly: against a schema whose entity table was created after the query service booted, the field is
unknown; restarting the service against that same database resolves it and returns rows.

- **`docker-compose.solver-ci.yml`.** `graphql-engine` now also waits for the substrate node, which owns the
  schema's DDL, to be healthy. Port 3100 therefore opens only once the entities exist.
- **`verify-solver-inventory.cjs`.** An `Unknown field` error is now fatal instead of retried. Waiting cannot fix
  a missing entity type, and retrying buried the one line that explained the failure. It exits in about a second.
- **The workflow** probes for the entities after the port opens and, if they are somehow still missing, restarts
  the query service once with a `::warning::` before giving up with the container logs.

`docker-compose.local.yml` has the same latent race — its `graphql-engine` waits for nothing at all. It is left
alone here: that job is green today, and gating it could turn a passing run into a hang if its Hyperbridge node
never reports healthy. Worth fixing separately.

Files: `docker/docker-compose.solver-ci.yml`, `scripts/tests/verify-solver-inventory.cjs`,
`.github/workflows/test-solver-inventory.yml`
