# How requests to a Hyperbridge node are paced

`http()` calls `limiterFor(httpUrl)`, which returns the `TokenBucket` (`src/utils/rateLimiter.ts`) for that URL's origin from a module-level map, creating it at `HYPERBRIDGE_RPC_MAX_RPS` or 8 req/s. Every coprocessor in the process pointed at the same node therefore shares one bucket, because the limit being respected counts requests per address, not per connection.

`BatchingHttpProvider.send` queues the call rather than sending it, and flushes on the next macrotask — so everything issued in one burst travels as a single JSON-RPC batch request, up to 32 calls. One flush is in flight at a time; whatever arrives meanwhile rides the next. Each flush takes exactly one token from the bucket before its HTTP request, because a request is what the endpoint counts. A lone call is sent as a plain request object rather than a one-element array.

The bucket refills continuously and holds at most one second's worth, so a burst of requests up to that size goes straight out and the rest are granted at the configured rate. Waiters are strictly FIFO: a caller arriving on an idle bucket queues behind anyone already waiting rather than taking their token.

Two server refusals are handled without a call being lost. `-32005` (`--rpc-disable-batch-requests`) disables batching for the provider's life and retries the calls one per request; `-32010` (`--rpc-max-batch-request-len`) halves the batch size and retries. Both come back as a single error object where an array was expected, which is how they are told apart from a per-call error.

Everything reaching the node over HTTP passes through it — the poll's ranged or per-block scan, `fetchPhantomOrder` (fanned out one per configured chain by simplex's `handlePhantomOrders`), `queryApi()` consumers such as simplex's `BalanceProvider`, and `sendViaHttp` when the websocket is down at signing time. That is the point: the limit applies to their sum, and this is the only place their sum exists. It also means a long queue delays them all, which is why `maxBlocksPerPoll` is small.
