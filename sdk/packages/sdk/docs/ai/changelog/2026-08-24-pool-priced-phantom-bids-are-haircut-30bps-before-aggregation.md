# 2026-08-24 — Pool-priced phantom bids are haircut 30bps before aggregation

A phantom bid that declares Uniswap V4 positions is quoting off those pools, and a pool price is what a trade gets before the pool takes its fee — so the amount such a bid names is richer than what the solver clears once the sourcing swap goes through. `aggregatePhantomBids` now nets 30bps out of every leg amount on a bid whose declaration carries positions, before the quote reaches the zero-check, the weighted median, or the bidder rows. Non-declaring bids are untouched: wallet inventory has already paid its cost of goods.

The haircut runs off the declaration rather than the positions that survive the on-chain ownership check, so a quote is priced on the same basis the solver priced it on — including on a chain with no V4 contracts configured, where declared positions contribute no weight. A leg whose amount the haircut rounds to zero falls into the existing declined-leg path.

`UNISWAP_QUOTE_HAIRCUT_BPS` and `applyUniswapQuoteHaircut` are exported from both `@/protocols/intents` and the `intents-helpers` sub-path, so the indexer and simplex read the same number instead of restating it.

Files: `src/protocols/intents/phantom-aggregation.ts`, `src/protocols/intents/index.ts`, `src/intents-helpers.ts`, `src/tests/phantomAggregation.test.ts`.
