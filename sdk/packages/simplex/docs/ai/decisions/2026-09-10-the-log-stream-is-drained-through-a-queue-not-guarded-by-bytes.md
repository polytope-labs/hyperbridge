# 2026-09-10 — The log stream is drained through a queue, not guarded by bytes

Decided: every frame leaves through one queue drained by an async pump that awaits the response's
`drain` event. The bound on a slow reader is a record count (`MAX_LOG_STREAM_QUEUE`, above
`MAX_LOG_PAGE` so a replay can never reach it), not `res.writableLength`.

The first version wrote the replay straight out in a `for` loop and guarded on
`res.writableLength > 1MB`. That cannot work, and review caught it: the loop never yields, so the
socket cannot drain *during* it, and `writableLength` accumulates monotonically across the whole
page. A 2000-record replay is routinely 1-2MB, so the guard fired on a reader who had simply never
been given a chance to read — and because a trip emits `event: gap`, and the page answers a gap by
re-running the feed, the same replay went out again and tripped it again. A healthy operator on the
tunnel got a page that never settled.

Awaiting drain also makes the guard mean what it says. A reader that is merely slow now applies
backpressure and receives every record; only one that has genuinely stopped reading accumulates
enough queued records to start losing the oldest.

The gap announcement itself stays, for the reason it was added: dropping silently lets the client
splice two non-adjacent stretches together and believe the feed is whole. It is now far rarer, and
no longer self-inflicted.

Rejected: raising the byte threshold. It moves the livelock rather than removing it — the replay
grows with the page size, and any threshold a full page can reach is one a healthy reader can trip.

Rejected: chunking the replay across ticks without awaiting drain. That yields to the event loop, so
`writableLength` becomes meaningful again, but it still writes past a reader that is not consuming;
the queue is where the bound belongs.
