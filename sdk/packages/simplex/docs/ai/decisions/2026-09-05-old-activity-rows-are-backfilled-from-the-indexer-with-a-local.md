# 2026-09-05 — Old activity rows are backfilled from the indexer with a local query, not the SDK's

Chosen: a one-shot boot-time backfill queries the Hyperbridge indexer's `iOrderV3s` entity directly
with `fetch` and writes summaries onto rows that predate capture. The maintainer found the fresh
history table sparse because every existing row was recorded before summaries existed.

Alternatives rejected: the SDK's `_queryOrderInternal` was tried first, but its `ORDER_STATUS` query
selects `orders` while its response type reads `orderPlaceds`, and the live mainnet indexer serves
neither (only `iOrderV3s`), so it cannot return this data today; fixing the SDK query is a separate
change with its own consumers. Reading the source chain's `OrderPlaced` log per commitment would
need the placement block, which the rows do not have. Backfilling lazily on `/api/activity/orders`
would make the first dashboard load wait on the indexer.

Also chosen: the referrer is stored as the whole 32-byte graffiti and decoded for display, after
the indexer showed HyperFX's tag is the ASCII name padded to 32 bytes rather than an address. The
earlier 20-byte truncation would have turned "HyperFX" into a meaningless address.
