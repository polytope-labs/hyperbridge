# 2026-09-28 — Simplex docs describe limit orders

The Simplex operator guide (`docs/content/developers/evm/simplex/`) now documents pricing through
limit orders on the HyperFX orderbook, replacing the price-curve and Uniswap V4 pages.

- `limit-orders.mdx` is new. It covers creating an order from the dashboard step by step (pair,
  side, amount in the paid-out token, rate in quote per base, accepted sources, fill chains),
  reading and cancelling orders, and how orders are matched and filled (including the fee check
  and partial fills).
- `limit-orders-api.mdx` is new. It covers managing orders directly over `/api/limit-orders` and
  `/api/orderbook/books`, including same-asset quotes, which only the API can create.
- `pricing.mdx` (Uniswap V4 funding and pool pricing) and `markets.mdx` (pairs and curves) are
  removed. `docs/vercel.json` redirects both to the limit orders page.
- `configuration.mdx` documents the required `[orderbook]` section and its defaults. The
  installation, treasury, confirmations, dashboard, overview and troubleshooting pages drop the
  curve and V4 references.
- The screenshots were captured from the operator UI running against a mock API with sample data.
- The SDK library guide, the API reference, the package README and
  `filler-config-example.toml` describe `simplex.limitOrders` and `[orderbook]` in place of
  curves, `maxOrderSize`, `referenceOnly` and `[vault.uniswapV4]`.
- `@hyperbridge/simplex` now exports the limit-order surface from its root:
  `LimitOrderController` and `LimitOrderValidationError` as values, and as types `LimitOrderStore`,
  `LimitOrder`, `LimitOrderInsert`, `LimitOrderFilter`, `LimitOrderPosting`, `LimitOrderSide`,
  `LimitOrderStatus`, `LimitOrderFill`, `LimitOrderFillInsert`, `LimitOrderHold`,
  `CreateLimitOrderRequest`, `PostedLimitOrder`, `CancelledLimitOrder` and `PostingOutcome`. A
  custom `SimplexDataStore` needs these to implement `limitOrders`, and `StoredBid` already
  referenced `LimitOrderHold`.
- `[simplex.overfillProtection]` is documented as it behaves: `maxOverfillBps` only logs a
  warning, and `maxConsecutiveClamps` is accepted but can no longer halt the filler, because the
  clamp that fed it existed for Uniswap V4 venue pricing.
