# 2026-09-09 — `--no-open` and `--log-format` are flags only, with no env-var equivalent (#1237)

Decided: neither flag has a `SIMPLEX_*` counterpart. The consumer in #1237 is Electron's `spawn`,
which builds argv directly, so there is nothing to make easier. Docker is the other supervisor, and it
already passes CLI arguments through `CMD` — the image's default command carries `--ui` for the same
reason.

Rejected: adding `SIMPLEX_NO_OPEN` / `SIMPLEX_LOG_FORMAT` alongside. Two ways to say one thing needs a
precedence rule, and that rule needs a test and a place in the docs, all for a caller that does not
exist yet. `SIMPLEX_HOME` is the package's one env var and it earns its place: config discovery has to
work before any argument is parsed. These do not have that problem — `--log-format` is read early, but
it is read from argv, which is available just as early.

If a supervisor that cannot construct argv does turn up, adding an env fallback inside
`logFormatFromArgv` is a couple of lines and breaks nothing.
