# Market setup and curve editing

`Wizard` initializes the strategies step with no transfer prefabs. `useStrategiesModel` seeds one
cross-asset draft on first entry, plus a USDT/CNGN draft when the selected catalog exposes CNGN and
USDT; `MarketRow` edits those drafts or a reference-only feed, and `assembleConfig` emits the selected
pair's bid/ask curves without same-token branching. The operator `CreateMarketForm` follows the same
cross-asset path and rejects identical resolved symbols before calling `POST /api/strategies`.

The operator and setup-wizard web market editors initialize the optional cap as blank, and their
assembly/API paths omit blank `maxOrderSize`, so new web markets are uncapped by default. Existing
configured caps round-trip unchanged. `CurveEditor` initializes every row created by its Add point
action with amount `1`; one-sided operator activation uses the same amount. Reference feeds remain
anchored at amount `0` because they are fixed-price feeds rather than order-size curves.

The setup market overview derives Buy and Sell summaries from enabled, non-reference draft curves.
Each side uses its first point with both an amount and price; incomplete sides remain hidden while the
user edits them. The order-cap input remains in `MarketRow`, but its summary is not rendered in the
overview row.
