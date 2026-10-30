# Tesseract resumes each pipeline from a saved checkpoint, and a failed substrate chunk fails its whole event query

## Every pipeline saves its last processed height

A new `ProcessedHeights` table in the relayer's sqlite database stores one row per pipeline, keyed
by `(source, dest)`. Both columns hold a state machine's display string, such as `SUBSTRATE-PDEX` or
`KUSAMA-4009`. The outbound fan-out serves every EVM destination from one cursor, so its row uses the
fixed dest `"outbound"`.

`handle_notification` writes the row after a range is handled and the cursor moved. `outbound::run`
writes it right after it advances its cursor. The row therefore never runs ahead of the in-memory
cursor. If a write fails, the relayer logs a warning and carries on: a checkpoint that lags only
makes the next restart re-read more.

At startup each pipeline reads its row and begins at `min(checkpoint, initial_height)`. A restart
therefore never skips a range. A lower configured `initial_height` still rewinds the pipeline, which
is how to deliberately re-read old ranges. To skip ahead, delete the pipeline's row or start from a
fresh database. A failed read of the row ends the pipeline rather than silently starting at the
initial height, because a crash is recoverable and a skipped range is not.

## A failed substrate chunk fails the whole event query

`SubstrateClient::query_ismp_events` reads in 100-block chunks. A chunk whose `ismp_queryEvents`
call fails now returns an error for the whole query, matching the EVM provider, instead of returning
the other chunks' events. The caller leaves its cursor unchanged and reads the range again, so a
partial read can no longer pass for "no events". This covers Hyperbridge itself, the Hyperbridge to
substrate pipelines, and substrate source chains such as PDEX.

## What a restart can still miss

The inbound task hands GET requests to the GET processor over an in-memory channel. A request
forwarded just before a crash, and not yet stored by the processor, can still be missed on restart.
Requests already stored in the database are retried as before.
