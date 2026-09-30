# 2026-09-07 — Action filters and pagination on the wallet ledger

The ledger rendered up to 200 rows in one table. It now has the order history's pager and a pill row
filtering by action (all, fills, sends, sweeps, redeems), 20 rows a page, with the page resetting
when the filter changes. Both are client-side because `/api/wallet/history` returns one merged,
sorted page of wallet transactions and fills rather than a queryable table; the endpoint is
unchanged. `Pager` and `pageNumbers` moved out of `Orders.tsx` into `ui/src/components/Pager.tsx`
with a `noun` prop, so the two pages cannot drift, and it now says "1 order" rather than "1 orders".

Files: `ui/src/components/Pager.tsx`, `ui/src/operator/{Wallet,Orders}.tsx`, `docs/ai/ChangeLog.md`.
