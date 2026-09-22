# 2026-09-22 — Quorum reads are bounded and audible

A quorum read now fails within its budget instead of waiting out its slowest endpoint, retries are
logged at `warn`, and one throttled endpoint no longer aborts startup.

## The deadline

`settleUntilQuorum` takes a deadline from the client's {@link QuorumBudget}. Past it, the endpoints
still outstanding are treated as failures, so the error names them.

The deadline cannot rescue a call. Early exit already resolves the moment the bar is reachable, and
it applies the same rule to the same responses `finalize` would see, so a deadline that fires is
always a call that was going to fail — it just fails in seconds rather than minutes. `QuorumError`
now lists the endpoints that never answered.

## Budgets, per call site

| | timeout | retries | deadline |
|---|---|---|---|
| `SCAN_BUDGET` | 5s | 1 | 12s |
| `CONFIRMATION_BUDGET` | 10s | 1 | 30s |

Scanning polls every 3s, so a read that has not answered in seconds is stale and is holding the scan
mutex. Confirmation polling goes through the same class (`ChainClientManager.getQuorumClient`) but
makes two requests in sequence per endpoint — head, then receipt — and gates paying out, so it gets
the roomier budget rather than the scan one. Wallet and `eth_call` clients are untouched at 30s × 4.

The scanner's own retry loop wraps the bounded call, so a scan's worst case is the head read (1
attempt, ~12s) plus the log read (2 attempts, ~24s). The head read does not retry internally: the
3-second tick is its retry loop, and a failed head read leaves the cursor untouched.

## What gets benched, and what that costs

`benchFor` replaces the rate-limit-only suspension check:

| Failure | Bench | Why |
|---|---|---|
| Request-rate limit | `Retry-After`, else 5 min | unchanged |
| Response that is not JSON | 30s | a plain-text quota notice surfaces as a parse error, so the rate-limit classifier never matched it and the endpoint was re-queried every scan |
| Head behind the quorum's | 15s | it fails every range at the quorum head until it catches up |

This widens what shrinks the voter set, which is a deliberate trade rather than a neutral change:
five endpoints with two lagging used to fail the call, and now decide it over the remaining three.
The bar is still a BFT threshold over the endpoints asked and the voters are still exclusively the
operator's, but a call can be decided by fewer of them than the operator configured. A benched
endpoint is one that cannot answer the question being asked; keeping it in the denominator turns its
problem into missed orders.

## Startup tolerance

`resolveChainConfigs` takes `tolerateUnreachable`. Boot and the paymaster keeper set it: endpoints
that cannot answer the chain-id probe are logged and kept, and only a chain where nothing answered
(or endpoints that disagree) is fatal. A filler with a dozen free endpoints per chain used to fail
boot whenever any one of them answered 429.

Runtime endpoint edits (`Simplex.setChainRpcUrls`) deliberately do not set it. That path probes
before mutating so a wrong-chain endpoint cannot be adopted, and an endpoint that never answered has
never been checked — on a quiet range it returns `[]` like an honest one, so it could join a quorum
on "no events".

## Retry logging

`RetryConfig.logger` is structural (`{ trace(message) }`) instead of a `ConsolaInstance`, which is
what kept callers with their own logging stack from passing one: `chain-scanner` omitted it,
`retryPromise` fell back to the SDK's silent default, and a retrying read logged nothing at any
level. The retry line now carries the error's first line, and `chain-scanner` passes an adapter that
records at `warn` — `trace` would still be invisible at the default level, and a retry that holds
the scan mutex is not a trace-level event.

## What this fixes

On a 24h mainnet run, one chain's scanner stopped for 366s with nothing in the log. A slow minority
kept the bar unreachable; each attempt waited ~123s for stragglers (30s × 4 transport attempts) and
`retryPromise` repeated it three times. Two endpoints in that set could never have voted: one
answered its quota notice in plain text, the other was behind the head.
