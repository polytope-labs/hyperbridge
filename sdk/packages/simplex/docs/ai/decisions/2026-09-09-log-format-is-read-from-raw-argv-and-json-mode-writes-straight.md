# 2026-09-09 — `--log-format` is read from raw argv, and json mode writes straight to stdout (#1237)

Decided: `bin/simplex.ts` calls `logFormatFromArgv(process.argv)` at module scope and builds its
console sink from the result. Commander declares `--log-format <pretty|json>` on `run` as well, but
only so that `--help` lists it and a bad value is rejected; the action never reads the parsed value.

Why the scan rather than the parsed option: `addLogSink(consoleSink(...))` runs while the module is
still evaluating, so the sink exists before anything can log. Commander has not run at that point. The
sink has to exist before the parse, so its format has to be known before the parse.

Why the action ignores `options.logFormat` even though it is available by then: the sink is already
writing, and a command line the two readers disagree about (`simplex run -c --log-format json`, where
commander binds `--log-format` to `-c`) would then put an ASCII banner in the middle of a stream a
supervisor is parsing as NDJSON. One reader, one answer. `RunOptions` in `src/cli/run-options.ts`
therefore omits `logFormat`, with the reason recorded on the field it would have occupied.

Both flags live in `addRunOptions` (`src/cli/run-options.ts`) with the rest of `run`'s options, after
#1249 split that builder out of the bin. That is also what lets the tests parse the *real* declarations
rather than a hand-copied mirror that could drift.

Rejected: moving the `addLogSink` call into each command's action, after the parse. It would remove
the double read, but the process-wide sink is what the setup wizard logs to — `UiServer`'s
`getLogger("ui")` and `setup-api`'s `getLogger("setup")`, which produce the first record of a wizard
run — as well as the keeper command, so each would need its own copy of the wiring. That is a real
cost paid for a cosmetic gain. (`init` is not in that list: nothing under `src/cli/init/**` resolves a
logger, and it spawns `simplex run` as a child rather than sharing the process.)

Decided: in json mode `consoleSink()` returns `process.stdout` itself. pino hands a destination one
finished NDJSON record per write, newline included, so there is nothing left to format.

Why not pino-pretty with `colorize: false`: pino-pretty would still reformat, drop fields and reorder
them. The point of json mode is that a parser downstream sees exactly what pino produced.

This also removes the interleaving hazard that forces one pino-pretty transform per writer on the
pretty path. Two pino instances sharing one pino-pretty transform interleave their chunks, and it
echoes the unparseable remainder as raw NDJSON. With no transform in between there is nothing
reassembling anything, so the process context and a running filler can share one `process.stdout`
safely. Checked by running two `LoggerContext`s into a shared stdout for 4000 records with 4 KB
payloads: no partial or interleaved lines.

Decided: in json mode the ASCII banner is not printed and the wizard's URL line becomes a log record
with a `url` field. Those two are the only writers to stdout outside the logger *in this package*, and
either one would break the promise that every line is a JSON object. The URL is more useful as a field
than as prose a parser would have to scrape.

The promise is therefore about what simplex itself writes, not about the process. `@polkadot/util`'s
logger routes `log` to `console.log`, and `@polkadot/api`'s `Init.js` uses it to announce "Runtime
version updated to spec=…"; `bin/quiet.ts` patches only `console.warn`, so that one line would reach
stdout as plain text on a genuine Hyperbridge runtime upgrade. Rejected for now: widening `quiet.ts` to
redirect that logger's `console.log` to stderr. It is the right fix if json mode ever needs to be
absolute, but it changes behaviour on the pretty path too, and this change was scoped to two flags.

Rejected: deciding the format by whether stdout is a TTY, the way many tools do. It is the more
convenient default, but it silently changes what an existing user gets the moment they pipe simplex
into a file — and `colorize: true` is currently explicit, so today they get colour there on purpose.
An explicit flag keeps "no flags" meaning exactly what it meant before.
