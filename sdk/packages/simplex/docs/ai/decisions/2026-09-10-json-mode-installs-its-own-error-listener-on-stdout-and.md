# 2026-09-10 — json mode installs its own `error` listener on stdout, and swallows (#1237)

Decided: when the format is `json`, `bin/simplex.ts` registers `process.stdout.on("error", () => {})`
once at module scope, next to where `logFormat` is resolved.

Why it is needed at all: the thing pino-pretty was providing was not only formatting. Its `build()`
ends in `pump(source, stream, destination)`, and pump attaches error handling to the destination —
measurably, `process.stdout.listenerCount("error")` goes from 0 to 2 the moment
`prettyStream({destination: process.stdout})` is constructed. json mode returns the bare stream, so it
dropped those listeners along with the transform. A stream with no `error` listener turns the first
failed write into an unhandled `error` event, and Node turns that into an uncaught exception.

That is not theoretical, and it lands exactly on the case the flag exists for. Spawned the built CLI
detached with `stdio: ["ignore", "pipe", "pipe"]`, let it bind, then destroyed the read end — the
parent going away while the filler keeps running, which is the whole point of the detached child in
[#1237]. `--log-format json` died with exit 1 and `Unhandled 'error' event`, 3/3. The same run without
the flag survived, 3/3. `simplex run --log-format json | head` is the same failure in a terminal.

Why swallow rather than re-throw non-EPIPE errors: parity with the path this replaces. On the pretty
path an stdout error tears down pump's chain and logging simply goes quiet; the filler keeps filling.
`LoggerContext`'s destination already states the principle — "A broken sink is the host's problem, not
a reason to fail a fill". Re-throwing anything other than EPIPE would make json mode strictly more
fragile than pretty, which inverts the point of the flag.

Rejected: registering the listener inside `consoleSink()`. It is called once per writer — the
process-wide sink and again per filler — so that adds a duplicate listener per Simplex and drifts
toward Node's max-listeners warning. The format is a module-level constant; the listener belongs with it.

Rejected: `process.stdout.on("error")` unconditionally, for both formats. On the pretty path pino-pretty
already installs its own, and adding a second changes existing behaviour for every current user — which
the "default behaviour is completely unchanged" requirement rules out.
