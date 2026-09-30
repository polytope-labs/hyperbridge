# 2026-09-10 — Log stream backpressure, a wasted file scan, and same-second launches

Four findings from review on #1246, all real.

The stream's backpressure guard defeated itself. It wrote the replay in a `for` loop and checked
`res.writableLength > 1MB` per record — but the loop never yields, so the socket cannot drain during
it and that number only accumulates. A 2000-record page is routinely 1-2MB, so a *healthy* reader
tripped the guard, got `event: gap`, and the page answers a gap by re-running the feed — which
replayed the same page and tripped again. An operator on the tunnel got a page that never settled.
Every frame now leaves through one queue drained by an async pump that awaits `drain`, and the bound
is a record count above `MAX_LOG_PAGE`, so a replay cannot reach it and the guard means what it
says. The regression test asserts a 1200-record, ~2.4MB replay reports no gap; it fails against the
old guard.

`LogStore.recent` decided `needsHistory` before measuring what the ring had returned. On any filler
whose ring had wrapped, an unresumed GET scanned every line of the launch file below `oldestInRing`
— gigabytes on a day at debug — and then `slice(-limit)` discarded all of it, because the ring alone
had already filled the page. The debounced search box repeated that per keystroke. It now returns
early when the ring covers the page.

The launch stamp had second resolution and the stream opened `flags: "a"`, so a `RestartSec=0`
restart appended a second seq-1..N run into the dead launch's file; `scanFile` then read those back
as this launch's history and they collided with the ring on seq. The stamp carries milliseconds and
the stream opens `wx`, so a collision is refused rather than appended into. The stream's error
handler now clears the path along with the stream — a half-owned file must not be read back as
history, nor advertised in the DTO.

`Operator.tsx` still called `useIsMobile()` after `handheld` took over its every use, leaving a live
`matchMedia` subscription re-rendering the shell for nothing. Biome only lints `src/`, so CI would
not have caught it.

Also here: the log viewer breaks out of the page inset. A log line is wide, and every column the
inset takes is message text that wraps instead — at 1440 that is ~90px moved from margin into the
message column. The page heading stays where every other page puts it; the toolbar and footer travel
with the stream, since a search field narrower than the content it filters looks like a mistake. The
inset itself became `--page-inset` on `.operator-main` so the breakout tracks it across the
breakpoints rather than hardcoding a value three media queries disagree about. And the SSE test
helper now buffers partial lines; a frame larger than a TCP segment arrives split, and it was
parsing the fragments.

Files: src/services/server/UiServer.ts, src/services/server/LogStore.ts, ui/src/operator/Operator.tsx,
ui/src/styles/logs.css, ui/src/styles/operator.css, ui/src/styles/responsive.css,
src/tests/ui-server.test.ts, src/tests/log-store.test.ts.
