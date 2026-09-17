# 2026-09-17 — SolverDelegation's boolean index made the schema uncreatable

`SolverDelegation.delegated` carried `@index`, and the entity carried a composite index
`["chain", "delegated"]`. In historical mode — the default, and what every deployment runs —
`addBlockRangeColumnToIndexes` in `@subql/node-core` rewrites **every** index as
`USING GIST (fields…, _block_range)`. `btree_gist` has no operator class for boolean, so Postgres
refuses:

```
ERROR: data type boolean has no default operator class for access method "gist"
```

The node then failed its schema migration and crash-looped, creating none of its tables. Confirmed
directly against Postgres 14: a gist index over `(boolean, int8range)` fails, the same index over
`(text, int8range)` succeeds, and `btree_gist` ships zero gist operator classes for `bool`. Enums are
fine — `anyenum` is covered — so `TrackedSolver`'s `["chain", "status"]` is safe.

Both indexes are gone. Filtering on `delegated` still works; it is a scan within whatever the
`(chain, solver)` index has already narrowed, and the orderbook's query filters on chain and solver.

This was not confined to the E2E: any deployment of this schema would have failed the same way, and
the existing local job would not have caught it — nothing there waits on the substrate node's health,
so its indexer can crash-loop while the job stays green.

Verified by running a real subql node against a fresh Postgres with this schema: 73 tables created,
including all eight solver tables, with every `solver_delegations` index gist over non-boolean
columns.

Files: `src/configs/schema.graphql`
