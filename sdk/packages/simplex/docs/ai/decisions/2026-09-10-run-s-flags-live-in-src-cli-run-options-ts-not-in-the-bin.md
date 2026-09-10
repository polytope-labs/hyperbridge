# 2026-09-10 — `run`'s flags live in src/cli/run-options.ts, not in the bin

Decided: the option declarations for `simplex run` sit in their own module, and `src/bin/simplex.ts`
applies them with `addRunOptions(program.command("run", { isDefault: true }))`.

Why: the bin parses `process.argv` at module load, so importing it from a test runs the CLI. Testing
the flags meant either copying the flag strings into the test — where they would have kept passing
after someone edited the real declaration back — or putting them somewhere importable. The `--ui`
bug this now guards survived precisely because nothing ever parsed the CLI's own flags.

Rejected: extracting the whole program into a `buildProgram()`. It reindents ~280 lines of action
handlers for a fix that changes one string, and those handler bodies are the part a unit test cannot
exercise anyway.

Rejected: leaving the declarations in the bin and asserting on a `new Option("--ui [host:port]")`
built inside the test. That tests commander, not simplex, and stays green when the bin regresses.
