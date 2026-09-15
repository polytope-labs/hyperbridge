# 2026-09-15 — Limit orders posted to the HyperFX orderbook

The operator can now create limit orders, and simplex advertises them on the HyperFX orderbook.
A limit order is what simplex offers to pay: a book, a side, a fill chain, a price and a size. It
lives in a new `limit_orders` table in `bids.db` behind `SimplexDataStore.limitOrders`, and the
orderbook entry is a derived copy that expires and is reposted. Amounts and prices are decimal
strings at 1e18 everywhere they cross the orderbook boundary, whatever decimals the tokens use on
their own chains.

## The API

`Simplex.limitOrders` and four routes on the operator server:

```
GET    /api/limit-orders?status=&chain=&book=
GET    /api/limit-orders/:id
POST   /api/limit-orders     { book, side, fillChain, price, size, acceptedSources, ttlSecs?, expiresAt? }
DELETE /api/limit-orders/:id
```

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

It is always encoded as FillOptions v2, without consulting the deployed gateway: the op never runs,
and v1 has nowhere to put `validUntil`, which the orderbook requires. `validUntil` here is a TTL in
seconds from the orderbook's receipt, not a block number.

The input is rounded up against the operator's price, so the rate the op carries is never better for
the taker than the price asked for. `REPLAYED` and `ORDER_EXISTS` are answered once by bumping
`orderNonce`, which changes the commitment and the userOpHash; the orderbook remembers every hash it
has accepted, so a fresh nonce is the only way past.

## Config

```toml
[orderbook]
enabled = true
url = "https://orderbook.hyperbridge.network/graphql"
defaultTtlSecs = 900
renewMarginSecs = 120
reconcileIntervalSecs = 300
requestTimeoutMs = 10000
```

Off unless enabled, and the limit orders themselves are not configured here. They are inventory the
operator opens and closes while the filler runs, so they live in `bids.db` and are created over the
API.

Pricing still comes from the pair curves; matching incoming orders against limit orders, and drawing
`remaining` down on a fill, land separately.
