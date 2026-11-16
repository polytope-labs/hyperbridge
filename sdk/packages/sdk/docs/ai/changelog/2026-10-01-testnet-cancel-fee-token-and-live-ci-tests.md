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

## Live cancellation tests

`test:intent-gateway-cancel` runs `src/tests/sequential/intentGatewayCancel.test.ts`, whose two tests
share one wallet and run in order. Each places an order whose output no solver can fill (0.01 USDC
for 1,000,000 USDC) and runs its cancellation to a full refund on the order's source chain. The
testnet relayer delivers Hyperbridge consensus updates to the EVM hosts but not messages, so each
test sends the `HYPERBRIDGE_FINALIZED` calldata to the source host's handler itself. When the
indexer records no Hyperbridge update on the host at the height the message needs, that calldata
batches the Hyperbridge consensus proofs the host is missing with the message, one per
authority-set rotation it lacks plus one covering that height, so the handler accepts them all in
one transaction. The test sends it with the gas estimate plus 2,000,000, capped at 16,777,216,
Chapel's per-transaction gas cap: the host swallows a failed app callback, so at the estimated limit
the callback can run out of gas under the 63/64 rule while the transaction still succeeds.

Cancel from source places a Polygon Amoy (`EVM-80002`) to BSC Chapel (`EVM-97`) order with a
deadline 40 Chapel blocks out and calls `cancelOrder(order, ismpClient, { from: "source" })`. The
SDK waits for a Chapel height past the deadline and proves the order's leg 0 `_partialFills` slot
there. The test sends the cancel on Amoy, and the SDK self-delivers the resulting GET to Hyperbridge.
The test asserts that:

- the cancel transaction targets the Amoy gateway with `value: 0n`, a nonzero `relayerFee` and the
  proof height, and emits `OrderCancelled` and a `GetRequestEvent` from the Amoy gateway for that
  Chapel slot at that height;
- Hyperbridge holds a response receipt for the GET while Amoy still holds the escrow;
- delivering the calldata, which carries the GetResponse, to the Amoy handler emits
  `GetRequestHandled` and `EscrowRefunded`, its refund `Transfer`s to `order.user` match the
  placement's transfers to the gateway per token, the leg escrow reads 0 and `isOrderRefunded` is
  true;
- within 5 minutes, the indexer records the GET as `HYPERBRIDGE_DELIVERED` and `DESTINATION` and
  holds its response.

Cancel from destination places a Chapel to Amoy order and cancels it from Amoy with
`{ from: "destination" }`. It asserts that:

- the cancel transaction targets the Amoy gateway with `value: 0n`, a nonzero `relayerFee` and
  height 0, and emits `OrderCancelled` and a `PostRequestEvent`;
- the POST is a `RefundEscrow` returning the escrowed input to `order.user`;
- Amoy freezes the order, with `_filled[commitment]` set to the canceller;
- Hyperbridge holds the refund POST, seen either as `HYPERBRIDGE_DELIVERED` on the status stream or
  through Hyperbridge's request receipt, while Chapel still holds the escrow;
- delivering the calldata to the Chapel handler emits `EscrowRefunded` with refund `Transfer`s that
  match the placement per token, the stream reaches `CANCELLATION_COMPLETE` with that delivery
  within 5 minutes of it and `isOrderRefunded` is true.

The cancel stream follows each request's status through the indexer. From source, the stream ends
at `HYPERBRIDGE_FINALIZED`, before the test delivers the GET response, so the explicit indexer check
is what fails when the indexer misses that delivery. From destination, every stream event after
`CANCEL_STARTED` comes from the indexer, so an indexer miss fails as a timeout on the next event:
`HYPERBRIDGE_FINALIZED` when it misses the Hyperbridge delivery, `CANCELLATION_COMPLETE` when it
misses the Chapel delivery.

Each test's timeout is the sum of its step budgets: 90 minutes from source (setup 10, Chapel proof
25, cancel 5, `HYPERBRIDGE_FINALIZED` 40, delivery 5, indexer 5) and 75 minutes from destination
(setup 10, Hyperbridge delivery 15, `HYPERBRIDGE_FINALIZED` 40, delivery 5,
`CANCELLATION_COMPLETE` 5).

Both need `PRIVATE_KEY`, `BSC_CHAPEL`, `POLYGON_AMOY` and `HYPERBRIDGE_GARGANTUA`. Each test
approves its order's input token and the fee tokens it pays to their gateways itself.

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

The job timeout is 300 minutes, covering up to 45 minutes of indexer setup and readiness plus each
test step's own timeout. Its test steps run in order, only once the readiness step succeeds, and
each runs even when an earlier test fails:

- `test:intent-gateway-cancel`, both cancel tests, with a 170-minute timeout;
- `test:hyper-fungible-token`, with a 25-minute timeout, which waits up to 5 minutes after
  Hyperbridge's request receipt for `queryPostRequest` to return the request with a delivered
  status;
- `test:get-request` (`getRequestBscAmoy.test.ts`), with a 45-minute timeout, on `BSC_CHAPEL` and
  `POLYGON_AMOY`. It fails at once, naming each of `PRIVATE_KEY`, `BSC_CHAPEL`, `POLYGON_AMOY` and
  `HYPERBRIDGE_GARGANTUA` that is missing.

This workflow runs no IntentGateway place-and-fill test, and simplex `fx.testnet.test.ts` runs in no
workflow. The `simplex testnet swaps` workflow (`.github/workflows/test-simplex-e2e.yml`) places and
fills orders on Chapel and Amoy.
