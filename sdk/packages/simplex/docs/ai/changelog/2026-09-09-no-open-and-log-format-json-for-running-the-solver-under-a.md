# 2026-09-09 — `--no-open` and `--log-format json`, for running the solver under a supervisor (#1237)

The desktop app spawns `simplex` as a detached child and captures its stdout to a log file. Two things
the CLI did unconditionally get in the way of that, so each now has a flag on `run`.

`--no-open` suppresses the `openBrowser()` call on the setup-wizard path. The app renders the wizard in
its own window, so a system browser opening alongside it is wrong. Nothing else about that path changes:
the server still binds and the URL is still reported. This is not `--no-ui`, which turns the UI server
off entirely.

`--log-format <pretty|json>` chooses how this process's logs reach stdout, and defaults to `pretty`, so
a terminal user sees what they saw before. In `json` mode `consoleSink()` returns `process.stdout`
itself and pino's NDJSON goes out untouched — no pino-pretty, so no ANSI escapes in the captured file.
The ASCII banner and the wizard's plain-text URL line are the only other writers to stdout: in json mode
the banner is dropped and the URL becomes a `cli` record carrying a `url` field. Every line of stdout is
then one JSON object.

The format is read from raw argv by `logFormatFromArgv`, because the process-wide sink is registered
while `bin/simplex.ts` is still evaluating, before commander has parsed anything. Commander declares the
flag too, so `--help` lists it and an unknown value is rejected with the allowed choices.
`src/tests/cli/log-format.test.ts` pins the scan and checks it agrees with commander across the
invocations that reach the flag.

Verified against the built bundle in a directory with no config. `--no-open` produced stdout
byte-identical to a default run and launched no browser (checked with a stub `xdg-open` on `PATH`).
`--log-format json` produced two records, both parsing as JSON, with zero ANSI escapes and no banner.
`--log-format pretty` was identical to passing nothing. A separate check ran two `LoggerContext`s into
one shared `process.stdout` for 4000 records and found no interleaved or partial lines.

Rebased onto #1249, which split `run`'s option declarations into `addRunOptions`
(`src/cli/run-options.ts`). Both flags moved there, and `open` joined its `RunOptions` interface;
`logFormat` deliberately did not. The test now parses the real builder instead of a hand-copied mirror,
which is what makes the `--ui`-swallows-the-next-flag case (fixed by #1249) testable here at all — the
suite went from 15 to 22 tests, including the first coverage `--no-open` has had.

Files: src/bin/simplex.ts, src/cli/run-options.ts, src/cli/log-format.ts (new),
src/tests/cli/log-format.test.ts (new), README.md.

No version bump. Four simplex branches were open against #1237 at once and a bump in each would have
collided on the same field for nothing — publishing is tag-driven, so the version can be set once on
whatever lands. This is the exception to the usual "bump inside the feature PR" rule, not a change to it.
