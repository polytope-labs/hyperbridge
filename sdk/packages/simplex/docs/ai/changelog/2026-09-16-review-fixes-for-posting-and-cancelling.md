# 2026-09-16 — Review fixes for posting and cancelling

## A failure is not a refusal

`post` marked an order `rejected` whichever way the orderbook answered, and `rejected` is terminal:
nothing looks at that row again. One `TIMEOUT`, one `DATABASE_UNAVAILABLE` or one dropped connection
during a create and the operator's order was dead with nothing retrying it. A retryable failure now
leaves the row `open` with no commitment and the reason in `lastError`, which is the shape
reconciliation already knows how to post again. A refusal still retires the order.

## A cancel that got no answer says so

`signAndCancel` turned an unreachable orderbook into `UNKNOWN_ORDER`, which is the orderbook's
considered answer that the entry is already gone. `cancel` reads that as "already gone" and clears
the commitment, so a `cancelOrder` that merely timed out left a live entry nothing here owned.
`CancelOrderResult` has a `failed` variant now, and a cancel that fails keeps the commitment on the
row for reconciliation to find.

## `ORDER_EXISTS` is not `REPLAYED`

`REPLAYED` is an op hash the orderbook remembers for an order that is gone, and a new nonce is the
only way past it. `ORDER_EXISTS` is a live entry sitting at the commitment we just tried, so bumping
the nonce puts a second entry behind one liability and records only the second. The client can read
`order(solver, commitment)` now, and a live entry that is ours is taken as the posting it already is.

## The declared chains are checked before posting

`serverInfo.chains` lists every chain the orderbook serves. A fill chain or accepted source it does
not serve is refused on the way in, naming the ones it does, rather than coming back as
`UNSUPPORTED_SOURCE_CHAIN` against a row already stored. It answers half of that code: whether the
input symbol is registered on a chain is the server's own config and still cannot be checked here.
