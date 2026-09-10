# Operator market list prices

`Operator` loads `/api/strategies`, whose `AdminStrategyDto` rows include the current bid and ask curve
points. `OperatorMarkets` selects the first valid configured price from each side and renders it with
the `token1/token0` unit. Venue-priced, reference-only, and sides without a valid curve render no
price value; clicking a row still opens the existing editor.
