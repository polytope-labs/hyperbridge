# 2026-09-04 — In-place schema migration on restart, not versioned manifest upgrades (#1163)

Chosen: run the substrate node with `--allow-schema-migration` and a forked node image that, on startup, diffs the
previously applied schema (persisted in an `appliedSchemaSDL` metadata key) against the current one and ALTERs existing
tables in place — add a nullable column, table, index, enum value, or relation. Editing `schema.graphql`, rebuilding,
and restarting is the whole workflow; existing rows are kept and there is no reindex.

Alternative rejected — SubQuery's supported versioned-manifest upgrades (a `parent` deployment crossing an upgrade
block). It needs published/parent-CID bookkeeping we do not run (no IPFS), historical indexing (the substrate node runs
without it), and it reindexes from the upgrade block just to add a nullable column. The wrong cost curve for the actual
need, which is additive.

The flag is on the substrate node only. It is the fork image that carries the in-place logic, and compose already makes
it the health-gated leader every EVM (official-image) node waits on; because the shared `schema.graphql` covers every
entity, the leader applies all DDL before any EVM node boots, so there is no multi-writer race. Destructive changes are
refused by default (they drop a column and its data); `SUBQL_ALLOW_DESTRUCTIVE_MIGRATION=true` is the explicit escape
hatch. First boot after enabling the flag only seeds the baseline; migration happens on subsequent edits.
