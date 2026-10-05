# Testnet cancels pay the relayer fee in the fee token, and SDK CI runs live testnet tests in their own job

## Cancel quote on testnet

`OrderCanceller.quoteCancelOrder`, which `IntentGateway.quoteCancelOrder(order, options)` exposes,
quotes a cross-chain cancel on the chain that receives the cancel transaction: the source chain for
the default source route, the destination chain for `{ from: "destination" }`. When that chain is in
`TESTNET_CHAINS`, the quote is `{ nativeValue: 0n, relayerFee }`, and the
`AWAITING_CANCEL_TRANSACTION` event from `cancelOrder` carries `value: 0n`. With no `msg.value` the
gateway pays the dispatch through `dispatchWithFeeToken`, pulling `relayerFee` of that host's fee
token from the sender, so the caller approves that fee token to that chain's IntentGateway before
submitting. Testnet hosts have no Uniswap router to swap native value into the fee token.

Mainnet quotes still carry the native dispatch fee, and same-chain orders still quote `0n` for both
values. The source-route quote builds its GET with `from` set to the source chain's IntentGateway,
matching the GET that gateway dispatches. Released in `@hyperbridge/sdk` 2.8.24.

## Live cancellation test

`test:intent-gateway-cancel` runs `src/tests/sequential/intentGatewayCancel.test.ts`. It places a
BSC Chapel (`EVM-97`) to Polygon Amoy (`EVM-80002`) order whose output no solver can fill, then
cancels it from Amoy with `{ from: "destination" }`. It asserts that:

- the cancel transaction targets the Amoy gateway with `value: 0n`, a nonzero `relayerFee` and
  height 0, and emits `OrderCancelled` and a `PostRequestEvent`;
- the POST is a `RefundEscrow` returning the escrowed input to `order.user`;
- Amoy freezes the order, with `_filled[commitment]` set to the canceller;
- Hyperbridge holds the refund POST, seen either as `HYPERBRIDGE_DELIVERED` on the status stream or
  through Hyperbridge's request receipt, while Chapel still holds the escrow;
- within 5 minutes of that receipt, `queryPostRequest` on the indexer returns the refund POST with
  `HYPERBRIDGE_DELIVERED`, `HYPERBRIDGE_FINALIZED` or `DESTINATION`, so a broken indexer fails the
  test.

The default run ends there. With `CANCEL_FULL_REFUND=true` (or `1`) the test waits for
`HYPERBRIDGE_FINALIZED`, self-delivers that calldata to the Chapel host's handler, since the testnet
relayer does not deliver to EVM chains, and asserts `CANCELLATION_COMPLETE` and `isOrderRefunded`.
That path can take over an hour, because Hyperbridge consensus reaches testnet EVM hosts about
hourly.

It needs `PRIVATE_KEY`, `BSC_CHAPEL`, `POLYGON_AMOY` and `HYPERBRIDGE_GARGANTUA`. The test
approves the Chapel input token and both chains' fee tokens to their gateways itself.

The cancel, HFT and GET tests read the indexer at `GARGANTUA_INDEXER_URL`. The `live` job sets it
to the local indexer at `http://localhost:3100`; when it is unset, they use the hosted
`https://gargantua.indexer.polytope.technology`.

## `.github/workflows/test-sdk.yml`

The `test` job builds, then runs SDK `typecheck`, SDK `test:intents-coprocessor`, simplex `lint`,
`test:data`, `test:unit` and `test:filler`, and SDK `test:concurrent`. It starts no local indexer.
`vitest.config.ts` does not set `dangerouslyIgnoreUnhandledErrors`, so `test:concurrent` fails on an
unhandled error; only the sequential live scripts pass that flag. Simplex `test:unit` includes
`src/tests/pairs.test.ts`, and `test:filler` names `fx.payout.test.ts` and
`quorum-public-client.test.ts`.

The `live` job runs after `test` in the single concurrency group `sdk-testnet-live` with
`cancel-in-progress: false`. Every run spends testnet funds from the shared `PRIVATE_KEY` wallet, so
runs from all branches never overlap and a running job is never cancelled. GitHub keeps only one
pending job per group, so a newer pending `live` run replaces an older pending one, which shows as
cancelled. It skips draft PRs and PRs from forks.

Before the tests, the `live` job starts a local indexer. It installs `docker-compose`, builds the
SDK with `build:node`, since the indexer imports `@hyperbridge/sdk/intents-helpers`, logs in to
Docker Hub and runs `pnpm start:local` in `packages/indexer`. That command's `ENV=local` codegen
starts Hyperbridge Gargantua, BSC Chapel, Polygon Amoy and Base Sepolia at their current heads,
read through the RPCs in `sdk/.env.local`. In `docker/docker-compose.local.yml`, `graphql-engine`
waits for postgres and the Gargantua indexer node to be healthy. A readiness step then waits for
GraphQL on port 3100, for a `stateMachineUpdateEvents` query to succeed (restarting
`graphql-engine` once if it does not, since the query service reads entity tables only at startup),
and for all four chains to be within 300 blocks of their heads. The job always prints the indexer
logs and runs `docker compose down -v` at the end.

Its test steps run in order, only once the readiness step succeeds, and each runs even when an
earlier test fails:

- `test:intent-gateway-cancel`, with `CANCEL_FULL_REFUND` set from the `cancel_full_refund`
  workflow_dispatch input;
- `test:hyper-fungible-token`, which, like the cancel test, waits up to 5 minutes after
  Hyperbridge's request receipt for `queryPostRequest` to return the request with a delivered
  status;
- `test:get-request` (`getRequestBscAmoy.test.ts`), on `BSC_CHAPEL` and `POLYGON_AMOY`. It fails
  at once, naming each of `PRIVATE_KEY`, `BSC_CHAPEL`, `POLYGON_AMOY` and `HYPERBRIDGE_GARGANTUA`
  that is missing.

This workflow runs no IntentGateway place-and-fill test, and simplex `fx.testnet.test.ts` runs in no
workflow. The `simplex testnet swaps` workflow (`.github/workflows/test-simplex-e2e.yml`) places and
fills orders on Chapel and Amoy.
