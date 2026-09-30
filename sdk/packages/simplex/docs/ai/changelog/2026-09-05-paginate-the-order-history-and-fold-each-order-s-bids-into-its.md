# 2026-09-05 — Paginate the order history and fold each order's bids into its row

Added `GET /api/activity/history?page&pageSize` (`OrderHistoryDto`): one page of orders, newest
activity first, each with its rows and the Hyperbridge bids submitted for its commitment (bid
`commitment` equals the order id), plus the newest order-less events (rebalances) for the first
page's footer. Backed by `ActivityStore.orderHistory(page, pageSize)` (SQLite: `GROUP BY order_id
ORDER BY MAX(id)` with `COUNT(DISTINCT order_id)`; memory mirrors it) and `BidStore.byCommitments`.
`Orders.tsx` now pages (20 per page, numbered pager with ellipses, "Showing x–y of n"), re-reads
the current page on SSE activity (400 ms coalesced) instead of merging rows client-side, and shows a
Bids column ("2 bids · 1 accepted · 1 retracted", tooltip listing each bid) in place of the separate
Submitted bids table; the bid metrics strip stays, with pending retractions as a badge. `OperatorContext.bids`
gains `byCommitments`; the test operator now wires `bids`. Added a ui-server test for paging and
bid folding.
Files: `src/data/{types,memory,recorder}.ts`, `src/data/sqlite/{activity,bids}.ts`,
`src/services/server/{UiServer,dto}.ts`, `src/tests/ui-server.test.ts`,
`ui/src/operator/Orders.tsx`, `ui/src/types.ts`, `ui/src/styles/operator.css`,
`docs/ai/{ChangeLog,Decisions,Flow}.md`.
