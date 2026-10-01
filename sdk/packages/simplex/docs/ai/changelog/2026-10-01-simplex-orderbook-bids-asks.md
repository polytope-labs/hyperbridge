# Simplex order book

The Order book sidebar page (`/orderbook`) shows the public bids and asks across all
solvers. Its styling follows the HyperFX app's pair page (`hyperbridge-fe`
`src/components/order-book`), condensed so the ladder is visible without scrolling. The
app's pair header, depth chart, and price history chart are deliberately left out.

- **Summary**: `Pair`, `From` (source chain), and `To` (fill chain) filters, then mid
  price with a live indicator, best buy, best sell, spread (price and bps of mid), buy
  liquidity (quote), and sell liquidity (base). The stats sit six across when the page
  content is at least 980px wide, three across below that, and two on phones. The filters
  stay mounted while a route loads or fails, so the focused select is kept and a failing
  book can be switched away from.
- **Ladder**: one full-width card, sells above the mid row and buys below, best prices
  against the mid. Columns: price (quote per base), fill chain, orders · solvers, size
  (base), total (quote notional), and sum (cumulative base). Narrow layouts keep only
  price, size, and sum. A price's title shows the level's worst price. Depth bars measure
  cumulative base against the side's total. Eight levels per side show at first; one
  button expands both.

Prices show the fraction digits the book's price bucket needs (2 to 6). Sizes show two
decimals, compact from one million. Symbols display upper-case (`CNGN`).

## API

`GET /api/orderbook/snapshot?book=<id>&sourceChain=<id>&fillChain=<id>` returns the
book's levels, `bidLiquidity`, `askLiquidity`, and `granularity` (the book's price bucket
width from `serverInfo.priceGranularities`, or null). Chain filters are optional. Values
stay integer strings at 1e18. A missing book returns 400, an unknown book 404, and an
orderbook transport or GraphQL failure 502.

## Refresh and derived values

The snapshot refreshes ten seconds after each request completes. Changing the pair or
route hides previous prices and aborts the old request. A failed refresh keeps the last
snapshot for that route and marks it stale.

Mid, spread, and best prices are derived from the filtered levels, not the book-wide
fields, so a route filter never quotes a price the ladder lacks. Crossed books are
labeled because the best levels can fill on different chains; one-sided books have no
mid or spread. An empty book says no solver is quoting.

Backing liquidity reads `availableLiquidity` with the same filters. It differs from
quoted depth: several orders can share one solver balance, so their advertised sizes
must not be summed and presented as funds available.
