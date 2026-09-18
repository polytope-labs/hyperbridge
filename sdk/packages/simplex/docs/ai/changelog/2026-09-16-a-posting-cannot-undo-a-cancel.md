# 2026-09-16 — A posting cannot undo a cancel

`setPosting` and `setStatus` write what the caller asks, so a posting that was already in flight when
the operator cancelled the order, or when the expiry sweep took it down, wrote the row back to `open`
with a fresh commitment and a live entry on the book. The cancel was undone and the order started
matching swaps again. Every posting path could do it: the repost after a fill and
reconciliation.

Both writes now take an optional list of statuses the row must still hold, and answer null when it
has moved on. `post` passes `open` and `resizing`, and when the write does not apply it withdraws the
entry the orderbook has just accepted, since nothing here owns it any more. A refusal arriving late
is dropped for the same reason: it says nothing about the row the operator left behind.

The guard is a condition on the update rather than a read followed by a write, so two callers racing
cannot both decide they were first.
