# 2026-08-25 — Intent quotes default to directional indexed rates without fallback

Chosen: `quoteIntent` defaults to an `indexed_rates` strategy that selects the depth-weighted aggregate `LiquidityPool.buyRate` for base-to-quote orders and `sellRate` for quote-to-base orders. Source and destination chains resolve the configured token deployments; raw amounts are calculated from the indexer's 18-decimal whole-token pool rate and both tokens' configured decimals. A missing directional rate is an error.

Alternatives considered:

- **Keep defaulting to the legacy directional Phantom snapshot.** Rejected: those snapshots resolve through a canonical Base market and do not use the pair-centric pool rate, so quotes can disagree with the indexer's current market.
- **Quote directly from one source/destination pair of `PoolChainLiquidity` rows.** Rejected: those rows are inputs to the indexer's pool price. `LiquidityPool.buyRate` and `sellRate` are the maintained depth-weighted merge of fresh chain samples and are the intended market-level quote.
- **Silently fall back to Phantom or Uniswap when a rate is absent.** Rejected: an order would be priced from a different market than the caller requested, hiding stale or incomplete indexer coverage and producing another unfillable quote.
- **Remove the old strategies immediately.** Rejected for compatibility: callers that explicitly select them can continue doing so, while all calls without a strategy use the corrected path.

The result uses a new `indexed_rates` discriminant and includes the selected rate side, value, timestamp, pair symbols, chains, and protocol fee. This makes the price used to construct the order inspectable without exposing indexer internals.

The public sell rate is the reciprocal of the indexer's base-per-quote direction. That reciprocal rounds up at 18 decimals: rounding down would let a quote-token-to-base-token order request slightly more base output than the indexed direction supports. Buy rates are already indexer-floored outputs and remain unchanged.
