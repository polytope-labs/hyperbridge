# 2026-09-10 — The second hand-rolled transaction is copied, not extracted into a helper

Decided: `SqliteStateStore.write` repeats the `BEGIN`/`COMMIT`/guarded-`ROLLBACK` block from
`SqliteActivityStore.attachOrder` verbatim rather than either simplifying it or factoring the two
into a shared `transaction(fn)`.

`2026-09-09-attachorder-s-transaction-is-explicit-begin-commit-rollback.md` rejected a helper
partly because "there is one call site". That is no longer true — this is the second — so the
question was worth asking again, and the answer is still no. A helper would have to decide what to
do about nesting, which `node:sqlite` rejects, and about savepoints; neither call site nests, so it
would be designing for a case that does not exist. Two call sites is also exactly where a helper is
least valuable and most likely to be built for the wrong shape.

What is *not* optional is the guard. Both halves of it are load-bearing and neither is obvious
from reading the block:

- `isTransaction !== false`, not `if (isTransaction)`. SQLite rolls a failed COMMIT back itself, so
  an unconditional ROLLBACK throws `cannot rollback - no transaction is active` over the real
  cause. But the property landed after the `engines.node` floor and the Node 23 line never got it,
  and `engines` is advisory in npm and pnpm — so truthiness would skip the ROLLBACK on a runtime
  that is admitted, wedging the connection mid-transaction.
- The inner `catch {}` covers only that runtime: where `isTransaction` cannot answer, the ROLLBACK
  is attempted anyway and its own failure must not displace `err` either.

Copying it whole is what keeps those two properties together. Simplifying either of them at this
call site would have reintroduced a defect the other call site already paid for in review.

Rejected: leaving `write` unwrapped and letting `persist` swallow whatever arrives. `persist` does
swallow it, but it logs it first, and the log is the only thing an operator has when a pause does
not survive a restart. `importRetiredStateFile` does not go through `persist` at all.
