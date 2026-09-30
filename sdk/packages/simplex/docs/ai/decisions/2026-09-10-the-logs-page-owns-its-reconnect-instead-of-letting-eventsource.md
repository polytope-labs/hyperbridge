# 2026-09-10 — The Logs page owns its reconnect instead of letting EventSource retry

Decided: on a stream error the page closes the `EventSource` and re-runs the whole feed effect after
a delay — a fresh `GET /api/logs` followed by a fresh stream — rather than letting the browser's
built-in reconnect re-open the same URL.

The built-in reconnect re-requests the identical URL, including its `after`, and the server answers
with the same replay it sent the first time. Every row already on the page arrives again, duplicate
React keys and all. Starting over instead re-reads the backfill and derives a correct `after` from
what the page actually holds, which is also the only path that recovers when the filler restarted
and the seq counter went back to 1. An `event: gap` frame takes the same path, for the same reason.

Rejected: de-duplicating on insert by dropping any record whose seq is not greater than the last one
held. Cheaper, but it reintroduces the reset trap in the client — after a restart every incoming seq
is below the watermark and the feed freezes.
