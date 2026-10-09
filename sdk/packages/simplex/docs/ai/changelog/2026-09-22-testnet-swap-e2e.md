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
transactions and no limit order posted for it has paid out more than its size: after every
scenario, `spent` is read from the posting solver's account for each posted limit order and must
not exceed the order's size. A tally that cannot be read is logged and does not fail the scenario.

| Scenario | Order |
| --- | --- |
| `same-chain` | 0.5 USDC → cNGN on BSC Chapel |
| `cross-chain` | 0.4 USDC on BSC Chapel → cNGN on Polygon Amoy |
| `partial` | 20000 cNGN on Polygon Amoy → USDC on BSC Chapel, too large for one solver |
| `multi-leg` | USDC → cNGN and cNGN → USDC legs, each reaching different solvers |
| `same-solver-levels` | 12 USDC, more than solver 1's best level takes |
| `multi-leg-levels` | both pairs, solver 1's levels only |
| `multi-leg-same-input` | two 5.5 USDC → cNGN legs against a level that takes 10 |
| `onchain-limit` | 0.5 USDC for cNGN on BSC Chapel, then the checks of the on-chain limit |
| `onchain-limit-refusal` | the same swap, with the best bid's limit order left one unit short of what it pays |

`multi-leg` and `multi-leg-levels` put two pairs in one order, which #1311 forbids (`mixedPairs`).
They run only when named, for a gateway without that rule.

`onchain-limit` (`check: "limit"`) checks the limit a solver's account puts on a limit order's
size (`e2e/budget.mjs`). Once its order is filled, the limit orders on the fill chain must show:

- `spent` above zero on at least one of them, and in total no less than the order's minimum output
- the same figure in the solver's own record: `size - remaining` from `GET /api/limit-orders/<id>`
  equals `spent`, polled for up to 90 seconds while the solver settles the fill
- a `debitOrder` that lands on the order's size is accepted, and one a unit past it is refused
  with `LimitOrderExceeded`. Both are simulated with `eth_call` from the account to itself at one
  block, and nothing is sent.

`onchain-limit-refusal` (`check: "refusal"`) has the limit refuse a real fill. When the bids
arrive, and before the SDK picks one, the swap worker (`e2e/refusal.mjs`) takes the bid paying the
most. Its solver's key sends a `debitOrder` from the account to itself that uses up the bid's limit
order until one unit less than the bid pays is left. The bid is then simulated as the SDK does, and
sent to the bundler anyway. The bid must take the whole order with no fee, so that its payout is
exactly what it approved. The runner then reads from the chain that:

- the SDK's own simulation refused the bid
- the operation is in a bundle, failed, and logged `LimitOrderExceeded` at the tally plus what the
  bid pays, and its execution moved no token out of the solver's account
- the tally is still where the `debitOrder` left it, and the solver filled nothing on the order
- the order was filled by another solver, whose own record matches its tally as in `onchain-limit`
- within 150 seconds, the solver has lowered the limit order to the room left on chain, or closed
  it. The solvers run with `reconcileIntervalSecs = 30`, so the correction lands within the scenario.

Both scenarios fail whatever their outcome when a solver's account on the fill chain is not
delegated to an implementation with `spent`, and the reason names what the account delegates to.

Pass scenario names as arguments, or through `E2E_SCENARIOS` (comma-separated, spaces allowed),
to run a subset. Without names, every scenario except the `mixedPairs` ones runs.

The orderbook refuses a limit order paying out under 10 USDC or 15000 cNGN
(`serverInfo.minOrderSizes`), so every limit order is posted at that floor: 10 USDC in for 15800,
15780 or 15600 cNGN out, and 16000 to 16200 cNGN in for 10 USDC out.

Swap sizes are the scenarios' own business, and most take a fraction of one limit order: 0.5 USDC,
0.4 USDC, 2000 cNGN. Only the ladder scenarios are larger, because reaching a second level means
swapping more than the first level's 10 USDC. A default run spends 24.4 USDC on BSC Chapel and
20000 cNGN on Polygon Amoy.

## Wallets

Users pay the solvers one token and are paid the other, so between them the wallets always hold
what a run needs, and only gas is consumed. Funds are moved back in both directions:

- **Before the run:** any user short of what its scenarios spend is topped up from the solver
  holding the most of that token. The solver keeps back what its largest book pays out.
- **Before each scenario's book is posted:** any solver that can't back its limit orders is topped
  up from the user holding the most of that token. The user keeps back what the run spends.

A top-up adds half again over the shortfall. The run fails early if no wallet can spare a top-up,
or if a wallet has no gas.

CI runs never overlap (`concurrency: simplex-testnet-swaps`), and a started run is never
cancelled. GitHub keeps one pending run per group, so a newer run replaces a pending one. Draft
pull requests and pull requests from forks are skipped. The wallets must not also be in use
by another simplex instance.

## Variables

| Variable | Value |
| --- | --- |
| `E2E_BSC_TESTNET_RPC_URL` | BSC Chapel RPC |
| `E2E_POLYGON_AMOY_RPC_URL` | Polygon Amoy RPC |
| `E2E_BSC_TESTNET_BUNDLER_URL` | BSC Chapel ERC-4337 bundler serving the chain's EntryPoint; CI uses Pimlico (`https://api.pimlico.io/v2/97/rpc?apikey=...`) |
| `E2E_POLYGON_AMOY_BUNDLER_URL` | Polygon Amoy ERC-4337 bundler serving the chain's EntryPoint; CI uses Pimlico (`https://api.pimlico.io/v2/80002/rpc?apikey=...`) |
| `E2E_ORDERBOOK_URL` | HyperFX orderbook URL; `/graphql` is appended unless it already ends with it |
| `E2E_HYPERBRIDGE_WS_URL` | Hyperbridge (Gargantua) WebSocket endpoint |
| `E2E_SOLVER{1,2,3}_PRIVATE_KEY` | solver EVM keys, funded on both chains |
| `E2E_SOLVER{1,2,3}_SUBSTRATE_KEY` | solver Hyperbridge accounts (mnemonic or hex seed) holding BRIDGE for bid fees |
| `E2E_USER{1,2}_PRIVATE_KEY` | user EVM keys with gas on both chains |
| `SECRET_PHRASE` | optional: the Hyperbridge account solvers are topped up with BRIDGE from |

Optional: `E2E_SCENARIO_TIMEOUT_MIN` (default 10) and `E2E_WORKDIR` (default a temp directory).
Output is redacted against every `E2E_*` value, both as given and as completed (the orderbook URL
with `/graphql`). Solver logs are printed, redacted, when a scenario fails, and are never uploaded,
since they quote endpoints.
