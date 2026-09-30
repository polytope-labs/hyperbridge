# 2026-09-04 — In-place substrate schema migrations (#1163 follow-up)

Enabled additive schema migrations for the substrate node so a new field on an existing entity no longer forces a
rename that orphans the old table's data. The behaviour lives in the forked node image (`polytopelabs/subql-node-substrate`),
bumped to `v6.4.8-0`; this repo turns it on with `--allow-schema-migration` on the substrate node only (it is the
health-gated leader every EVM node waits on, and the shared `schema.graphql` covers all entities). Destructive changes
(drop or retype a field, drop an entity) stay refused unless `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true`.
Files: scripts/generate-compose.ts, scripts/templates/partials/docker-command.hbs, docker/docker-compose.local.yml, docker/docker-compose.nexus-ci.yml, README.md, docs/ai/Decisions.md, docs/ai/Flow.md.
