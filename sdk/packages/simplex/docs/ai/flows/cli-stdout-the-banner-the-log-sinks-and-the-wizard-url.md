# CLI stdout: the banner, the log sinks, and the wizard URL

Read from `bin/simplex.ts` and exercised against the built `dist/bin/simplex.js` on 2026-09-09.

1. Module evaluation, before commander sees anything. `logFormatFromArgv(process.argv)` scans raw argv
   for `--log-format` (both `--log-format json` and `--log-format=json`, last occurrence wins, stopping
   at `--`) and falls back to `pretty`. `addLogSink(consoleSink(logFormat))` then registers the sink on
   the process-wide `LoggerContext`. On the `run` path its real consumers are the `cli` logger and,
   in wizard mode, `UiServer`'s `getLogger("ui")` and `setup-api`'s `getLogger("setup")` — the first
   record of a wizard run is `[ui]`, not `[cli]`. The keeper command uses it too. `init` does not:
   nothing under `src/cli/init/**` resolves a logger, and it spawns `simplex run` as a child anyway.
2. In json mode only, an `error` listener goes on `process.stdout`. pino-pretty's `pump()` chain
   installs one on whatever destination it is given (0 listeners before `prettyStream(...)`, 2 after);
   a bare stream has none, and without it the first write after a reader disappears is an unhandled
   `error` event that kills the process.
3. `consoleSink(format)` returns `process.stdout` for `json` and a `pino-pretty` transform for
   `pretty`. The transform is built with `destination: process.stdout`, not `.pipe()` — piped, it
   echoes each record's raw NDJSON next to the formatted line.
4. `program.parse(process.argv)` runs. Both flags are declared in `addRunOptions` (`src/cli/run-options.ts`),
   alongside the rest of `run`'s options. `--log-format` uses `.choices()`, so an unknown value exits
   here with the allowed values and the action never runs. `--no-open` is a commander negated boolean:
   `options.open` is `true` unless the flag is present.
5. The `run` action writes `ASCII_HEADER` to stdout only when `logFormat === "pretty"`.
6. The config branch. With a config present, `startFiller` is called and passes `consoleSink(logFormat)`
   as `SimplexOptions.logger`, so the filler's own `LoggerContext` gets a second sink. On the pretty
   path that is a second pino-pretty transform, which is deliberate: two pino instances sharing one
   transform interleave their chunks. On the json path both sinks are the same `process.stdout`, which
   is safe because nothing is reassembling anything. The flow ends there — the filler and, if enabled,
   the UI and tunnel start, and everything after is logged through those sinks. (`startFiller` is
   *defined* earlier in the action, but only called here or from the wizard's `onSaveAndStart`, so on
   the no-config path there is no second sink until the operator saves.)
7. With no config and `--no-ui`, the CLI writes "No config found (looked for ./filler-config.toml…)"
   to stderr and exits 1. No wizard, no URL, no browser — and in pretty mode a banner already on
   stdout with nothing after it.
8. Otherwise the wizard binds (falling back to an ephemeral port if the preferred one is taken) and the
   URL is reported: `console.log` with the surrounding blank lines in pretty mode, or
   `logger.info({ url }, ...)` on the `cli` module in json mode.
9. `openBrowser(url)` runs unless `--no-open` was passed. It is best-effort in any case — a failed
   spawn logs "Could not open a browser" and the printed URL is the fallback.

So stdout in pretty mode is: banner, then colourised single-line records, then the wizard URL block. In
json mode every line simplex itself writes is one JSON object, with the wizard URL among the records.

The one writer outside simplex's control is `@polkadot/util`'s logger, which routes `log` to
`console.log`; `@polkadot/api`'s `Init.js` uses it to announce "Runtime version updated to spec=…".
`bin/quiet.ts` patches only `console.warn`, so that line would land on stdout as plain text. It fires
only on a genuine Hyperbridge runtime upgrade mid-run, and never on the wizard path, which builds no
`ApiPromise` — but a json-mode consumer should tolerate a non-JSON line rather than assume none exists.
