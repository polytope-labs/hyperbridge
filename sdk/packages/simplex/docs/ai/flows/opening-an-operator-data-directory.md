# Opening an operator data directory

Read from the source and exercised by `src/tests/data/sqlite-compat.test.ts` against real
better-sqlite3-written databases on 2026-09-09.

1. `bin/simplex.ts` `startFiller` calls `openDataStore(options.dataDir)`, which is
   `new SqliteDataStore(resolveDataDir(dataDir))` — the directory defaults to
   `.simplex-data` under the working directory. There is no memory fallback and no
   error translation: the CLI submits bids, and bid rows are how locked deposits are
   found again, so a store that cannot open is fatal.
2. `SqliteDataStore` creates the directory if absent, then opens two separate files with
   `node:sqlite`'s `DatabaseSync`: `bids.db` and `activity.db`. They are separate so a data
   directory written by an earlier version is picked up unchanged. `activity.db` gets
   `PRAGMA journal_mode = WAL`; that setting lives in the file header, so an existing
   database already carries it and a new one keeps it from here on. `bids.db` is left on
   the default rollback journal.
3. `SqliteBidStore`'s constructor runs `CREATE TABLE IF NOT EXISTS bids (...)` plus its four
   indexes — a no-op on an existing file, since `IF NOT EXISTS` will not reshape a table that
   is already there. It then reads `columnNames(db, "bids")` and `ALTER TABLE ... ADD COLUMN`s
   any of `dead`, `pending` that are missing, each `INTEGER NOT NULL DEFAULT 0`. This is the
   only thing that upgrades a pre-#1074 database, and it runs on every open.
4. `SqliteActivityStore`'s constructor does the same for `events` and `wallet_txs`: create if
   absent, then add `order_json TEXT` to `events` and `token_in`/`amount_in TEXT` to
   `wallet_txs` when `columnNames` says they are missing. Rows written before those columns
   existed therefore read back with `order: null` and `tokenIn`/`amountIn: null`.
5. `FileStateStore` takes the same directory for `runtime-state.json`. It is plain JSON on
   disk, not SQLite, and is unaffected by any of the above.
6. Who closes it depends on who opened it. `Simplex.start` sets `ownsData: !options.data`, and
   `core/boot.ts` closes the store on shutdown only `if (options.ownsData)` — so a store the
   caller handed in is the caller's to close, since it may be shared with another solver. The
   CLI is exactly that case: it passes `data: dataStore`, so `Simplex.stop()` leaves the
   databases open and `bin/simplex.ts` closes them itself on the way out. A store `Simplex`
   defaulted for itself is the owned case and does close during `stop()`. Either path skips a
   database whose `isOpen` is already false, because `DatabaseSync.close()` throws on a second
   call where better-sqlite3's was a no-op. Nothing registers a `process.on("exit")` hook — an
   earlier version did, which leaked a listener per instance and made two fillers in one
   process race each other's teardown. Both files are crash-safe unclosed.

The migrations are additive and idempotent: nothing drops or rewrites a column, so reopening a
directory that has already been migrated changes nothing, and an older binary pointed at a
migrated directory still reads it (it just ignores the extra columns).
