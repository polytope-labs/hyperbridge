# 2026-09-15 — Uniswap V4 funding removed

Simplex no longer sources liquidity from Uniswap V4 LP positions, and no longer prices a market from
a pool. `UniswapV4FundingPlanner`, `UniswapV4LiquidityState`, the V4 ABI and the `[vault.uniswapV4]`
config block are gone, along with the wizard's pricing-method step and its position editors.

The ERC-4626 treasury vault is now the only funding venue, so `FundingVenue` loses
`getExoticTokenPrice`, which only a pool could answer.

## What this means for a market

Every `[[pairs]]` entry needs its own bid and/or ask price curve again. A curve-less pair used to be
legal when V4 positions were configured, with the pool mid standing in as the price; there is no
pool to stand in now, so `validatePairConfigs` requires a curve on every pair and `FXFiller` refuses
a pair set that has none.

Two behaviours that only existed under pool pricing go with it. `[vault.uniswapV4].side`, which made
an LP one-sided, has no equivalent: a curve-priced pair goes one-sided by omitting a curve, which is
how it already worked. The per-position price guard is gone too, since it existed to catch a
manipulated, stale or thin pool.

A config carrying `[vault.uniswapV4]` still loads: `migrateLegacyConfig` reports the block as
dropped rather than carrying it into a config that could no longer act on it.
