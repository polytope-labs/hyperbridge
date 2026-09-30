# 2026-09-15 — Price curves removed

Prices come from the operator's limit orders, so the curves they replaced are gone everywhere: from
`[[pairs]]`, from the engine, from the admin API, and from both the CLI wizard and the operator UI.

## What a market is now

A `[[pairs]]` entry is two symbols and nothing else:

```toml
[[pairs]]
token0 = "USDC"
token1 = "CNGN"
```

`bidPriceCurve`, `askPriceCurve`, `maxOrderSize` and `referenceOnly` are all gone with
`FillerPricePolicy`. The per-order cap is the matched limit order's own `remaining`, one-sidedness is
which side the operator posted, and the same-asset rules (ask-only, priced below par) are expressed
by the order rather than checked against a curve.

`interpolated-curve.ts` keeps its confirmation half. `ConfirmationPolicy` and `InterpolatedCurve`
never had anything to do with prices; the price half of the file, including `bookCrossedAt`, went.

Reference-only pairs went with the USD anchor graph they fed. Confirmation depth is still sized in
dollars, but the rate now comes from the limit orders, so a pair carries no rate to anchor with and
`unanchoredToken0Symbols` had nothing left to compute.

## What the operator can still do

`GET /api/strategies` lists the markets and `POST`/`DELETE` open and close them. The curve editor
(`PUT /api/strategies/:index/curves`) and the cap editor (`PUT /api/strategies/:index` and
`DELETE /api/strategies/:index/max-order-size`) are gone, along with `PairController.setCurve`,
`clearCurve`, `setMaxOrderSize` and `clearMaxOrderSize`. Prices are set by posting limit orders.

The operator UI collects which markets to trade and no longer asks how they are priced. The
`simplex init` wizard no longer asks at all: its markets step is gone, it writes no `[[pairs]]`, and
an update run carries through whatever a config already had. Declaring a market up front cannot
express a limit order, which names two amounts and a fill chain and is created while the filler runs.
A config with no `[[pairs]]` is a filler waiting for its first market rather than an error.

Two things a market used to carry are now gaps rather than replacements. The solver link built a
shareable swap page out of a market's curve prices and has been removed rather than left refusing
every market; rebuilding it on a limit order's rate is straightforward and worth doing. And a
`[[pairs]]` entry no longer gates matching at all, since the matcher reads the limit orders alone, so
adding or removing a market does not change what the filler fills.
