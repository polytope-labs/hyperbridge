# 2026-09-05 — Bid placed and Retracted columns link to the Hyperbridge explorer

The Bids summary cell became two columns: "Bid placed" (time of the latest bid and its extrinsic
hash linking to `https://<nexus|gargantua>.statescan.io/#/extrinsics/<hash>`; a failed bid shows
"Failed" with its error) and "Retracted" (retraction time and its extrinsic link, or a dash).
`OrderHistoryDto` gains `network` (from a shared `runningNetwork` helper in `UiServer`) so the UI
picks the explorer; `BidDto` declares the `retractedAt` and `retractExtrinsicHash` fields the API
already returned. Also settles bids and retypes legacy bid-time "filled" rows at boot (the
settlement pass added to `backfillOrderSummaries`, keyed on `volumeUsd` being set only on bid-time
rows), using `ActivityStore.unsettledOrders` / `retypeLegacyBid`.
Files: `src/services/server/{UiServer,dto}.ts`, `src/data/{backfill,types,memory}.ts`,
`src/data/sqlite/activity.ts`, `src/core/boot.ts`, `src/tests/{ui-server,activity-backfill}.test.ts`,
`ui/src/operator/Orders.tsx`, `ui/src/lib/format.ts`, `ui/src/styles/operator.css`,
`docs/ai/{ChangeLog,Flow}.md`.
