# 2026-09-16 — Postings are kept alive, swept and reconciled

`LimitOrderLifecycle` runs three jobs on their own clocks while the filler runs, and `bootFiller`
starts and stops it with the filler. Each is also callable on its own from `LimitOrderService`.

**`heartbeat()`** signs `Heartbeat(solver, timestamp)` on the same EIP-712 domain as `CancelOrder`
and sends it on half of `serverInfo.heartbeatIntervalSecs`. It answers null while nothing is posted,
because the orderbook only knows a solver it has accepted an order from and would answer
`UNKNOWN_SOLVER`. `SIGNATURE_REUSED` and `SIGNATURE_EXPIRED` are both about the timestamp alone, so
either is answered by re-signing once a second later. A posting that comes back `surfaced: false`
means the solver is still suspended, so `post` fires a heartbeat on the spot.

**`expireStale(now)`** withdraws every order that has outlived its TTL and closes the row. That TTL
is the order's whole life: it is written into the posting and derived into `expiresAt` when the order
is created, and nothing renews either. When it runs out the posting lapses and the order is done, and
the operator posts a fresh one if they still want the depth. `repost` refuses an expired order too,
so neither the sweep nor reconciliation can put one back up.

**`reconcile()`** pages through `solver(address).orders` and compares it with what is open here.
An entry no limit order owns is cancelled, an order whose entry has gone is posted again without the
cancel `repost` normally leads with, and an entry the solver cannot cover is left where it is with
`UNDER_FUNDED` on the row for the operator, since reposting it would only have it cut down again.
"Cannot cover" is `resized`, or a `backed: false` on an entry carrying a `validatedAt`: `backed` is
also false before any cycle has read a balance, and a posting surfaces at its full quoted size
meanwhile. A row written in the last two minutes is skipped whatever its status, because a posting
may still be in flight, which
`2026-09-16-reconciliation-waits-out-a-posting-rather-than-locking-against-it.md` covers.

A pass that throws is logged and its clock carries on, and a tick is skipped while the previous pass
is still running. An orderbook that is down at boot does not stop the filler starting: it prices from
the local limit orders either way, and the heartbeat period falls back to
`FALLBACK_HEARTBEAT_INTERVAL_MS` until `serverInfo` can be read.

Config: `orderbook.reconcileIntervalSecs` was already accepted and validated but unread, and now
defaults to `DEFAULT_RECONCILE_INTERVAL_SECS` (300). The expiry sweep runs on a fixed 30 second
clock, since what it acts on is each order's own TTL.
