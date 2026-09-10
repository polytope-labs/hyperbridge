# 2026-09-10 — `simplex --ui` with no value stopped erroring

`run` declared the UI bind as `--ui [<[host:]port>]`. The `[...]` was meant to make the value
optional, but commander derives the two properties independently from the flags string:
`this.required = flags.includes("<")` and `this.optional = flags.includes("[")`. The placeholder
`<[host:]port>` contains a `<`, so the option came out both required and optional, and required
wins. `simplex --ui` and `simplex run --ui` each died with `error: option '--ui [<[host:]port>]'
argument missing`, even though the flag was declared and documented as taking an optional value.

The placeholder is now `[host:port]`, which has no angle brackets. Nothing else about the option
changes — `--ui 9000`, `--ui 192.168.1.50:8686`, `--ui 0.0.0.0:8686`, `--no-ui` and passing nothing
at all parse exactly as they did, verified against the pinned commander 12.1.0 through the real
default-subcommand route. A valueless `--ui` now yields `true`, which the existing handler already
reads correctly: `uiEnabled = options.ui !== false` turns the UI on, and `typeof options.ui ===
"string"` is false, so the bind stays at the default `127.0.0.1:8686`. `simplex --ui` therefore
means the same as `simplex`, said out loud, and reads as the counterpart of `--no-ui`. The
description gained one clause — "a bare port keeps the host at 127.0.0.1" — because the placeholder
no longer carries that nuance.

The `run` flags moved into `src/cli/run-options.ts` so a test can parse the real declarations:
`src/bin/simplex.ts` calls `program.parse(process.argv)` at module load, so a test cannot import it.
`DEFAULT_UI_PORT` moved with them, and the action handler's inline options type — a hand-maintained
duplicate of the flags — is now `RunOptions`, declared beside the flags that produce it. The new
test fails against the old placeholder with `required: true`.

Found while adding `--ui-socket` (#1237), which deliberately left `--ui` alone: its acceptance
criteria required the CLI's existing TCP behaviour to be unchanged.

Files: src/cli/run-options.ts (new), src/bin/simplex.ts, src/tests/cli/run-options.test.ts (new),
and the flag table in the repo's docs/content/developers/evm/simplex/dashboard.mdx.
