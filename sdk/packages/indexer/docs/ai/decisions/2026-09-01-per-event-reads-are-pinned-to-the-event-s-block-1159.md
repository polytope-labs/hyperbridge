# 2026-09-01 — Per-event reads are pinned to the event's block (#1159)

Chosen: the balance and position reads a refresh performs are pinned to the block of the event that triggered
them, via the SDK's `blockTag` parameter, and the read memo is keyed by that block so several events in one block
share one set of reads.

Alternative rejected — read at the chain head, as the periodic sweep does. It is what the first cut did, and it
is not replayable: reindexing an old fill would stamp today's balance onto it. The pool guard (skip a pool whose
last snapshot postdates the event) hid that by making the refresh a no-op during a resync; pinning the reads
makes the guard a cost optimization rather than the only thing standing between a replay and wrong data.

Only the event's own chain is pinned. Block numbers are per chain and a refresh reaches across every chain the
pool is quoted on, so the others stay at the head — the correct reading available for them.
