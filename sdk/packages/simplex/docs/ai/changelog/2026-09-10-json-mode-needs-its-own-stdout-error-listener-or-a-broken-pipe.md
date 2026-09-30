# 2026-09-10 — json mode needs its own stdout `error` listener, or a broken pipe kills the filler (#1237)

Found by auditing the branch that added `--log-format json`, and fixed on it before merge.

`consoleSink()` returns a bare `process.stdout` in json mode. What that also removed, without anyone
noticing, was error handling: pino-pretty's `build()` ends in `pump(source, stream, destination)`, and
pump attaches `error` listeners to the destination — `process.stdout.listenerCount("error")` goes from
0 to 2 the moment `prettyStream({destination: process.stdout})` is constructed. A stream with no
listener turns a failed write into an unhandled `error` event, which Node turns into an uncaught
exception.

The failure lands precisely on the case the flag was added for. Spawning the built CLI detached with
`stdio: ["ignore","pipe","pipe"]` and then destroying the read end — the desktop app quitting while
the filler it spawned keeps running — killed it with exit 1 and `Unhandled 'error' event`, 3/3 runs.
The same invocation without `--log-format json` survived, 3/3. `simplex run --log-format json | head`
is the same failure at a terminal.

Fix: `if (logFormat === "json") process.stdout.on("error", () => {})`, once at module scope beside the
`logFormat` constant, because `consoleSink()` is called per writer. Swallowing matches the pretty path,
where such an error tears down pump's chain and logging goes quiet while the filler keeps filling —
`LoggerContext` already treats a broken sink as the host's problem rather than a reason to fail a fill.
Re-verified after the fix: json survives 3/3, and all five acceptance criteria still hold (`--no-open`
and `--log-format pretty` stdout byte-identical to a default run; json 2/2 valid lines, 0 ANSI bytes).

The audit also corrected three inaccurate claims in the previous entry's docs. `init` and
`validateConfig` were named as consumers of the process-wide sink; neither resolves a logger, and the
real wizard-path consumers are `UiServer`'s `getLogger("ui")` and `setup-api`'s `getLogger("setup")`.
The Flow.md walk placed the filler's second sink before the config branch, when `startFiller` is only
*defined* there and called inside `if (configPath)`, and it omitted the `--no-ui` + no-config exit.
And "every line of stdout is one JSON object" is true of every writer this package controls but not of
the process: `@polkadot/util`'s logger routes `log` to `console.log`, and `bin/quiet.ts` patches only
`console.warn`, so a Hyperbridge runtime upgrade mid-run would print one plain-text line. The claims
are now scoped to what simplex itself writes.

CI ran none of this. `.github/workflows/test-sdk.yml` gated simplex on `test:filler`, which pins four
network-backed files; `src/tests/cli` and `src/tests/logger.test.ts` were gated by nothing, so the
flags' own 15 tests never ran on a PR. Added a `test:unit` script over those two paths and a CI step
ahead of `test:filler`. It is network-free and takes ~20s.

Files: src/bin/simplex.ts, package.json (test:unit), .github/workflows/test-sdk.yml,
docs/ai/Decisions.md, docs/ai/Flow.md, README.md.
