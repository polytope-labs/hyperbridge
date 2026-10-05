# 2026-10-05 — Profitability from limit order fills

Second part of #1372. The Overview shows the return on inventory over the last seven days, and a
new Analytics page breaks profit down over a period the operator picks.

## A fill records what it took in

`limit_order_fills` gains `amount_in`: the escrow the fill released to the solver, at 1e18 in the
token the order takes in. A new database is created with it. An existing one gets the column
added in place the first time this version opens it, and its rows stay null. The column is
nullable with no default, so a solver still on the earlier version can keep writing fills to the
same file.

The fill event counts that escrow in the input token's own units on the order's source chain, and
only the bid knows that token's decimals. So `BidPlan` carries `inputDecimals`, the hold a bid takes
carries it as `LimitOrderHold.takeDecimals`, and `settleFilledLimitOrder` scales the released input
with it. A fill whose event carried no inputs, or whose hold predates this, records null.

`LimitOrderStore.fillHistory()` reads every fill, oldest first, joined with its order's `book`,
`base`, `quote`, `side` and `price`. It is a required method, so a custom `LimitOrderStore` has to
add it, and should keep `amountIn` on the fills it records.

## Inventory snapshots

`activity.db` gains `inventory_snapshots (id, taken_at, balances)`: what the solver held, as whole
tokens per symbol in a JSON object. A database from an earlier version simply gains the empty table.

`InventoryRecorder` (`src/data/inventory.ts`) listens to the balance provider and records a
snapshot when the last one is a day old. It only records a complete read (`status: "fresh"`, every
asset's `total` known): a read that missed a chain would record a fall in inventory that never
happened. Inventory is `total` per symbol, wallet and vaults together, summed over every chain. A
signerless filler records nothing.

`SimplexDataStore` gains a required `inventory: InventoryStore` (`record`, `latest`, `since`), so a
custom data store has to add it.

## How profit is counted

`summarizeProfit` in `src/orderbook/profitability.ts`. Decided with David on 2026-10-05: profit is
the change in inventory over the period, compared with the inventory at the period's start.

- A buy (`BID`) takes base in and pays quote out; a sell (`ASK`) the reverse. A fill with no
  `amountIn` has the side it took in worked out from its order's `price`, and is counted in
  `estimatedFills`.
- Added up per token, the period's fills are what the period did to inventory. Its value in dollars
  is the profit.
- Each token has one dollar price throughout: stables at $1, and every other token reached through
  each book's latest rate (`usdFactorsFrom`). A book's rate is midway between its latest buy and
  its latest sell, over all of its history, since either alone sits half a spread to one side.
- So volume bought and not yet sold counts at the latest rate, and a figure for a past bucket moves
  when the rate does. Buckets always add up to the period.
- A token no book connects to a dollar stable is left out of the dollar figures and named in
  `unpricedTokens`.
- Network fees and `order.fees` are not counted.

## The inventory a period began with

`inventoryAt(instant)` takes the record nearest the instant: a stored snapshot, or the balances as
they stand now. It then carries that record to the instant by the fills between them and by the
sends recorded in the wallet ledger. A sweep or a redeem is not an outflow, since vault positions
are inventory. A token that comes out negative is counted as zero.

A record is used only within `INVENTORY_REACH_MS`, seven days, of the instant. A deposit from
outside is on no record, so the further a record is carried the less it can be trusted. This is why
today's balances can stand in for the start of the 7-day period on the day of the upgrade, while a
30-day period shows its profit with no return until a snapshot exists near its start.

`returnPct` is `profitUsd / startInventoryUsd`, and null when the inventory is unknown or zero. Each
bucket is compared with the inventory at its own start, resolved the same way.

## The API

`GET /api/analytics/profitability?period=<7d|30d|12w|12m|all>&tz=<minutes>`

- `period` defaults to `7d`. Each period has one bucket size: `7d` and `30d` by day, `12w` by week
  (starting Monday), `12m` by month, and `all` by year from the first fill.
- `tz` is what JavaScript's `getTimezoneOffset()` returns on the viewer's clock, so a bucket starts
  at their midnight. It defaults to `0` and must be a whole number of minutes within 14 hours.
- An unknown period or a bad `tz` answers `400`.

The response (`Profitability`) holds `totals`, one `series` entry per bucket including empty ones,
one `books` entry per book, `startInventory` (its dollar value, its tokens, and whether it came
from a `snapshot` or from `balances`), `estimatedFills` and `unpricedTokens`.

`simplex.limitOrders.profitability(period, tzOffsetMinutes)` is the same read from the library.

## The dashboard

- **Overview** gains a fourth summary figure, **7-day return**: `totals.returnPct` for `7d`, green
  above zero and red below, with the profit and the inventory it is a return on beneath it. With
  nothing to compare it is a dash and says why. It links to Analytics.
- **Analytics** is a new page at `/analytics`, after History in the sidebar. It has a period
  control, four figures (profit, return on inventory, starting inventory, volume), the running
  total as a line over each bucket's profit as bars, a table by pair and a table by bucket.
  Pointing at a bucket in either chart reads its figures out above them. The charts are drawn at
  the width of their container, so their type stays legible on a phone.
- The handheld bottom bar has seven columns.
