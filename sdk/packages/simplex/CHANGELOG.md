# @hyperbridge/filler

## 0.8.0

### Minor Changes

- **Pair engine**: every market is a top-level `[[pairs]]` entry (cross-asset swaps and same-asset cross-chain transfers alike), priced by its own bid/ask curves in token1-per-token0. The `[[strategies]]` array and the stable strategy are removed — a config containing `[[strategies]]` fails at startup with a migration error.
- **Two independent profit gates**: `order.fees` must cover fill gas + the relayer fee, and every leg must be independently profitable in its own quote asset (margins in different quote assets are never summed). Venue-priced and one-sided legs are directional (fee gate only), guarded by the Uniswap price band.
- **USD-anchored confirmation sizing**: the operator's own curves are the price feed (USD stables at $1, curve mids as FX edges); every pair's token0 must be anchored or startup fails. `referenceOnly = true` pairs contribute their rate to the anchor graph without opening a market.
- **Startup validation**: crossed or zero-spread books, same-token asks at or above par, unknown symbols, unanchored assets, malformed `EVM-` chain keys (confirmation policies, per-chain watchOnly, `[assets]`), and uncovered chains are all startup errors; admin live curve edits enforce the same book invariants.
- **Hardened RPC quorum**: in-repo registry of public endpoints merged into a two-tier weighted quorum (operator BFT + two public witnesses); receipt-not-found counts as a valid no vote; reads resolve as soon as the quorum is met; same-chain orders skip the quorum entirely.
- **Unified asset registry**: symbols resolve from the SDK chain registry (`chain.ts`) under the user `[assets]` escape hatch — the simplex-local curated table is removed.


## 0.1.0

### Patch Changes

- Updated fee value
- Updated dependencies
    - @hyperbridge/sdk@1.3.22
