# @hyperbridge/filler

## Unreleased

### Minor Changes

- Local web UI: browser setup wizard (`simplex` with no config) that writes the config and starts the filler in-process, plus an operator dashboard (status, pause/resume, graceful stop, balances, inflight price-curve edits persisted to the config file, overfill self-halt reset, live activity feed over SSE, manual vault sweep/redeem, runtime allowlist and log-level changes, rebalancing trigger view)
- Web wizard supports MPCVault/Turnkey signers and Uniswap V4 pool pricing
- Terminal setup wizard via `simplex init`
- `run` is the default command and `-c` is optional; configs are discovered at `./filler-config.toml` or `$SIMPLEX_HOME/config.toml`
- BREAKING: `--admin-port` is replaced by `--ui <[host:]port>` / `--no-ui`; the UI (same port 8686, same curve API) is now on by default on loopback, and mutating API requests require the `X-Simplex-UI: 1` header
- Markets can be added and removed from the operator dashboard at runtime — including custom-token markets — validated against the full config (duplicates, USD anchoring, symbol resolution) and persisted; removals never touch vault funds
- `pause()`/`resume()` on the filler, persisted across restarts
- Fixed: one-sided bid-only hyperfx configs crashed config validation

## 0.8.2

### Patch Changes

- The public RPC registry is removed entirely: the quorum is now exactly the operator's configured `rpcUrls` on every chain, with a flat BFT threshold (`floor(2N/3) + 1`) — no more two-tier operator/witness split. What 0.8.1 did for BSC (whose free endpoints structurally cannot serve `eth_getLogs`) applies everywhere: public endpoints proved to be a liveness liability (rate limits, response-size caps, archive gaps) without adding integrity beyond what independent operator endpoints already provide. Configure at least two organisationally independent providers per source chain; four or more to tolerate a faulty one. `PUBLIC_RPC_URLS`/`getPublicRpcUrls` exports and `FillerConfigService.getQuorumRpcUrls` are gone; `QuorumPublicClient` drops its `operatorCount` constructor parameter.

## 0.8.1

### Patch Changes

- BSC is removed from the public RPC registry and runs an operator-only quorum: its free endpoints structurally cannot serve `eth_getLogs` (method caps, archive-token requirements, plan quotas), so the two-witness floor was rejecting reads the operator's endpoint had answered correctly and the BSC block scanner deadlocked on `QuorumError`. Ethereum, Arbitrum, Base and Polygon keep their public witnesses.

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
