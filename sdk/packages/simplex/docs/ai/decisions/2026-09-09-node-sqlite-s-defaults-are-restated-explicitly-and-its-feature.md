# 2026-09-09 — node:sqlite's defaults are restated explicitly, and its feature guards compare against `false` (#1236)

Decided: pass `{ timeout: 5000 }` to both `DatabaseSync` constructors, and write every
capability guard as `x === false` / `x !== false` rather than `!x` / `x`.

**The timeout, and why the PRAGMA rather than the constructor option.** `DatabaseSync` takes a
`timeout` option, and using it was the obvious move — but it only landed in v22.18.0 and v24.0.0,
so it is accepted and ignored on 22.16, 22.17 and every 23.x, which `engines.node` allows. A
reviewer measured 0.45ms to failure on 23.11 against the option-based version. `@types/node`
declares it `@since v22.16.0`, which is simply wrong; nodejs.org's version table is the source to
trust when the two disagree. `PRAGMA busy_timeout` is ordinary SQLite, so it applies on every
runtime that has `node:sqlite` at all, and it can be read back for a direct assertion instead of
inferred from a stopwatch.

Swapping libraries silently swaps defaults, and this one is load-bearing:
better-sqlite3 sets a 5000ms busy timeout unless told otherwise (`lib/database.js`:
`'timeout' in options ? options.timeout : 5000`), while `node:sqlite` leaves it at 0. Nothing in
the diff mentioned locking, which is exactly why it slipped through — the migration was audited
for return shapes and API equivalence, not for constructor defaults. 5000 is chosen to match what
the store already had, not because the number is special; the point is not to change locking
behaviour in a commit that is not about locking.

Rejected: catching `SQLITE_BUSY` and retrying in the store. That reimplements, worse, what
SQLite's own busy handler does in C, and it would have to be added at ten call sites.

Rejected: leaving it at 0 and treating contention as a caller problem. The callers are the fill
loop and the retraction sweep; neither has anywhere to put the error, and the bid store is the
record that makes a locked deposit reclaimable.

**The guards.** `db.isOpen` (Node 22.15+) and `db.isTransaction` (22.16+, and never backported to
the 23 line) are read to decide whether to close and whether to roll back. Written as `!db.isOpen`
and `if (db.isTransaction)`, a runtime *missing* the property takes the convenient branch: the
close is skipped for every database, and a failed transaction is never rolled back — which leaves
the connection inside an open transaction so nothing commits again for the life of the process.
Comparing against `false` inverts that: absent means "do the work", and the existing try/catch
absorbs the outcome. The rollback path additionally swallows a ROLLBACK error, because `err` — the
original failure — is what propagates either way, and losing it to `cannot rollback - no
transaction is active` would be strictly worse.

Rejected: relying on `engines.node` to make the missing-property case unreachable. `engines` is
advisory — npm prints `EBADENGINE` and installs anyway, pnpm likewise by default — so it documents
intent, it does not enforce it. A guard that is only correct when a manifest field is obeyed is
not a guard.
