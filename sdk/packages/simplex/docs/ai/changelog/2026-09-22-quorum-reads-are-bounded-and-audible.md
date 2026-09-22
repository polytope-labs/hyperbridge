# 2026-09-22 — Quorum reads are bounded and audible

A quorum read now fails within `QUORUM_CALL_DEADLINE_MS` (12s) instead of waiting out its slowest
endpoint, and the scanner's retries are logged.

`settleUntilQuorum` still resolves the moment the bar is reachable from the endpoints that have
answered. What changed is the other path: when the bar is *not* reachable, the outstanding endpoints
are treated as failures once the deadline passes, so `finalize` decides on the rest or throws a
`QuorumError` naming the endpoints that held the call up. The trust model is unchanged — a decision
still requires the same quorum.

Per-endpoint HTTP budget for these reads drops to `READ_TIMEOUT_MS` 5s with one retry, from 30s with
three. A scan polls every 3s, so a read that has not answered in seconds is already stale.

`benchFor` replaces the rate-limit-only suspension check and names three cases:

| Failure | Bench | Why |
|---|---|---|
| Request-rate limit | `Retry-After`, else 5 min | unchanged |
| Response that is not JSON | 30s | a plain-text throttle notice or HTML error page surfaces as a parse error, so the rate-limit classifier never matched it and the endpoint was re-queried every scan |
| Head behind the quorum's | 15s | it fails every range at the quorum head until it catches up |

`QuorumError` labels each failing endpoint with its class (`[rate-limited]`, `[malformed response]`,
`[behind the head]`).

`RetryConfig.logger` in the SDK is now structural (`{ trace(message) }`) rather than a
`ConsolaInstance`, which is what kept callers with their own logging stack from passing one:
`chain-scanner` omitted it, `retryPromise` fell back to the SDK's silent default, and a retrying
read logged nothing at any level.

## What this fixes

On a 24h mainnet run, one chain's scanner stopped for 366s with nothing in the log. A slow minority
kept the bar unreachable; each attempt waited ~123s for stragglers (30s × 4 transport attempts) and
`retryPromise` repeated it three times. Two endpoints in that set could never have voted: one
answered its quota notice in plain text, the other was behind the head. With these changes the same
episode costs one 12s call that says which endpoints were waited on.

Non-scan clients (fills, `eth_call`, gas) keep the 30s budget — a fill should be patient.
