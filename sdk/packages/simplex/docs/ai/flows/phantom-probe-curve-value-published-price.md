# Phantom probe: curve value -> published price

Verified 2026-08-19 by reading the path end to end and reconciling against live mainnet bids and
the nexus indexer.

```
FXFiller.quotePhantomFill(order)                      src/strategies/fx.ts
  canFill(order)                                      bail if halted / unsupported / one-sided
  resolveOrderLegs(order)                             order legs -> ResolvedLeg[]
  sizeOrder(order, legs, venuePriceMemo())            per-leg notionals ONLY here
  for each leg:
    resolveLegRates(..., legNotionals[i], ...) -> rate    curve sampled at THIS leg's size
    computeLegPolicyOutput(input.amount, ..., null, rate) <-- precision collapses HERE
  returns TokenInfo[] (token, amount)
```

Two things to keep straight about this path:

- **`sizeOrder`'s exposure outputs are unused here.** `cappedByPair` and `capFractionByPair` ration
  real fills; a probe commits no capital, so it passes a `null` budget and prices the whole input.
  Only `legNotionals` is consumed, as the rate sample point. See Decisions.md.
- **`computeLegPolicyOutput` is where an arbitrary-precision `Decimal` becomes the integer that
  leaves the process**, floored. Nothing downstream can recover the discarded fraction — the
  filler's `Decimal` rate is never transmitted. The floor is deliberate and load-bearing.

The integer then travels unchanged:

```
outputs[i].amount                   e.g. 715
  -> fillOrder calldata outputs[i]  uint256, covered by userOpHash
  -> paymasterAndData               declaration: accepted sources = every configured chain
                                    (acceptedSourceChainsFor), plus declared V4 positions
  -> bid submitted to the coprocessor
  -> aggregatePhantomBids           quotes.push({ price, weight })
  -> weightedMedian(backedQuotes)   SELECTION — returns an input element verbatim
  -> PhantomOrderPriceSnapshotV2    medianPrice = lowestPrice = highestPrice
  -> indexer updateLiquidityPools   renormalized by the leg's own standardAmount
```

A quote's weight in that median is the solver's balance of **that leg's output token on the
destination chain** — so a solver holding over half the leg's weight sets the published price
verbatim, and inventory in the wrong token buys no influence on that leg.

## Precision budget

The output integer _is_ the price, to whatever resolution the output token's decimals allow. One
whole cNGN priced into 6-decimal USDC quotes ~715 base units, so the grid is `1/715` = 0.14%.
Chains whose output token has 18 decimals carry full precision on the same leg — which is why
EVM-56 publishes `716845878136200` where Base publishes a bare `715`. The lever is the pallet's
standard amount, not the rounding mode; see Decisions.md.
