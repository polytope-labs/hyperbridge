# 2026-09-15 — Limit orders posted to the HyperFX orderbook

The operator can now create limit orders, and simplex advertises them on the HyperFX orderbook.
An operator states a limit order as what simplex takes in and what it pays out for that, for example
10,000 USDC in for 139,000,000 cNGN out. The rate and the side of the book follow from those two
amounts, so an order is directional by construction: that one prices USDC to cNGN swaps and can
never price cNGN to USDC, which needs its own order. An operator can hold as many at once as they
like, and an incoming order is matched against all of them.

Orders live in a new `limit_orders` table in `bids.db` behind `SimplexDataStore.limitOrders`, and the
orderbook entry is a derived copy that expires and is reposted. Amounts and prices are decimal
strings at 1e18 everywhere they cross the orderbook boundary, whatever decimals the tokens use on
their own chains.

## The API

`Simplex.limitOrders` and four routes on the operator server:

```
GET    /api/limit-orders?status=&chain=&book=
GET    /api/limit-orders/:id
POST   /api/limit-orders     { fillChain, tokenIn, amountIn, tokenOut, amountOut, acceptedSources, ttlSecs? }
DELETE /api/limit-orders/:id
```

`amountIn` and `amountOut` are whole tokens, as decimal strings: `"1000"`, `"1500.25"`. Nobody
creating an order should have to know an asset's decimals, let alone that the orderbook normalises
everything to 1e18, so the handler scales them while it validates. Anything with more than 18 decimal
places, or that is not a plain decimal, is refused with the amount named.

`acceptedSources` is required and non-empty: it names the source chains the order accepts swaps
from, and the orderbook refuses an order that declares none. A create emits `limit-order:posted` or
`limit-order:rejected`, a cancel emits `limit-order:cancelled`.

A request is validated against `serverInfo` and `books` before anything is stored, so the rejection
codes the orderbook reserves for bad requests (`TTL_TOO_SHORT`, `MIN_ORDER_SIZE`, `UNSUPPORTED_PAIR`)
should never come back. The order is stored before it is posted, so a posting that fails leaves a
row carrying the reason rather than a request that vanished.

## What gets signed

`ContractInteractionService.prepareLimitOrderUserOp` builds a `fillOrder` UserOperation for a
synthetic same-chain order at the operator's rate. It is a price commitment, not a transaction:
nothing ever submits it, the orderbook only verifies the solver signature over the userOpHash and
reads the amounts out of the calldata. So the gas fields are fixed rather than estimated, and
`paymasterAndData` carries the accepted-source declaration instead of a paymaster.

`FillOptions` takes one quote per leg, so the order's single leg carries `outputs[0]` — what the
operator pays — and `inputs[0]`, the whole input they want for it. That pair is the rate, since the
order's own output amount is zero as the orderbook requires. `validUntil` is a TTL in seconds from
the orderbook's receipt, not a block number.

There is one `fillOrder` shape, the one the gateway speaks, so the op is built without choosing a
version: the encoder has no other to offer.

The derived rate is rounded in simplex's favour, and so is the input the op is built from, so
neither the stored rate nor a repost quotes better than the two amounts the operator gave. `REPLAYED` and `ORDER_EXISTS` are answered once by bumping
`orderNonce`, which changes the commitment and the userOpHash; the orderbook remembers every hash it
has accepted, so a fresh nonce is the only way past.

## Config

```toml
[orderbook]
url = "https://orderbook.hyperbridge.network/graphql"
defaultTtlSecs = 900
reconcileIntervalSecs = 300
requestTimeoutMs = 10000
```

The section is required. Simplex prices every fill from the operator's limit orders and those
live on the orderbook, so there is no configuration without one.

`ttlSecs` is the only clock. It is the TTL written into the posting and the life of the order
itself: `expiresAt` is derived from it when the order is created, and nothing renews it. When it
runs out the posting lapses and the order is done.

The limit orders themselves are not configured here. They are inventory the
operator opens and closes while the filler runs, so they live in `bids.db` and are created over the
API.

Pricing still comes from the pair curves; matching incoming orders against limit orders, and drawing
`remaining` down on a fill, land separately.
