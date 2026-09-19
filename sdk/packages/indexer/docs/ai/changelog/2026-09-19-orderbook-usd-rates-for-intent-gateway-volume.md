# 2026-09-19 — Intent gateway USD volume is priced from the HyperFX orderbook

`fetchOrderbookUsdPrice` (`src/services/orderbookRates.service.ts`) was a mock returning no price. It now
queries the orderbook's GraphQL API, so a token without a $1 peg contributes to `CumulativeIntentGatewayVolumeUSD`
instead of being skipped with a warning. Tokens in `STABLE_SYMBOLS` (`USDC`, `USDT`) are still taken as $1 without
a query; that list moved into the rates service, which both prices against it and shares it with
`IntentGatewayV3Service`.

**The rate.** One `POST` asks `bestRate(tokenIn: <symbol>, tokenOut: <stable>) { base quote rate }` for every
stable as aliases in a single document, with the symbol as a GraphQL variable — it comes off an ERC-20 `symbol()`
call and is never interpolated into the query.

Every rate is quote per 1 base at 1e18, on both sides of the book, and the entry names both symbols, so where the
stable sits decides the arithmetic:

- the stable is the `quote` — the rate is dollars per 1 token, already the price;
- the stable is the `base` — the rate is tokens per $1, so the price is its reciprocal. The `USDC`/`cNGN` book
  quoting 1,500.5 cNGN per USDC prices one cNGN at 1/1500.5 dollars.

`side` is deliberately not read: it says which of `tokenIn` and `tokenOut` is the base, leaving the units to be
derived, where `base` and `quote` name them outright and do not flip with the direction the pair is read in. The
symbols are compared against the stable the alias asked for rather than the token's own symbol, because the
orderbook matches a pair on the exact string it was sent.

The first stable that answers wins. A pair no book trades fails its alias alone, leaving the others answered, and
`data` is null only when every alias failed — the ordinary shape for an unlisted token, which stays unpriced.

**One orderbook URL.** `HYPERFX_ORDERBOOK_URL` replaces `HYPERFX_WATCHLIST_URL` as the tracked environment
variable. It holds the orderbook's base URL, and `orderbookEndpoint` (`src/utils/orderbook.ts`) resolves both
`solvers` (the watchlist poll) and `graphql` (rates) against it, keeping a path prefix if there is one. A value
that already names an endpoint — the whole `.../solvers` URL the old variable held — is accepted as the host it is
on. **Deployments setting `HYPERFX_WATCHLIST_URL` must be updated, or solver watchlist discovery stops polling.**

**Caching and failures.** Answers are cached per symbol, with concurrent callers sharing one in-flight request, so
a block's orders cost one request and a repeatedly priced token costs at most one per minute. A rate or a
"no book quotes this" is reused for 60 s; an unreachable or misbehaving orderbook is retried after 5 s. Nothing
here throws: a token simply goes unpriced, its raw per-token volume is recorded either way, and a rate of zero or
less is refused rather than inverted into a nonsense price.

**Determinism.** A rate describes now, and the orderbook keeps no history to read at a past block, so a resync
prices a historical order at today's rate. This is why only the USD rollup depends on it; the raw per-token
amounts beside it are exact.
