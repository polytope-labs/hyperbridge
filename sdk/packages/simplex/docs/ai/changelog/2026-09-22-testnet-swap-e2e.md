# 2026-09-22 — Testnet swaps end to end in CI

`e2e/run.mjs` runs three simplex solvers against the live HyperFX orderbook on BSC Chapel and
Polygon Amoy. Users place orders, and the SDK's `executeBest` executes the solvers' bids until each
order is filled on the destination chain. Settlement through Hyperbridge is not awaited. The
`simplex testnet swaps` workflow (`.github/workflows/test-simplex-e2e.yml`) runs it on pushes to
main, on pull requests that touch the strategy, core, orderbook or SDK intents code, and on demand.

## Scenarios

Each scenario gets a freshly posted book, so an earlier scenario's fills never starve a later one.
Every solver posts three standing orders (`standingOrders`); some scenarios add solver 1 levels
(`EXTRA_LEVELS`). A scenario passes when its order reaches `FILLED` in at least `minFills` fill
transactions.

| Scenario | Order |
| --- | --- |
| `same-chain` | 50 USDC → cNGN on BSC Chapel |
| `cross-chain` | 40 USDC on BSC Chapel → cNGN on Polygon Amoy |
| `partial` | 500000 cNGN on Polygon Amoy → USDC on BSC Chapel, too large for one solver |
| `multi-leg` | USDC → cNGN and cNGN → USDC legs, each reaching different solvers |
| `same-solver-levels` | one leg only solver 1's three levels reach |
| `multi-leg-levels` | both pairs, solver 1's levels only |
| `multi-leg-same-input` | two USDC → cNGN legs at the same levels |

`multi-leg` and `multi-leg-levels` put two pairs in one order. Once #1311 is deployed to the
testnet gateway, `placeOrder` rejects those, and both scenarios must be removed.

Pass scenario names as arguments, or through `E2E_SCENARIOS` (comma-separated), to run a subset.

## Wallets

Before the solvers start, any user short of what the run spends is topped up from the solver
holding the most of that token. A solver keeps back 400 USDC or 800000 cNGN for its own limit
orders. Users pay the solvers one token and are paid the other, so the wallets need only gas. The
run fails early if a top-up is impossible or a wallet has no gas.

CI runs never overlap (`concurrency: simplex-testnet-swaps`). The wallets must not also be in use
by another simplex instance.

## Variables

| Variable | Value |
| --- | --- |
| `E2E_BSC_TESTNET_RPC_URL` | BSC Chapel RPC; also the bundler unless `E2E_BSC_TESTNET_BUNDLER_URL` is set |
| `E2E_POLYGON_AMOY_RPC_URL` | Polygon Amoy RPC; also the bundler unless `E2E_POLYGON_AMOY_BUNDLER_URL` is set |
| `E2E_ORDERBOOK_URL` | HyperFX orderbook URL; `/graphql` is appended unless it already ends with it |
| `E2E_HYPERBRIDGE_WS_URL` | Hyperbridge (Gargantua) WebSocket endpoint |
| `E2E_SOLVER{1,2,3}_PRIVATE_KEY` | solver EVM keys, funded on both chains |
| `E2E_SOLVER{1,2,3}_SUBSTRATE_KEY` | solver Hyperbridge accounts (mnemonic or hex seed) holding BRIDGE for bid fees |
| `E2E_USER{1,2}_PRIVATE_KEY` | user EVM keys with gas on both chains |

Optional: `E2E_SCENARIO_TIMEOUT_MIN` (default 10) and `E2E_WORKDIR` (default a temp directory).
Solver logs are printed redacted when a scenario fails and are never uploaded, since they quote
endpoints.
