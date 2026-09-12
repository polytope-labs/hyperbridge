# 2026-08-27 — Request pacing lives at the provider, keyed by endpoint

Chosen: an `HttpProvider` subclass whose `send` waits on a shared `TokenBucket`, with one bucket per endpoint origin in a module-level map.

Alternatives considered:

- **Pacing inside `pollPhantomOrders`** — a sleep between blocks, or a queue around the scan. Rejected: the poll is not the only caller. Balance polling, the offchain fan-out in simplex's `handlePhantomOrders`, and the HTTP submission fallback all hit the same endpoint, and the limit counts them together. Pacing the loudest caller leaves the sum unpaced, and it is the sum the node sees.
- **A bucket per `IntentsCoprocessor`.** Rejected: several fillers in one process each hold their own coprocessor, so N instances would each pace to the full budget and collectively exceed it by N. The limit is a property of the endpoint, so the bucket is too.
- **`p-queue` with `intervalCap`,** which the package already depends on. Rejected: its interval is a fixed window that refills all at once, so a burst arriving just after a boundary is passed straight through, which is the exact shape being defended against. A token bucket refills continuously.
- **Reacting to 429s only** (backoff, no pacing). Kept as well, but not instead: a 429 is already a request spent, and a limiter that is shedding load may be counting rejections too. Backoff recovers from a breach; the bucket is what stops causing them.

Two consequences worth knowing. The bucket makes a large scan a *queue*, so anything sharing the endpoint waits behind it — which is why `maxBlocksPerPoll` dropped to 20 in the same change; at 500 a catch-up would have queued ~1000 requests, over two minutes of them, ahead of a bid submission that is worth nothing after five blocks. And the bucket is FIFO on purpose: without it, a caller arriving on an idle bucket takes the token a queued one was waiting for, and a steady arrival stream starves the queue.

Why not simply raise the limit with the provider: the endpoint is derived from the websocket's, not configured (phantom orders live in that node's offchain storage), so operators do not necessarily control it. `HYPERBRIDGE_RPC_MAX_RPS` exists for those who do.
