# 2026-10-05 — The limit order list is tabbed, and each row is a progress bar

First part of #1372. The Overview's limit order list is split by side and by status, and a row
shows how much of its order has been consumed. Nothing changes on the server: the list still reads
`GET /api/limit-orders`.

## Tabs

Two controls sit above the list, and the list shows the orders matching both.

- **Buys / Sells** selects `side` (`BID` / `ASK`). The number on each is that side's live orders.
- **Live / Filled / Cancelled** selects the status group, from `tabOf` in
  `ui/src/operator/limitOrders/limitOrderModel.ts`. The number on each is its count for the
  selected side.

| Tab | `LimitOrder.status` |
|-----|---------------------|
| Live | `open`, `resizing` |
| Filled | `filled` |
| Cancelled | `cancelled`, `expired`, `rejected` |

Each list pages ten at a time, and changing either control returns to the first page.
`PillTabs` options take an optional `count`, shown beside the label.

Posting an order moves the list to it: its side, the first page, and Live, or Cancelled when the
orderbook refused it. `useLimitOrders().create` returns the order from `POST /api/limit-orders`
for that, and `CreateLimitOrderForm` hands every order it posted to `onCreated`. With several fill
chains, a refused order is shown ahead of the ones that posted.

## Rows

A row leads with the rate (`rateParts`: the figure large, `quote per base` beneath it) and ends with
the cap, which is `LimitOrder.size` in the token the order pays out.

The row's background is the bar, from `progressOf`:

- the wash and the rule along its bottom cover `(size - remaining) / size`;
- a fainter stretch of the rule past it covers `reserved / size`, capped at what is left, since
  bids each hold against `remaining` without counting one another and their sum can pass it.

Both are cut to hundredths of a percent, so a bar never shows more consumed than was. The bar is
green for a buy, red for a sell, and grey for anything under Cancelled.

`describeProgress` puts the same figure in words: `62% filled`, `Nothing filled yet`,
`Filled in full`, with decimals only where a whole percent would read as 0 or 100.

The line beneath says what is left, what live bids hold, and the token taken in and its chain. A
closed order says how it ended and when, from `updatedAt`. An order closed under the dust floor
and a refused one lead with the date, ahead of the explanation, since a row cuts a long line short.

## Badges

`rowBadge` leaves an order that is on the book unbadged: being under Live already says so. Posting,
Resizing, Filled, Cancelled, Expired and Refused keep their badge. The detail sheet still shows
every status, On the book included.

An `open` order is on the book only once it has a `commitment`. Without one it is Posting, also
when a posting failed and left a `lastError` on it to be posted again. An order on the book with a
`lastError` (its old entry could not be cleared, or it is under-funded) stays unbadged. Either
way the second line shows the `lastError` in amber.

Below 900px a row folds to two lines, and below 640px the two controls each take the full width.
