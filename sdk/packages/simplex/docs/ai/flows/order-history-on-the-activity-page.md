# Order history on the Activity page

`OrderPlaced` logs decode with `args.graffiti`; `reconstructOrdersFromLogs` copies it onto
`ReconstructedOrder`, the chain scanner spreads it into `ScannedOrder`, and `EventMonitor.handleOrder`
emits `newOrder` as `{ order, transactionHash, graffiti }`. `ActivityRecorder` subscribes and, before
writing the `detected` row, builds an `OrderSummary`: 20-byte user, source/destination state machine
ids, the placement tx hash, the referrer (graffiti's last 20 bytes, or null when zero or equal to
the user), each input/output leg as `{ token, amount, symbol, decimals }` via the injected
`describeToken` (registry symbol match over built-in plus `[assets]` symbols, decimals from
`ContractInteractionService.getTokenDecimals`; failures leave nulls), and the deadline. The summary
is cached per order id (2,000 entries) and attached to that order's later `filled`, `executed` and
`skipped` rows. `SqliteActivityStore` persists it as `order_json` (column added by a
`PRAGMA table_info` migration on open); `/api/activity/orders` and the SSE stream return it as
`ActivityEventDto.order`. Because the detection write awaits token lookups, a `skipped` row can
land before its `detected` row; `Orders.tsx` therefore groups events by order id and derives the
status by precedence (filled > executed > skipped > detected), not by row order. Each order renders
one row: referrer, status badge with detail (the strategy, or the skip/failure reason; a fill carries
no detail), amount in and out (token icon with a chain badge, amount formatted from decimals by
`ui/src/lib/format.ts`, chain label from `chainLabels` or the init catalog), user, placed time and
date, and links to the HyperFX order page, the placement tx and the fill tx on the chains'
explorers. Rows written before this change start with `order: null` and show the id; at boot
`backfillOrderSummaries` (`src/data/backfill.ts`) lists up to 500 such order ids
(`ActivityStore.orderIdsMissingSummary`), queries the network's indexer (`simplex.indexerUrl`, else
nexus/gargantua by whether any resolved chain is testnet) for each with four parallel workers,
builds the summary with the same `describeToken`, writes it onto every row of the order
(`attachOrder`), and re-emits those rows as recorder `event`s so the SSE stream replaces them in
open dashboards. Orders the indexer does not know keep `order: null`. The referrer is the full
32-byte tag; `describeReferrer` shows padded-ASCII tags as text ("HyperFX"), zero-prefixed tags as
a short address, and other values as short hex.

Paging: `Orders.tsx` requests `/api/activity/history?page=N&pageSize=20`. `UiServer` calls
`activity.orderHistory(page, pageSize)` — SQLite groups `events` by `order_id`, orders groups by
their newest row id, counts distinct orders for `total`, then loads the page's rows in one `IN`
query — and `bids.byCommitments(orderIds)` (bid `commitment` is the order id), returning each order
with its rows (newest first) and bids (newest first), plus the newest order-less events for page
one's footer. The status, legs, user and links derive from the rows as before; the Bids cell counts
accepted (successful, unretracted), retracted and failed bids with a tooltip per bid. An SSE frame
of any kind schedules a re-read of the current page after 400 ms.
