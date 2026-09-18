# 2026-09-15 — Phantom bidding removed from simplex

Simplex no longer polls Hyperbridge for phantom orders or bids on them. The shared
`HyperbridgeScanner`, its `SimplexOptions.hyperbridgeScanner` option and the whole phantom branch of
`IntentFiller` are gone, along with `ContractInteractionService.preparePhantomBidUserOp` and
`quotePhantomFill` on the strategy interface. Operator prices will come from limit orders posted to
the HyperFX orderbook instead, which lands separately.

The real bid path is untouched. `prepareBidUserOp`, `BidManager.prepareSubmitBid`, the retraction
sweep, the bid store, the paymaster and the filler's own Hyperbridge connection all behave exactly
as before, and `Simplex.start` still builds or accepts an `orderScanner`.

`RuntimeState` is down to `paused`, since `phantomBids` was the only other key, and the filler no
longer takes a `StateStore`. `SqliteStateStore` still keeps one row per key, so the state store
tests carry a marker key of their own to exercise the per-key merge. `FillerConfig` loses
`uniswapV4PositionsByChain`, which only the phantom declaration read.

Files: `src/core/filler.ts`, `src/core/boot.ts`, `src/simplex.ts`, `src/index.ts`,
`src/scanner/types.ts`, `src/services/ContractInteractionService.ts`, `src/strategies/base.ts`,
`src/strategies/fx.ts`, `src/data/types.ts`, `src/data/state.ts`, `src/data/sqlite/state.ts`,
`sdk/packages/sdk/src/types/index.ts`, `filler-config-example.toml`, `package.json`, and the
matching tests.
