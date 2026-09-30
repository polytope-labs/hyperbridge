# 2026-09-10 — A Logs page in the dashboard, with launch-long history on disk

The filler's logs were only readable on the console of whatever terminal or container it was
started from, so an operator on the dashboard could not see why a fill was skipped. Adds a Logs
page to the operator sidebar, backed by `LogStore` (`src/services/server/LogStore.ts`).

`LogStore` is fed by registering its `sink()` on a `LoggerContext`, exactly as the console sink is,
and keeps two tiers because "all of it" and "fast" pull in opposite directions. Every record is
appended to this launch's NDJSON file under `<dataDir>/logs/`, which is the history and is bounded
only by the disk; older launches are pruned, keeping five. The last 5000 are also held in a ring,
which is what the live stream broadcasts and what answers a query outright as long as nothing has
been evicted. When a query does reach past the ring's oldest record the older half is read back off
the file, splitting at that seq — which also sidesteps the write stream's buffer, since the records
not yet flushed are exactly the ones the ring still holds.

The store is constructed and registered at module load, before anything can log, and handed to
`Simplex.start` inside a fan-out sink rather than attached afterwards: `start()` builds the filler's
context and runs all of boot inside itself, so a sink added after it resolves misses every boot
record. `openLaunchFile` runs once `--data-dir` is parsed and writes out whatever the ring already
holds, so the file genuinely begins at launch.

Two routes. `GET /api/logs` returns the backfill plus the level, the ring capacity, how many records
have been captured and where the history is written. `GET /api/logs/stream` is an SSE tail that
replays from `after` and then goes live; it subscribes before awaiting the replay, since reading
history can hit the file, and orders the two sources by seq on the way out.

The level pills set the filler's log level in both directions — raising records more, lowering stops
recording it at all, which is how an operator keeps a long-running filler from writing a log file
they do not want. The PUT is awaited rather than optimistic, because changing the level re-runs the
feed and its GET reads the level back out of the config. `setLogLevel` moves the process-wide
context as well as the filler's, so the one level the page reports is true for the whole merged feed.

The page is desktop-only: a log line is a wide, dense, monospace record, and it is the only page
holding an open stream and thousands of rows. The gate is `HANDHELD_QUERY`, not the layout's width
breakpoint, because a phone in landscape is wider than that breakpoint.

Files: src/services/server/LogStore.ts, src/services/server/dto.ts, src/services/server/UiServer.ts,
src/bin/simplex.ts, ui/src/operator/Logs.tsx, ui/src/styles/logs.css, ui/src/operator/Operator.tsx,
ui/src/lib/route.ts, ui/src/lib/hooks.ts, ui/src/components/ui/ResponsiveDialog.tsx,
ui/src/components/InterfaceIcons.tsx, ui/src/types.ts, ui/src/main.tsx, ui/src/styles/operator.css,
ui/src/styles/responsive.css, src/tests/log-store.test.ts, src/tests/ui-server.test.ts.
