# Flow

AI-maintained map of how code paths in `sdk/packages/simplex` actually execute. Only flows that
have been read and verified are documented; coverage grows as areas of the package are touched.

## Phantom probe: curve value -> published price

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
  -> bid submitted to the coprocessor
  -> aggregatePhantomBids           quotes.push({ price, weight })
  -> weightedMedian(backedQuotes)   SELECTION — returns an input element verbatim
  -> PhantomOrderPriceSnapshotV2    medianPrice = lowestPrice = highestPrice
  -> indexer updateLiquidityPools   renormalized by the leg's own standardAmount
```

A quote's weight in that median is the solver's balance of **that leg's output token on the
destination chain** — so a solver holding over half the leg's weight sets the published price
verbatim, and inventory in the wrong token buys no influence on that leg.

### Precision budget

The output integer *is* the price, to whatever resolution the output token's decimals allow. One
whole cNGN priced into 6-decimal USDC quotes ~715 base units, so the grid is `1/715` = 0.14%.
Chains whose output token has 18 decimals carry full precision on the same leg — which is why
EVM-56 publishes `716845878136200` where Base publishes a bare `715`. The lever is the pallet's
standard amount, not the rounding mode; see Decisions.md.

## Venue pricing (Uniswap V4 funded pairs)

Verified 2026-08-19.

```
resolveLegRates(...)
  curveless pair && token0 is a USD stable
    -> venuePriceMemo() -> getVenueUsdPrice(chain, token1)
         -> UniswapV4FundingPlanner.getExoticTokenPrice
              picks the position with the largest pool liquidity
              -> computeDirectPoolPriceUsd -> sdkPool.token0Price / token1Price
    -> checkPriceGuard(...)   reject if outside maxDeviationBps of the static reference
    -> rate = 1 / venueUsd
  otherwise -> the pair's ask/bid curve at the leg's notional
```

`computeDirectPoolPriceUsd` returns the **raw pool mid** derived from `sqrtPriceX96`. The pool's
fee tier is read and stored on the hydrated position (`pos.fee`) but never applied to the price,
and there is no size or impact term — `computeLegPolicyOutput` extends the mid linearly across the
whole priced quantity. `checkPriceGuard` is the only defense on this path, and it checks deviation
from a static reference, not execution cost. A venue-priced pair that has to swap through its own
pool to source inventory pays a fee tier it never quoted against.
