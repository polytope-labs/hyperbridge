# 2026-09-09 — Audit follow-ups on the node:sqlite migration (#1236)

A multi-lens adversarial audit of the migration commit turned up one real regression and a set of
smaller defects. All fixed here.

**The regression: the busy timeout was silently lost.** better-sqlite3 defaults `timeout` to
5000ms; `node:sqlite` defaults it to 0. `new DatabaseSync(path)` with no options therefore turned
every lock contention into an instant `SQLITE_BUSY` throw where the old store waited five seconds.
Anything holding either file for a moment — an operator running `sqlite3 bids.db`, a backup, a
second process on the same `--data-dir` — would have failed the write racing it, and a dropped bid
write is a deposit the retraction sweep can no longer find. Both databases now pass
`PRAGMA busy_timeout = 5000`. Verified empirically: it reads 0 on a plain handle, and a contended
write now blocks ~5s before failing instead of ~28ms.

The first fix used `DatabaseSync`'s `timeout` constructor option and was wrong — caught in review
(#1247). That option was added in v22.18.0 and v24.0.0, so it is *silently ignored* on 22.16,
22.17 and the entire 23 line, all of which `engines.node` admits; on Node 23.11 the contended write
still failed in 0.45ms. Note `@types/node` marks it `@since v22.16.0`, which is what the first fix
trusted; the version table on nodejs.org is authoritative and disagrees. The PRAGMA is plain
SQLite and works wherever `node:sqlite` does. Two tests cover it: one reads `busy_timeout` back off
the store's own connections (fast, and runtime-independent — the option-based version passes the
behavioural test on a lucky runtime), one holds a real write lock from a second connection.

**Two guards failed unsafe on runtimes below the engines floor.** `if (!db.isOpen) continue` skips
the close entirely when the property is missing (it landed in 22.15), and
`if (this.db.isTransaction)` skips the ROLLBACK when *that* property is missing — the Node 23 line
never received it — leaving the connection wedged mid-transaction so no later write commits. Both
now compare against `false`, so a runtime lacking the property does the safe thing instead of the
convenient one. `engines` is advisory in npm and pnpm, so "our floor forbids it" is not a guard.

**The compatibility suite was not wired into CI.** `test-sdk.yml` ran only `test:filler`, which
names four specific files, none under `src/tests/data` — so the safety net for operator databases
would never have run on a PR. Added a `test:data` script and a CI step; it needs no network or
secrets and takes seconds.

Also: the CLI's static store import made `--help`, `--version` and `init` emit Node's SQLite
`ExperimentalWarning` on every run (Node 24.19 has dropped it, but 22.16–24.1x still emit it, and
the lazy import this migration deleted used to avoid it) — the bin shebang and the Docker
ENTRYPOINT now pass `--disable-warning=ExperimentalWarning`. The fixture generator's
`rmSync(OUT, { recursive: true })` deleted the `.gitignore` whose `!*.db` is what re-includes the
fixtures past the repo-wide `**/*.db` rule; it now removes only the database files.
`docs/.../sdk/simplex.mdx` still told readers "Node.js 22 or later" directly under the install
snippet, contradicting both `engines.node` and its own SQLite paragraph. `Decisions.md` claimed
tsup 9 had shipped and reversed the `removeNodeProtocol` default — it has not; 8.5.1 is latest.
A comment in `publish-simplex.yml` still gave "a native sqlite module" as a reason the arm64 image
needs a native runner.

Files: src/data/sqlite/index.ts, src/data/sqlite/activity.ts, src/bin/simplex.ts,
src/tests/data/sqlite-compat.test.ts, scripts/Dockerfile, scripts/make-legacy-db-fixture.mjs,
package.json, docs/ai/Decisions.md, ../../../.github/workflows/test-sdk.yml,
../../../.github/workflows/publish-simplex.yml, ../../../docs/content/developers/sdk/simplex.mdx.
