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

In `IntentFiller.handleNewOrder`, once the evaluation returns a strategy that would bid. An order
nothing here bids on is never sent: each screen costs a signature, a request, and a billed
analysis of every address the orderbook has not screened. The screen runs beside whatever is
left of the source confirmation wait, and a drop aborts that wait. The handler checks a reorged
placement first, since nothing screening says can save it. It then awaits the verdict, and only
then reads the rest of the confirmation result and calls `executeOrder`. When the handler is done
with the order, however it ends, it aborts a screen still retrying.

A dropped order emits `orderSkipped` with one of these reasons:

- `Screening flagged the order`
- `Screening unavailable: <code>`, where `<code>` is the orderbook's `MessageRejectionCode` or
  `ScreeningFailureCode`, or one of `REQUEST_FAILED`, `GRAPHQL_ERROR`, `COMMITMENT_MISMATCH`,
  `NO_COMMITMENT`, `SCREENER_ERROR`
- `Screening unavailable: no screener`, when `setScreener` was never called. Boot always calls
  it, so this reason only shows up in an embedder or a test.

## The request

`OrderScreener` signs EIP-712 `ScreenOrder(address solver, bytes32 commitment, uint64 timestamp)`
under the orderbook's `eip712DomainName`/`eip712DomainVersion` (`LimitOrderService.signingDomain`),
the same domain as `CancelOrder`. It sends `order` as `abi.encode(order)` (the `placeOrder` input
tuple, the bytes the commitment hashes) whenever those bytes hash to `order.id`. The orderbook
reads the parties from them, and needs no indexer record of the order.

Boot calls `assertOrderEncoding()` before anything starts. It hashes `ORDER_ENCODING_VECTOR`, an
order with every field set, and compares the result with the commitment Foundry's `cast` computes
for it independently. A drifted encoding stops the filler there. Otherwise every mainnet order
would go out without its bytes and wait on the orderbook's indexer.

`screenOrder` waits `SCREEN_ORDER_TIMEOUT_MS` (35 s), or the configured request timeout when
that is longer. The orderbook gives one screening up to 25 s and cuts `/graphql` off at 30 s with
a 503, so simplex receives its answer rather than abandoning work the orderbook is still doing.

## Retries

The orderbook spends the signature once it has checked it, so every attempt is signed again at a
strictly later timestamp. An attempt is retried when:

- the request got no answer, including an HTTP error such as the orderbook's 503;
- the answer is `ScreeningFailed` with `retryable: true` (`ORDER_NOT_ON_CHAIN`,
  `PROVIDER_UNAVAILABLE`, …);
- the answer is `SIGNATURE_EXPIRED` or `SIGNATURE_REUSED`;
- the answer is `NO_ACTIVE_ORDER`. That is often a passing state: between `LimitOrderService`
  withdrawing a limit order and posting its replacement, or until the next heartbeat brings
  suspended postings back.

The first retry comes after 500 ms, and the wait doubles to at most 8 s (`SCREENING_RETRY`).
Retries go on for 60 s, or for the order's expected confirmation wait (confirmations × block time)
when that is longer: the order waits that long anyway, and a screen that gives up drops it for
every solver at once.

Every other answer is final:

- `SCREENING_DISABLED`, `UNSCREENABLE_PARTY` and any other refusal;
- a GraphQL error (`GRAPHQL_ERROR`, from `OrderbookGraphqlError`), which is the orderbook
  refusing the request as written, for example while it does not serve `screenOrder` yet;
- a verdict naming another commitment.

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
Elliptic credentials. Deploy the orderbook first. At boot, `reportScreeningStatus` reads
`serverInfo.screeningEnabled` in a query of its own, so the long-standing `limits` query still
works against an older orderbook. If the filler has a chain outside `TESTNET_CHAINS`, it logs an
error when the orderbook has screening off or does not know the field yet.

Whether an order is screened is still decided by `TESTNET_CHAINS`, not by `screeningEnabled`.
Taking the orderbook's flag would let a mainnet filler pointed at an orderbook without screening
fill mainnet orders unscreened. A testnet chain the SDK does not list needs an SDK release
anyway, since `TESTNET_CHAINS` also picks its orderbook URL and its gas pricing.
