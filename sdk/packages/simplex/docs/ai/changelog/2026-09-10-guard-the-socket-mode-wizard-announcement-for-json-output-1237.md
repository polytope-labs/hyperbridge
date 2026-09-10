# 2026-09-10 — Guard the socket-mode wizard announcement for json output (#1237)

Rebasing #1237 onto #1245 (Unix domain socket listen mode) landed the exact collision the audit of
this branch predicted. #1245 added an unguarded

    console.log(`\n  No config found — the setup wizard is serving on ${uiSocket}\n`)

on the socket wizard path. Git merged it without complaint — it sits about twenty lines from the
guarded TCP announcement, so the resolver got no signal — and `--ui-socket <path> --log-format json`
would then emit one plain-text line into a stream a supervisor is parsing as NDJSON. That is the exact
promise `--log-format json` exists to make.

Guarded it the same way as its TCP sibling: a `cli` record carrying a `socket` field in json mode,
the prose line otherwise.

Worth noting for the next merge: this is now the second announcement on the wizard path, and both are
easy to add a third alongside without noticing the rule. A `console.log` reaching stdout anywhere in
`run` breaks json mode, and nothing in the suite catches it.

Files: src/bin/simplex.ts.
