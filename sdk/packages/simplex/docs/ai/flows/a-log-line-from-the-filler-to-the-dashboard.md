# A log line, from the filler to the dashboard

Read from the source and exercised by `src/tests/log-store.test.ts` and the `logs` block of
`src/tests/ui-server.test.ts`.

1. Something calls `logger.info({...}, "msg")`. `ModuleLogger` resolves through its `LoggerContext`
   per call, so the level in force is the current one, not the one at construction.
2. The context's pino instance drops the record outright if it is below the context's level —
   nothing downstream can recover it — and otherwise serializes it to NDJSON and hands the line to
   every registered sink.
3. `bin/simplex.ts` constructs one `LogStore` at module load and registers it on the process-wide
   context there and then, before anything has had a chance to log. The filler's own context gets it
   via `Simplex.start`'s `logger` option, wrapped with the console sink in a `fanout` — handed in
   rather than added afterwards, because `start()` builds that context and runs all of boot inside
   itself, so a sink attached after it resolves misses every boot record.
4. `LogStore.write` splits on newlines and parses each chunk. The numeric level comes from a regex on
   the raw line, not from the parsed object: a payload field named `level` displaces pino's own and
   would otherwise drop the record. Anything without a recognisable level is discarded and consumes
   no seq, so a resuming stream never skips. The rest becomes `{seq, time, level, module, msg,
   detail}` — `moduleTag` loses its brackets, every non-envelope field is re-serialized into `detail`
   (flat-copied and capped), and a caller's own `level` is put back into `detail` as data.
5. `append` writes the record to this launch's file as one NDJSON line — never awaited, so a slow
   disk cannot stall the fill that logged it — then stores it in the ring at `head` (O(1); an array
   `splice` here measured ~7x the cost of everything else on this path), then calls every subscriber
   inside a try/catch.
6. `openLaunchFile`, called from the `run` action once `--data-dir` is parsed, prunes to the last
   five launches, opens `<dataDir>/logs/simplex-<stamp>.log` with `wx`, and writes out whatever the
   ring already holds — which is what makes the file start at launch rather than at that call. A
   stream error always clears the stream, so recording stops; whether it clears the *path* turns on
   `stream.pending`. Still pending means the open itself failed — the `wx` collision, or a directory
   that cannot be written — so nothing of this launch reached that path and it must not be read back
   as this launch's history. Already open means a full disk or the like, and the file is ours and
   complete up to the failure, so the path stays and `scanFile` keeps answering from it. The footer
   reads the difference: `persisted` for a file still being written, a retained `path` without it for
   "history to here on disk", neither for memory-only.
7. The page mounts and issues `GET /api/logs?level&q&after&limit`. `UiServer` builds a `LogQuery`
   with `logQueryFrom` (unparseable values fall back rather than 400) and answers with
   `await logs.recent(query)` plus `config.simplex.logging`, the ring capacity, the count captured
   since launch, and where the history is being written.
8. `recent` drops an `after` at or beyond this launch's counter (seq restarts at 1, so a mark from a
   dead process would otherwise filter out everything) and matches the ring. If the ring alone fills
   the requested page it returns there — anything older could not survive the final slice, and the
   scan is the expensive part. Otherwise, and only when the query reaches past the ring's oldest seq,
   `scanFile` streams the launch file line by line for records below that seq, keeping at most
   `limit`, and the two halves are concatenated.
9. The page then opens `GET /api/logs/stream`, resuming from the last seq the backfill gave it.
   `streamLogs` subscribes *first* and buffers into `pending`, because the replay it then awaits can
   hit the file; once the replay is queued the buffered records follow, ordered by seq. Live records
   are filtered by `level` and `q` only — `after` gated the replay and stops there.
10. Everything leaves through one queue drained by an async pump that awaits `drain`, so a slow
    reader gets backpressure rather than being written past. Only a reader that has stopped entirely
    accumulates past `MAX_LOG_STREAM_QUEUE` records and starts losing the oldest; the first frame
    after that is preceded by `event: gap`, which makes the client re-run the whole feed rather than
    splice a hole shut.
11. In the browser, records land in a ref and flush into state every 150ms; `LogRow` is memoized, so
    a flush reconciles the arrivals rather than all 2000 rows. The scroll container follows the tail
    unless the operator has scrolled more than 40px off the bottom, and re-anchors whenever the list
    is replaced wholesale.
12. Changing the level, the (debounced) search, Clear, or Pause tears the whole feed down and starts
    it again — one effect owns both requests, so there is no state to keep in sync between them. A
    stream error or a `gap` does the same, after 2s in the error case, rather than letting
    `EventSource` re-open the same URL and be handed the same replay.
13. Choosing a level awaits `PUT /api/log-level` before anything else, since the feed's own GET reads
    the level back out of the config and would otherwise race the write. `handleLogLevel` calls
    `operator.setLogLevel`, which in the CLI moves both the filler's context and the process-wide
    one, writes `config.simplex.logging`, and persists the TOML. Step 2 then starts — or stops —
    letting those records through.
14. `UiServer.stop()` unsubscribes and ends every open tail; `req.on("close")` does the same for one
    client that navigates away. `logStore.close()` on shutdown ends the launch file; the records stay
    on disk for the next five launches.
