# 2026-09-12 — Cancellation is nonterminal (#1185)

The indexer records cancellation initiation as metadata without changing the parent row. The SDK exposes `CANCELLED` but keeps the stream open until `FILLED`, `REDEEMED`, or `REFUNDED`. When the chronologically last entry is cancellation, an existing terminal entry takes precedence. Other status ordering retains the existing behavior.

Rejected: terminating at cancellation, because a dispatched cancellation may still await source-chain processing; selecting cancellation solely by timestamp, because timestamps from independent chains and equal timestamps within a block do not establish settlement order.

The existing order query's `orders` response field does not match the parser's `orderPlaceds` field, and its legacy order shape differs from `IOrderV3`. This pre-existing query migration is outside the cancellation persistence fix. The stream regression tests inject the parser's expected shape and do not establish compatibility with a deployed GraphQL endpoint.
