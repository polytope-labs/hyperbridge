# 2026-09-01 — `blockTag` is a parameter of the balance read, and a per-chain map on the memo (#1159)

Chosen: `getTotalSolverBalance` takes a `blockTag` defaulting to `"latest"`, and `memoizedSolverBalance` takes a
`Record<chain, blockTag>`.

Alternative rejected — a single `blockTag` on the memo. A refresh reaches across every chain a pool is quoted on,
while the event that triggered it happened on one; block numbers are per chain, so one tag applied to all of them
would read some other chain at an arbitrary point in its history. The map pins the event's chain and leaves the
rest at the head, which is the only correct reading available.

Alternative rejected — leave every read at the head. Simpler, and it is what a periodic sweep wants, but a
per-event re-read at the head is not replayable: reindexing an old fill would stamp today's balance onto it.
