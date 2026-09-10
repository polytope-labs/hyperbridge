# 2026-09-09 — `attachOrder`'s transaction is explicit BEGIN/COMMIT/ROLLBACK, guarded by `isTransaction` (#1236)

Decided: `node:sqlite` has no `db.transaction(fn)` wrapper, so the one call site that needed one
writes the boundary out:

```ts
this.db.exec("BEGIN")
try {
    ...
    this.db.exec("COMMIT")
} catch (err) {
    if (this.db.isTransaction) this.db.exec("ROLLBACK")
    throw err
}
```

The transaction is load-bearing, not decoration: `attachOrder` SELECTs the event ids that lack an
order summary and then UPDATEs them, and it returns those ids to the caller as the rows it
changed. Without one boundary around both, a concurrent writer could fill a row in between and the
returned ids would name rows this call did not touch.

Why the `isTransaction` guard rather than an unconditional ROLLBACK: SQLite rolls back
automatically when a COMMIT fails, and a ROLLBACK with no transaction open throws
`cannot rollback - no transaction is active` — which would replace the real error with a
misleading one on the way out. `isTransaction` is a thin wrapper over `sqlite3_get_autocommit()`
and answers exactly the question being asked. Note BEGIN sits *outside* the `try` on purpose: if
BEGIN itself throws there is nothing to roll back, and catching it would attempt one.

Rejected: `try { rollback } catch {}` — swallowing whatever the ROLLBACK throws. Same behaviour in
the common case, but it also hides a genuine rollback failure, which is exactly the failure worth
seeing. It would have avoided the 22.16.0 floor (see above); that was not worth trading a real
diagnostic for.

Rejected: reimplementing a generic `transaction(fn)` helper over BEGIN/COMMIT. There is one call
site. A helper would have to decide about nesting (`node:sqlite` throws on a nested BEGIN) and
savepoints for a case that does not exist here.
