# 2026-09-24 — quoteIntent returns unfillable quotes

`IntentGateway.quoteIntent` no longer throws when the route cannot fill the trade. It returns the orderbook's quote with `fillable: false`, and `maxFillableIn` holds the most the route could fill, in raw source-token units. The optimistic quote then has empty `legs`. The pessimistic quote has `amountOut` 0 and a `null` `rate` and `priceBucket`.

- **Exact input** returns the orderbook's answer for `amountIn` as served.
- **Exact output** past the route's `maxFillableIn` sends the orderbook an input beyond it and returns that unfillable quote. The input is guessed from the best rate, or is one raw unit when no order serves the route. `amountIn` is the input the SDK last tried.
- `InsufficientOrderbookLiquidityError` is removed. `OrderbookQuoteNotConvergedError` and `OrderbookRequestError` still throw.

Validation: `orderbookMarket.test.ts` covers unfillable exact input for both quotes, and exact output past the route's depth and on a route with no orders. Queried directly, the testnet orderbook answers an input past its depth with `fillable: false` for both `quote` and `quotePessimistic`. The sequential suite could not run here because the public BSC Chapel RPCs rejected or rate-limited requests.
