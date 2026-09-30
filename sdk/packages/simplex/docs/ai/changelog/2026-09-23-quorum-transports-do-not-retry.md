# 2026-09-23 — Quorum transports do not retry

The per-endpoint transports behind a quorum call are built with `retryCount: 0`. One endpoint now
contributes exactly one request to a call, and `QuorumBudget` is down to `timeoutMs` and
`deadlineMs`.

## The clamp that never applied

`suspensionMsFor` already honours a provider's `Retry-After`, clamped to
`[1s, RATE_LIMIT_SUSPENSION_MS]`. Its comment says why: a per-second limiter answering
`Retry-After: 1` should not cost five minutes of degraded quorum.

viem implements the same header one layer down, in `buildRequest`:

```js
const retryAfter = error?.headers?.get('Retry-After')
if (retryAfter?.match(/\d/)) return Number.parseInt(retryAfter, 10) * 1000
```

Uncapped, ahead of our configured `retryDelay`, and it reaches the socket first — so the clamp never
got a chance to apply. `shouldRetry` returns true for 403, 408, 413, 429 and the 5xx family, so this
was the ordinary path for a throttled free endpoint, not an edge case. With no transport retry there
is nothing left for the header to delay, and the bench is the only thing acting on it.

## Headroom

A retrying task's worst case was timeout, backoff, timeout: 5s + 500ms + 5s against a 12s deadline,
so 1.5s of margin. The deadline is meant to catch a call going wrong, but at that margin it also
catches an endpoint using its budget exactly as designed, plus any jitter in when the timeout fires.
`withTimeout` is `setTimeout` then `controller.abort()`, so the 5s cap is only as punctual as the
timer.

One attempt puts the worst case at 5s against 12s, and at 10s against 30s for confirmations. A
deadline that fires now means something outside the budget went wrong.

## What this costs

The bar is `quorumThreshold(queried.length)`, computed over the endpoints asked rather than the ones
that answered, so an un-retried transient does not lower the bar — it removes a voter. Where one
500ms retry used to rescue a call, four simultaneous transients on a 12-endpoint chain are now a
`QuorumError`.

It self-corrects on the next call, since benched endpoints drop out of `queried` and the bar
recomputes over the remainder. The shape to expect is a miss on the first call of a rate-limit wave,
then adaptation. How often a second attempt was actually succeeding is not measurable from outside
viem, so this is a real trade rather than a free one.

## Where the 311 deadline hits came from

Not the endpoints. A 19h mainnet soak hit the deadline 311 times, all inside 45 minutes of a single
14:00–19:00 window, three or more chains in the same minute. Bucketing the soak's 1148 minutes by
concurrent CI runs on the host gave zero hits across 174 CI-idle minutes, 1.3% at 1–2 concurrent and
10–20% at 3 or more. The filler shares a machine with the self-hosted runners, and a Rust build
delays Node's timers in bursts — median tick interval was unmoved at 3.02s against 3.00s while p95
went from 6s to 24s.

So those hits are an artefact of the test host and not evidence that the endpoints or the filler were
unhealthy. They are still the reason to act: a production filler shares a box with something too, and
7s of headroom absorbs what 1.5s does not.
