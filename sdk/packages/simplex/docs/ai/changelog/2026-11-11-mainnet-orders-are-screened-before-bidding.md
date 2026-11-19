# 2026-11-11 — Mainnet orders are screened before any bid

Simplex screens every mainnet order through the HyperFX orderbook's `screenOrder` mutation
(polytope-labs/hyperfx-orderbook#76) before it bids. The orderbook checks the order's user and
beneficiary against Elliptic and keeps one verdict per commitment. Simplex drops an order the
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

## The solver's own screening

The orderbook screens the solver too, and a solver it has not cleared cannot ask for a screen:

- A solver is held suspended until its first clean screen. That screen comes once the
  orderbook's validation cycle has read the solver's balance. Until then every mainnet order is
  dropped with `NO_ACTIVE_ORDER`.
- A suspension (`SOLVER_SUSPENDED`) lasts until a screen passes. The orderbook keeps the
  solver's orders meanwhile, so `LimitOrderService` treats a posting refused with it like a
  retryable failure: the limit order stays `open`, with no commitment and the code in
  `lastError`, and reconciliation posts it again.
- A ban (`SOLVER_FLAGGED`) is final, and a posting refused with it retires the limit order as
  `rejected`, like any other refusal.

## Orderbook API surface

- `OrderbookClient.screenOrder`, in `ORDERBOOK_DOCUMENTS`. `ScreeningFailed.code` is aliased to
  `failureCode`, as on the submission results.
- New codes:
  - `RejectionCode`: `SOLVER_FLAGGED`, `SOLVER_SUSPENDED`.
  - `FailureCode`: `SCREENING_UNAVAILABLE`.
  - `MessageRejectionCode`: `NO_ACTIVE_ORDER`, plus the heartbeat-signer codes
    `SOLVER_NOT_VALIDATED`, `HEARTBEAT_SIGNER_EXPIRED`, `AUTHORIZATION_SUPERSEDED` and
    `SOLVER_FLAGGED`. Simplex never receives the heartbeat-signer codes, because the solver's own
    key signs its heartbeats.
  - `SCREENING_FAILURE_CODES`.
- `src/tests/fixtures/orderbook-schema.graphql` is the PR's schema at `ef00848`.

## Deployment order

Simplex drops every mainnet order until the mainnet orderbook serves `screenOrder` with
Elliptic credentials. Deploy the orderbook first.
