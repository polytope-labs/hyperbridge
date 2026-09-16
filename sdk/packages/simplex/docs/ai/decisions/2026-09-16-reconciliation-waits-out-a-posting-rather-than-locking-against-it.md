# 2026-09-16 — Reconciliation waits out a posting rather than locking against it

Decided: a limit order whose row was written less than two minutes ago is skipped by reconciliation
instead of being posted again, whatever put it in that state.

Three paths leave a live row with no entry on the book for as long as a round trip takes:

- `create` inserts the row `open` with no commitment, then posts;
- `settleFill` moves it to `resizing`, cancels the old entry and posts a new one;
- `renewExpiring` cancels and posts, and the row stays `open` throughout.

Reconciliation running in any of those windows sees exactly what it sees after a crash: a row that
should have an entry, and an orderbook that does not list one. Posting from both would put two live
entries behind one liability, which is the thing section 6 went out of its way to avoid. The rule was
first written for resizes alone, which left the other two unguarded.

The grace period reads the one piece of evidence that is already there. `updatedAt` is written on the
insert and on every status write, so a row touched moments ago has a posting in flight and one that
has sat for minutes was stranded by something that is not coming back. `repost` marks the row
`resizing` as it starts, which both refreshes that stamp and says what the row is doing. The cost of guessing wrong
is one reconcile cycle of an order not being advertised, against advertising the same output twice.

Rejected: a mutex on the service. It would be correct in process and worth nothing across a restart,
which is the case reconciliation exists for, and it would have to be held across network calls.

Rejected: a `reposting` flag on the row. That is the same information `updatedAt` already carries,
with a second write in front of every repost and a new way for a crash to leave a row lying about
what it is doing.
