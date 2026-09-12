# Operator state moved into `bids.db`; `runtime-state.json` retired

`FileStateStore` is now `SqliteStateStore`, backed by a `runtime_state` table in `bids.db` — the
database `SqliteBidStore` already holds open — with one row per `RuntimeState` key and the value
JSON-encoded. `SqliteDataStore` passes it the bids connection and the data directory.

Two bugs go with the file. Its `writeFileSync` truncated before writing, so a crash mid-write left
JSON that would not parse and `get()` fell back to `{}`, silently resuming a paused filler and
stranding the phantom deposit the file existed to reclaim. And every writer went through
`patchRuntimeState`'s read-merge-write, so the phantom batch and an operator pause could drop one
another's key. `StateStore` gained an optional `patch`, which the SQLite store implements as an
upsert of only the named keys inside one transaction; `patchRuntimeState` uses it when present and
otherwise keeps the old get-then-set, which is what `MemoryDataStore` gets. A key set to
`undefined` deletes its row. `node:sqlite` has no transaction helper, so `write` brackets its
statements with `BEGIN`/`COMMIT` and rolls back on a throw.

Existing data directories are migrated on open: if `runtime_state` is empty the store reads
`runtime-state.json` from the data directory, then from `.filler-data/` relative to cwd, writes
what it finds, and unlinks the copies it read. Guarding on an empty table means it can never
overwrite state the database already holds; deleting every readable copy — not just the imported
one — means an empty database cannot resurrect a pause the operator has since lifted.

Only files that were read back are deleted, which the first draft got wrong: it unlinked both paths
whether or not the read succeeded, so a `runtime-state.json` that existed but could not be read
(no permission, an I/O error, malformed JSON) was destroyed and its pause and phantom bids lost —
the exact state the import exists to rescue. Found in review by @royvardhan, who reproduced it with
a mode-000 file holding a real pause. A missing file is still silent, since that is the normal
case; every other read failure logs and leaves the file where it is. A parsed value that is not a
plain object counts as a failed read for the same reason.

Writes still swallow their errors (logged now, which the file store could not do) so a pause that
cannot be persisted still pauses the filler; reads no longer do, so an unreadable database fails
`bootFiller` instead of starting a filler that was paused.

No version bump here. The `FileStateStore` export from `@hyperbridge/simplex/sqlite` is gone and
the on-disk format changed, so whichever release picks this up owes at least a minor.

Files: `src/data/sqlite/state.ts`, `src/data/sqlite/index.ts`, `src/data/state.ts`,
`src/data/types.ts`, `src/tests/data/state-store.test.ts`,
`docs/ai/flows/operator-state-on-disk.md`,
`docs/ai/flows/opening-an-operator-data-directory.md`,
`docs/ai/flows/phantom-bid-deposits-across-restarts.md`.
