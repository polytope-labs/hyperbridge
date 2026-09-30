# 2026-09-09 — Both tsup entries set `removeNodeProtocol: false`, and the build greps for it (#1236)

Decided: turn tsup's `node:`-prefix stripping off, and verify the bundle afterwards in
`scripts/build.sh`.

tsup 8 rewrites `import ... from "node:sqlite"` to `from "sqlite"` by default. The flip to `false`
is announced for the next major but has not shipped — 8.5.1 is the latest release — so this is a
default to live with, not something an upgrade clears today. The rewrite is
harmless for most builtins because `fs`, `path` and friends resolve with or without the prefix.
`node:sqlite` does not: there is no bare `sqlite` builtin, so the import fails with
`ERR_MODULE_NOT_FOUND: Cannot find package 'sqlite'` the moment the process starts. Both the CLI
bundle and the `@hyperbridge/simplex/sqlite` library entry were broken this way, i.e. it would
have shipped.

Why a grep in the build rather than a test: this failure only exists *after* bundling. The vitest
suite imports from source, where nothing rewrites specifiers, so every test can pass against a
bundle that cannot start — which is exactly what happened here (25/25 green, image dead on
`--version`). The check runs inside `pnpm build`, so the Docker image and any release build both
fail loudly instead of shipping.

Rejected: setting an esbuild `target` of `node22`. The stripping is tsup's own plugin, not
esbuild's — plain esbuild at `--target=es2020` preserves the prefix — so a target change would not
have fixed it, and would have silently changed downleveling for the whole bundle.

Rejected: importing `sqlite` through `createRequire` to sidestep specifier rewriting. It defeats
the rewrite but replaces a plain static import with indirection that no longer type-checks
naturally, to work around a bundler default that upstream has already agreed to reverse.

Rejected: relying on the Docker build alone to catch it. That is what caught it this time, but
only because the image was actually run; a build that merely succeeds proves nothing here, and
`pnpm build` is the narrower place to assert the invariant.
