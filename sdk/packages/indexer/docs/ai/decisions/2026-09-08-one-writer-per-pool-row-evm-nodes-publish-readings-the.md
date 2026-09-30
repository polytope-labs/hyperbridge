# 2026-09-08 — One writer per pool row: EVM nodes publish readings, the Hyperbridge node folds them (#1214)

The setting this answers: the multichain indexer is one SubQuery node process per chain, all writing one Postgres
schema. Each process serves `Entity.get` from a private LRU (500 entries, one hour, refreshed on every read) that
nothing another process writes ever invalidates, flushes whole rows per block with no locking, and, in the
historical timestamp mode every node here runs under, closes the previous row by range and inserts a new one with
no uniqueness on (id, range). Every cross-chain swap had the destination node (`OrderFilled`), the source node
(`EscrowReleased`) and the Hyperbridge node (snapshot) all re-deriving and saving the same `LiquidityPool` row
from private copies. Verified against `@subql/node-core` 19.3.1 (the EVM image) and the forked substrate node,
whose changes touch only RPC resilience and schema migration.

Chosen: every row has exactly one writer. The pool family belongs to the Hyperbridge node. An EVM node publishes
what it measured into `SolverInventoryReading`, keyed by its own chain, and `foldInventoryReadings` on the
Hyperbridge node turns those into pool rows. A process that is the only writer of a row is always consistent with
its own cache, so both halves of the race disappear by construction rather than by coordination.

Alternative rejected — derive the aggregate in the database (a view over the chain rows). Removes the pool-level
race but not the chain-row race between the snapshot writer and the EVM refresh, and needs raw SQL outside the
schema-driven migrations.

Alternative rejected — advisory locks and direct SQL from the handlers. Possible (`--unsafe`, DB credentials in
the environment) but bypasses the store cache and the historical ranges, so it fights the framework everywhere.

Alternative rejected — keep several writers and only switch cross-node reads to field queries. Fixes the stale
reads, leaves the concurrent whole-row overwrite; narrows the window without closing it. Field queries are still
the rule for every cross-node read (they go to Postgres; `get` does not), which is why `declaredV4Positions` now
reads through one, on the provider link rather than the id: the store only indexes `id` in historical mode. The FX-pricing read of `LiquidityPool` in `IntentGatewayV3Service` is a remaining
cross-node `get`, out of scope here.

Chosen: an EVM node reads only its own chain, pinned to the event's block. A pool spans chains, but the event
moved inventory on one of them; the other chains' inventories are their own nodes' to publish, and the periodic
phantom sweep still corrects drift everywhere. This is also what keeps the reading rows single-writer.

Chosen: the fold scans the whole reading table each block and filters in memory. The store's `getByFields`
offers only `=`, `!=`, `in` and `!in`, so "newer than a watermark" cannot be expressed; the table is bounded by
bidders times tokens times chains, and an in-process memo of what was folded makes a quiet block cost one page.
Idempotency rests on the per-row compare, not on the memo: a reading applies only if it postdates the bidder
row's `lastUpdatedAt` and the new nullable `refreshedAt`. Nullable because the in-place migration refuses a
non-null column, and separate from `lastUpdatedAt` because that pair records which snapshot priced the pool.
Bidder rows are matched to a reading by provider, chain and output token in memory, case-insensitively, so no
index on `outputToken` was added.

Chosen: the fold runs on every Hyperbridge block. The Hyperbridge manifest uses no dictionary, so the node already
fetches every block, and the handler sits inside the `enableLiquidityIndexing` block of the substrate template —
placing it beside `handlePendingStatusFlush` would have run it on every substrate chain.

Chosen: balance-series rows are keyed by the fold's own Hyperbridge block with the reading's observation time as
`snapshotTime`. The series keeps its clock without the EVM nodes reading Hyperbridge's head over RPC, which is
gone. Same-key collisions with a snapshot on the same block still resolve to the larger reading.

Known limits, stated rather than solved: a node reads only rows whose historical range contains its own current
block time, so an EVM node running ahead of Hyperbridge writes readings the fold sees a block or two later, and
the mainnet template flushes the store asynchronously every five seconds, which adds to that lag. A lagging EVM
node's replayed readings are older than the rows' snapshot times and are ignored. Both schema changes are
additive; if the deployment has not restarted since in-place migration was enabled, the first restart records the
baseline and the DDL lands on the second. The disabled multichain rewind lock combined with unfinalized EVM blocks
is a separate race, not addressed here.
