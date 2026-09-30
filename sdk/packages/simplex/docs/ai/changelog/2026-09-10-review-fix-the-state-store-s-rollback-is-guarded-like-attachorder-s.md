# 2026-09-10 — Review fix: the state store's ROLLBACK is guarded like `attachOrder`'s

`SqliteStateStore.write` brackets its statements with `BEGIN`/`COMMIT` by hand, because
`node:sqlite` has no transaction helper — but it rolled back unconditionally, which is the shape
`2026-09-09-attachorder-s-transaction-is-explicit-begin-commit-rollback.md` rejected. SQLite rolls
a failed COMMIT back on its own, so `ROLLBACK` from the catch block throws
`cannot rollback - no transaction is active` before `throw err` is reached, and the real cause is
discarded.

That cost more here than it would have at the other call site. Through `set` and `patch` the error
only ever reaches `persist`, which logs it: an operator whose pause will not survive a restart read
`Could not persist operator state: cannot rollback - no transaction is active` instead of the
`SQLITE_FULL` or `SQLITE_IOERR` that named the actual problem. Through `importRetiredStateFile` the
`write` is deliberately outside `persist` and `openDataStore` no longer wraps store construction,
so a COMMIT that failed during the one-time migration aborted boot reporting the rollback rather
than the import.

Now the same guarded form as `SqliteActivityStore.attachOrder`: `isTransaction !== false` — the
comparison, not truthiness, because the Node 23 line never got the property and `engines` is
advisory — with the ROLLBACK in its own catch for the runtime that cannot answer the question.

One test, failing against the previous behaviour: a `DatabaseSync` proxy whose COMMIT rolls back
and then throws, as SQLite does, asserting the logged record carries the real error and not the
rollback one.

Files: `src/data/sqlite/state.ts`, `src/tests/data/state-store.test.ts`.
