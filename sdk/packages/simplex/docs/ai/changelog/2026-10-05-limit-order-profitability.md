# 2026-10-05 — Profitability from limit order fills

Second part of #1372. The Overview shows the spread realized over the last seven days, and a new
Analytics page breaks profit down over a period the operator picks.

## A fill records what it took in

`limit_order_fills` gains `amount_in`: the escrow the fill released to the solver, at 1e18 in the
token the order takes in. Existing databases get the column added in place, and their rows stay
null.

The fill event counts that escrow in the input token's own units on the order's source chain, and
only the bid knows that token's decimals. So `BidPlan` carries `inputDecimals`, the hold a bid takes
carries it as `LimitOrderHold.takeDecimals`, and `settleFilledLimitOrder` scales the released input
with it. A fill whose event carried no inputs, or whose hold predates this, records null.

`LimitOrderStore.fillHistory()` reads every fill, oldest first, joined with its order's `book`,
`base`, `quote`, `side` and `price`. It is a required method, so a custom `LimitOrderStore` has to
add it, and should keep `amountIn` on the fills it records.

## How profit is counted

`summarizeProfit` in `src/orderbook/profitability.ts` replays every fill in order, per book, against
a position kept at average cost.

- A buy (`BID`) pays quote and takes base; a sell (`ASK`) the reverse. The rate of a fill is what
  it paid against what it took in. A fill with no `amountIn` is priced at its order's `price`, and
  counted in `estimatedFills`.
- A fill that adds to what is held moves the average and realizes nothing. A fill the other way
  closes what is held at that average, and the difference between its rate and the average is the
  profit. Volume past what was held opens a position the other way at the fill's own rate.
- Profit is counted in the bucket of the fill that realized it. Fills from before the period still
  run, since they set the cost the period's fills close against.
- A book whose base and quote are one asset holds no position: each fill realizes what it took in
  less what it paid out.

Average cost was chosen over comparing a period's average sell with its average buy because it adds
up across buckets and counts volume bought before the period and sold inside it.

Dollars come from whichever of a book's tokens is a dollar stable (`USD_STABLE_SYMBOLS`): the quote
at face, or the base through the rate of the fill being valued. A book with neither is left out of
every dollar figure and named in `unpricedBooks`; its own row still carries its profit in quote.
An open position is valued at cost, not marked to market. Network fees and `order.fees` are not
counted.

## The API

`GET /api/analytics/profitability?period=<7d|30d|12w|12m|all>&tz=<minutes>`

- `period` defaults to `7d`. Each period has one bucket size: `7d` and `30d` by day, `12w` by week
  (starting Monday), `12m` by month, and `all` by year from the first fill.
- `tz` is what JavaScript's `getTimezoneOffset()` returns on the viewer's clock, so a bucket starts
  at their midnight. It defaults to `0` and must be a whole number of minutes within 14 hours.
- An unknown period or a bad `tz` answers `400`.

The response (`Profitability`) holds `totals`, one `series` entry per bucket including empty ones,
one `books` entry per book, `openPositionUsd`, `estimatedFills` and `unpricedBooks`. `spreadPct` is
`realizedUsd / matchedUsd` as a percentage, and null when nothing was matched.

`simplex.limitOrders.profitability(period, tzOffsetMinutes)` is the same read from the library.

## The dashboard

- **Overview** gains a fourth summary figure, **7-day spread**: `totals.spreadPct` for `7d`, green
  above zero and red below, with the dollars realized beneath it. With nothing matched it is a
  dash and says why. It links to Analytics.
- **Analytics** is a new page at `/analytics`, after History in the sidebar. It has a period
  control, four figures (realized profit, spread, matched volume, open position), the running
  total as a line over each bucket's profit as bars, a table by pair and a table by bucket.
  Pointing at a bucket in either chart reads its figures out above them. The charts are drawn at
  the width of their container, so their type stays legible on a phone.
- The handheld bottom bar has seven columns.
