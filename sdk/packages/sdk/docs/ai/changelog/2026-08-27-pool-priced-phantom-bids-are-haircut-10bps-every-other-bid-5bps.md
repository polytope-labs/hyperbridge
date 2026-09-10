# 2026-08-27 — Pool-priced phantom bids are haircut 10bps, every other bid 5bps

The pool haircut added on 2026-08-24 drops from 30bps to 10bps, and bids that declare no Uniswap V4 positions — previously untouched — now pay a 5bps haircut of their own. `aggregatePhantomBids` picks one of the two per bid from the same signed declaration it already read: `applyUniswapQuoteHaircut` when positions are named, `applyPhantomQuoteHaircut` otherwise. They never stack. Both still land on the individual quote before the zero-check, the weighted median, and the bidder rows, so the declined-leg path still catches a leg amount a haircut rounds to zero.

`PHANTOM_QUOTE_HAIRCUT_BPS` and `applyPhantomQuoteHaircut` are exported alongside the Uniswap pair from `@/protocols/intents` and the `intents-helpers` sub-path, so the indexer and simplex keep reading both numbers from one place.

Files: `src/protocols/intents/phantom-aggregation.ts`, `src/protocols/intents/index.ts`, `src/intents-helpers.ts`, `src/tests/phantomAggregation.test.ts`.
