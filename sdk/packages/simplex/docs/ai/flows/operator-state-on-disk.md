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
and the new ones not yet written. The rollback is guarded on `isTransaction !== false`, the same
shape as `SqliteActivityStore.attachOrder`: SQLite rolls a failed COMMIT back itself, and rolling
back again throws `cannot rollback - no transaction is active` over the error worth reading.

On construction the store imports the `runtime-state.json` earlier versions wrote — first from the
data directory, then from `.filler-data/` relative to the process's cwd — but only when
`runtime_state` is empty, so it can never overwrite state the database already holds. Every copy it
managed to read is then deleted, not just the one it imported: a copy left behind is read again by
the next empty database, resurrecting a pause the operator has since lifted.

A file it could not read is left alone. A missing file is silent, since that is the normal case;
any other failure — no permission, an I/O error, malformed JSON, or a parsed value that is not a
plain object — logs at warn and keeps the file, because it may still hold the pause and deleting it
would destroy the state the import exists to rescue.

Writes are wrapped so a failure logs instead of throwing: a pause that cannot be persisted still
pauses the filler. `patch` wraps its read-back too, returning the requested patch if the database
cannot be read — production reaches the store only through `patch`, and `UiServer` pauses the
filler before awaiting it, so a throw would report failure for a pause that had already happened.
`get` is deliberately unwrapped, so a database that cannot be read fails `bootFiller` rather than
starting a filler the operator had paused.

The two candidate paths are resolved to absolute and de-duplicated before any of this: with
`--data-dir .filler-data` they name the same file, and importing and unlinking it twice logged a
failure to delete a file that was already gone.
