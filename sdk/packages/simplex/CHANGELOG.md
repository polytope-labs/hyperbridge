# @hyperbridge/filler

## 0.12.1

### Patch Changes

- MPCVault signing failures now name the failing RPC, the signing-request uuid and MPCVault's x-request-id, and the error guard also trips on code-only errors (MPCVault sends empty messages with only a code set). `executeSigningRequests` retries INVALID_ARGUMENT/NOT_FOUND twice with short backoff: MPCVault intermittently rejects a uuid its own createSigningRequest just returned, which failed live fills with `3 INVALID_ARGUMENT: Invalid uuid` before the callback co-signer was contacted.

## 0.12.0

### Minor Changes

- BREAKING (internal constant): `POST_OP_GAS_LIMIT` is replaced by `POST_OP_GAS_LIMIT_SIMPLEX` (40,000) and `POST_OP_GAS_LIMIT_CIRCLE` (100,000). The single shared constant was pinning the Simplex paymaster's postOp limit at 100,000 while its postOp needs roughly 8-12k; the EntryPoint penalises the unused remainder without billing the user for it, so the paymaster was absorbing that penalty on every sponsored operation. 40,000 is the largest value the EntryPoint leaves penalty-free regardless of postOp cost. Measured on a mainnet fork, per-operation margin rises about 2.4×. The contract ceiling stays at 100,000 so an in-place proxy upgrade never rejects clients still sending the old limit; the client simply sends the lower value.
- SimplexPaymaster PERMIT2 mode. On chains whose fee token has no EIP-2612 permit (BSC pegged USDC/USDT), Simplex no longer keeps a capped allowance to the paymaster that it tops up with native-funded approvals. The bootstrap is now a single funded `approve(Permit2, max)` per token; every operation after that carries a per-op, single-use, deadline-bounded Permit2 signature naming the paymaster as spender, so nothing is exposed to the paymaster at rest and native gas is never needed again. Existing solvers migrate on their own: their current paymaster allowance is used until it drains, then the Permit2 approval replaces the refill. Mode 2 is only used against paymaster deployments that support it; older deployments keep the previous behaviour.

## 0.9.2

### Patch Changes

- SDK helpers now take their endpoint from the operator's configured `rpcUrls` instead of `publicClient.transport.url`. A chain with more than one RPC is served by a viem `fallback` transport, whose `url` is `undefined`; passing that through left the SDK's `EvmChain` with no RPC, so viem silently substituted the chain's built-in public default (`polygon.drpc.org` on Polygon, `eth.merkle.io` on Ethereum) and every `IntentGateway` call — phantom bids included — bypassed the configured endpoints. Quorum configs were affected on every chain with two or more URLs. An unconfigured chain now throws instead of degrading to a public endpoint.
- Block-scanner poll period is configurable via `simplex.blockScanIntervalSeconds`, and the default drops from the previously hardcoded 1 second to **3 seconds**. Each tick costs one `eth_blockNumber` plus one `eth_getLogs` per chain per endpoint, so this takes a chain from ~172k to ~58k requests per endpoint per day — the difference between exhausting a free RPC's quota and fitting inside it. The cost is seeing new orders up to 3 seconds later, which matters in a contested market: set `blockScanIntervalSeconds = 1` to restore the old cadence. Fractional values are allowed (0.5 polls twice a second) with a 0.1 minimum; zero, negative and non-numeric values are rejected at config-parse time rather than being coerced by `setInterval` into a spin loop.
- Error logging no longer dumps the contract ABI. viem hangs the full ABI off its errors and repeats the message once per wrapper in the cause chain, so a single failed `readContract` printed roughly 1,800 lines; the log serialiser now keeps the type, short message, details, failing endpoint and root cause, and trims the stack to six frames — the same failure prints 10 lines. The endpoint and root cause are newly surfaced as their own fields.

## 0.9.0

### Minor Changes

- BREAKING: order reconstruction is pure log decoding. `OrderPlaced` logs now carry the full order (call payloads and graffiti), so the `debug_traceTransaction` calldata-recovery path and the per-transaction occurrence pairing are removed — order detection needs nothing beyond `eth_getLogs`. The filler only understands the new event schema: it must run against upgraded IntentGatewayV2 deployments, sees nothing from pre-upgrade gateways (different event topic), and logs an error for any decoded log missing its call payloads. Upgrade the filler in lockstep with the gateway deployment.
- `reconstructOrdersFromLogs` is now synchronous and `ReconstructDeps` reduces to `{ onError }` — the `getPlaceOrderCalldata` dependency is gone.
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
