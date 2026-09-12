# 2026-09-05 — Backfill order details on old activity rows from the indexer; decode referrer names

Rows recorded before order summaries existed showed only an id. `src/data/backfill.ts` now runs at
boot (fire-and-forget) and, for the newest 500 distinct order ids without a summary, queries the
Hyperbridge indexer's `iOrderV3s` entity (the SDK's `ORDER_STATUS` query targets `orders`/
`orderPlaceds`, which this indexer no longer serves), builds the same `OrderSummary` the recorder
would have, attaches it to every row of that order (`ActivityStore.attachOrder`, new alongside
`orderIdsMissingSummary`; SQLite and memory implementations), and re-emits the rows through the
recorder so open dashboards refresh over SSE. The endpoint defaults per network
(`DEFAULT_INDEXER_URLS`: nexus for mainnet, gargantua for testnet) and can be overridden with
`simplex.indexerUrl`, which `emit-toml` preserves. The referrer is now stored as the full 32-byte
graffiti tag: apps write their name as padded ASCII (the live indexer returns "HyperFX" that way),
so `ui/src/lib/format.ts` `describeReferrer` renders printable tags as text, address-shaped tags as
a short address, and anything else as short hex. Added tests for the backfill (indexed order
attached to every row, unknown order left alone, indexer failure touches nothing).
Files: `src/data/{backfill,recorder,types,memory}.ts`, `src/data/sqlite/activity.ts`,
`src/core/boot.ts`, `src/config/filler-toml.ts`, `src/cli/init/emit-toml.ts`,
`src/tests/{activity-backfill,activity-recorder}.test.ts`, `ui/src/lib/format.ts`,
`ui/src/operator/Orders.tsx`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
