# 2026-09-07 — Wallet-funded phantom bids are no longer haircut; only pool-priced bids pay 10bps

The 5bps `PHANTOM_QUOTE_HAIRCUT_BPS` introduced on 2026-08-27 is removed, along with `applyPhantomQuoteHaircut` and both exports. `aggregatePhantomBids` now haircuts a bid only when its declaration names Uniswap V4 positions, by the unchanged `UNISWAP_QUOTE_HAIRCUT_BPS` of 10bps; a bid with no declared positions is published exactly as quoted. The haircut still lands on the individual quote before the zero-check, the weighted median, and the bidder rows.
Files: `src/protocols/intents/phantom-aggregation.ts`, `src/protocols/intents/index.ts`, `src/intents-helpers.ts`, `src/tests/phantomAggregation.test.ts`, `docs/ai/Flow.md`, `docs/ai/Decisions.md`.
