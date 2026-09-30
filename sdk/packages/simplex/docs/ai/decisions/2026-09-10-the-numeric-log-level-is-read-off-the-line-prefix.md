# 2026-09-10 — The numeric log level is read off the line prefix, not the parsed object

Decided: `parseRecord` recovers pino's numeric level with a regex on the raw line, falling back to
the parsed field, and puts any caller-supplied `level` into `detail`.

pino writes its own base fields first and the caller's merge object verbatim after, so
`logger.warn({ level: "debug" }, …)` emits a line with two `level` keys and `JSON.parse` keeps the
caller's. The numeric lookup then failed and the record was dropped — silently, and only from the
dashboard, since the console sink renders it fine. The instance that made this worth fixing rather
than documenting is the UI server's own `warn({ level }, "Log level changed from the UI")`: the one
record documenting a level change disappeared from the page it was logged for.

Rejected: renaming the payload field at that call site. It fixes one caller and leaves the trap for
every future one, in a package where `{ level }` is a natural thing to log.

Rejected: pino's `nestedKey`, which would nest every merge object under one key and remove the class
of collision entirely. It also changes what `pino-pretty` renders for every record on the console,
which is a far larger blast radius than the bug.
