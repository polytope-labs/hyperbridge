# 2026-09-10 — The Logs level pills set the filler's level in both directions

Decided: choosing a level on the Logs page PUTs `/api/log-level` and the filler starts recording at
that level — raising *or* lowering. There is no separate view filter.

The first revision made this raise-only, on the reasoning that narrowing a view should not silence
the console. That was wrong about what the control is for. A filler runs for weeks; the reason an
operator reaches for the level is to stop it writing a log file they do not want, and a control that
can only ever increase verbosity cannot do the one thing they came for. Raise-only also turned out
to be unenforceable: `capture` is local state refreshed only by this page's own backfill, so a
second tab holding a stale value would happily PUT a lower level anyway.

Rejected: two controls (a view filter plus a capture setting). More expressive — you could watch
errors only while still recording trace — but it makes the common case a two-control puzzle, and the
history is on disk now, so narrowing the view is what the search box and a re-query are for.

The PUT is awaited rather than optimistic. Changing the level re-runs the feed effect, whose
`GET /api/logs` reports the level straight out of `config.simplex.logging`; fired concurrently, that
read can answer before the write lands and show the operator the old level back.

`setLogLevel` moves the process-wide context as well as the filler's. The page merges records from
both and reports one level for the result, so leaving the process context pinned at its default made
that a lie in both directions — stray `info` records below a raised floor, and modules that never
followed a lowered one.
