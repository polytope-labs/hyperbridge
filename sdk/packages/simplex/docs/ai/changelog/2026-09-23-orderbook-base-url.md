# 2026-09-23 — An orderbook base URL is completed to its GraphQL endpoint

A HyperFX deployment serves GraphQL under its base URL (`https://orderbook.hyperfx.finance/mainnet`
→ `.../mainnet/graphql`). An `[orderbook] url` set to the base alone failed every request with:

```
Orderbook returned HTTP 405 Method Not Allowed
```

The host redirects the base to its landing page, which refuses POST. Simplex posted to the URL as
given, so the failure hit the heartbeat interval read at boot, every reconciliation, and every
limit order, while the filler otherwise came up fine.

`OrderbookClient` now completes the endpoint with `graphqlEndpoint()`: a URL that does not already
end in `/graphql` gains it, trailing slashes and surrounding space are trimmed, and a URL carrying
a query string is left as the operator wrote it. `simplex init` still writes the full endpoint, so
this only matters for a hand-written or hand-copied URL.

An HTTP failure now names the endpoint it used (`Orderbook at <url> returned HTTP 405 ...`), so a
misconfigured URL is visible in the log rather than inferred.
