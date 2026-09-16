# 2026-09-16 — Reconciliation waits out a resize rather than locking against it

Decided: a `resizing` limit order whose row was written less than two minutes ago is skipped by
reconciliation instead of being posted again.

`settleFill` moves a row to `resizing`, cancels the old entry and posts a new one. Reconciliation
running in that window sees exactly what it sees after a crash: a row that should have an entry, and
an orderbook that does not list one. Posting from both would put two live entries behind one
liability, which is the thing section 6 went out of its way to avoid.

The grace period reads the one piece of evidence that is already there. `updatedAt` is set when the
status moves, so a row that has only just gone to `resizing` has a repost in flight, and one that has
sat there for minutes was stranded by something that is not coming back. The cost of guessing wrong
is one reconcile cycle of an order not being advertised, against advertising the same output twice.

Rejected: a mutex on the service. It would be correct in process and worth nothing across a restart,
which is the case reconciliation exists for, and it would have to be held across network calls.

Rejected: a `reposting` flag on the row. That is the same information `updatedAt` already carries,
with a second write in front of every repost and a new way for a crash to leave a row lying about
what it is doing.
