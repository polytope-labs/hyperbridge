# 2026-11-11 — Mainnet orders are screened before any bid

Simplex screens every mainnet order through the HyperFX orderbook's `screenOrder` mutation
(polytope-labs/hyperfx-orderbook#76) before it bids. The orderbook checks the order's user and
beneficiary against Chainalysis and keeps one verdict per commitment. Simplex drops an order the
verdict flags, and drops one it cannot get a verdict for. Nothing on mainnet is filled unscreened.

## Which orders

`requiresScreening(order)` (`src/orderbook/screening.ts`) is true unless both `order.source` and
`order.destination` are in the SDK's `TESTNET_CHAINS`. That is the same set the SDK uses to pick
the testnet orderbook. A chain missing from it counts as mainnet, so it is screened.

## Where it runs

In `IntentFiller.handleNewOrder`, once any strategy's `canFill` is true. An order nothing here can
fill is never sent. Screening runs alongside the confirmation wait and the evaluation, and a drop
aborts the wait. The verdict is checked after evaluation and before the confirmation result and
`executeOrder`. A dropped order emits `orderSkipped` with one of these reasons:

- `Screening flagged the order`
- `Screening unavailable: <code>`, where `<code>` is the orderbook's `MessageRejectionCode` or
  `ScreeningFailureCode`, or one of `REQUEST_FAILED`, `COMMITMENT_MISMATCH`, `NO_COMMITMENT`,
  `SCREENER_ERROR`
- `Screening unavailable: no screener`, when `setScreener` was never called. Boot always calls
  it, so this reason only shows up in an embedder or a test.

## The request

`OrderScreener` signs EIP-712 `ScreenOrder(address solver, bytes32 commitment, uint64 timestamp)`
under the orderbook's `eip712DomainName`/`eip712DomainVersion` (`LimitOrderService.signingDomain`),
the same domain as `CancelOrder`. It sends `order` as `abi.encode(order)` (the `placeOrder` input
tuple, the bytes the commitment hashes) whenever those bytes hash to `order.id`. The orderbook
reads them only if its indexer has not seen the order yet.

The orderbook spends the signature once it has checked it, so every attempt is signed again at a
strictly later timestamp. There are up to four attempts, 500 ms, 1 s and 2 s apart
(`SCREENING_BACKOFF_MS`). An attempt is retried when:

- the request got no answer;
- the answer is `ScreeningFailed` with `retryable: true` (`ORDER_NOT_ON_CHAIN`,
  `PROVIDER_UNAVAILABLE`, …);
- the answer is `SIGNATURE_EXPIRED` or `SIGNATURE_REUSED`.

Every other answer is final. That includes `SCREENING_DISABLED` and `NO_ACTIVE_ORDER`: the
orderbook screens only for an active solver that holds an active, unexpired order on the book. A
verdict naming another commitment is also refused.

## Orderbook API surface

- `OrderbookClient.screenOrder`, in `ORDERBOOK_DOCUMENTS`. `ScreeningFailed.code` is aliased to
  `failureCode`, as on the submission results.
- New codes: `SOLVER_FLAGGED` (`RejectionCode`), `SCREENING_UNAVAILABLE` (`FailureCode`),
  `NO_ACTIVE_ORDER` (`MessageRejectionCode`), and `SCREENING_FAILURE_CODES`.
- `src/tests/fixtures/orderbook-schema.graphql` is now the PR's schema.

## Deployment order

Simplex drops every mainnet order until the mainnet orderbook serves `screenOrder` with a
Chainalysis key. Deploy the orderbook first.
