# Operator state on disk

`SqliteDataStore` opens `bids.db` and `activity.db` in the data directory and hands `bids.db` to
both `SqliteBidStore` and `SqliteStateStore`. The state store keeps one `runtime_state` row per
`RuntimeState` key (`paused`, `phantomBids`), value JSON-encoded.

- `set` replaces every row inside one transaction.
- `patch` upserts only the keys it was given, and a key set to `undefined` deletes its row.
- `patchRuntimeState` calls `patch` when the store has one, and otherwise reads and writes the
  whole record back, which is what `MemoryDataStore` gets.

`node:sqlite` has no transaction helper, so `write` brackets its statements with `BEGIN`/`COMMIT`
by hand and rolls back on a throw. Without it a `set` could be observed with the old rows deleted
and the new ones not yet written.

On construction the store imports the `runtime-state.json` earlier versions wrote — first from the
data directory, then from `.filler-data/` relative to the process's cwd — but only when
`runtime_state` is empty, so it can never overwrite state the database already holds. Both copies
are then deleted: one left behind is read again by the next empty database, resurrecting a pause
the operator has since lifted.

Writes are wrapped so a failure logs instead of throwing: a pause that cannot be persisted still
pauses the filler. Reads are not, so a database that cannot be read fails `bootFiller` rather than
starting a filler the operator had paused.
