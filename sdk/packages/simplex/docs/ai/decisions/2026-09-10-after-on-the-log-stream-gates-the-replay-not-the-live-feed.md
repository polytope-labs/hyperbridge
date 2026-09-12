# 2026-09-10 — `after` on the log stream gates the replay, not the live feed

Decided: `GET /api/logs/stream?after=N` skips records at or below N in the backfill, then streams
everything new that matches `level` and `q` — regardless of its seq.

`after` exists so the page can resume from the last record it holds without re-receiving it. Applied
to records that have not happened yet it means nothing, and it actively breaks: seq is per-process,
so a filler restart resets it to 1, and a client reconnecting with a seq from the old process would
match nothing forever. The page would sit there, connected, permanently blank. The replay loop's own
watermark is what prevents a double-send, and it starts at 0 rather than at `after` for the same
reason. `LogStore.recent` applies the same rule from the other side: an `after` at or beyond this
launch's counter is dropped rather than filtering everything out.

Rejected: making seq durable across restarts (persisted, or seeded from the clock). The ring is
explicitly a live tail and the file is the durable record; paying for persistence to make a resume
token survive a restart of the very process whose logs it indexes is backwards.
