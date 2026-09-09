/**
 * Regenerates the `src/tests/data/fixtures/legacy-v0` databases used by
 * `sqlite-compat.test.ts`.
 *
 * The fixtures stand in for an operator's data directory written by a release
 * that still used better-sqlite3, at the schema that shipped *before* the
 * in-place column migrations (`dead`/`pending` on bids, `order_json` on events,
 * `token_in`/`amount_in` on wallet_txs). Committing the real files is the point:
 * they prove the node:sqlite store opens a database another driver wrote and
 * migrates it, which a database this store created itself could never prove.
 *
 * better-sqlite3 is no longer a dependency, so this script is not wired into any
 * package script — regenerating means installing it by hand:
 *
 *   pnpm add -D better-sqlite3 && node scripts/make-legacy-db-fixture.mjs
 *
 * The DDL below is copied verbatim from the commits that introduced it
 * (9149fc52 for bids, b3af77e3 for events/wallet_txs) and must not be
 * "modernised" — its whole value is being what old installs actually have.
 */
import Database from "better-sqlite3"
import { mkdirSync, rmSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const OUT = join(dirname(fileURLToPath(import.meta.url)), "..", "src", "tests", "data", "fixtures", "legacy-v0")
rmSync(OUT, { recursive: true, force: true })
mkdirSync(OUT, { recursive: true })

const bids = new Database(join(OUT, "bids.db"))
bids.exec(`
	CREATE TABLE IF NOT EXISTS bids (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		commitment TEXT NOT NULL,
		extrinsic_hash TEXT,
		block_hash TEXT,
		success INTEGER NOT NULL,
		error TEXT,
		created_at TEXT NOT NULL DEFAULT (datetime('now')),
		retracted INTEGER NOT NULL DEFAULT 0,
		retracted_at TEXT,
		retract_extrinsic_hash TEXT
	);

	CREATE INDEX IF NOT EXISTS idx_bids_commitment ON bids(commitment);
	CREATE INDEX IF NOT EXISTS idx_bids_success ON bids(success);
	CREATE INDEX IF NOT EXISTS idx_bids_retracted ON bids(retracted);
	CREATE INDEX IF NOT EXISTS idx_bids_created_at ON bids(created_at);
`)
const insertBid = bids.prepare(`
	INSERT INTO bids (commitment, extrinsic_hash, block_hash, success, error, created_at, retracted, retracted_at, retract_extrinsic_hash)
	VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
`)
// A won bid still holding a deposit — the row the retraction sweep must still find.
insertBid.run(`0x${"aa".repeat(32)}`, `0x${"11".repeat(32)}`, `0x${"22".repeat(32)}`, 1, null, "2025-01-02 03:04:05", 0, null, null)
// A bid already retracted, so the sweep must leave it alone.
insertBid.run(`0x${"bb".repeat(32)}`, `0x${"33".repeat(32)}`, `0x${"44".repeat(32)}`, 1, null, "2025-01-02 03:05:05", 1, "2025-01-02 04:00:00", `0x${"55".repeat(32)}`)
// A failed bid, which never held a deposit.
insertBid.run(`0x${"cc".repeat(32)}`, null, null, 0, "insufficient balance", "2025-01-02 03:06:05", 0, null, null)
bids.close()

// WAL is what the store sets on activity.db, and it is persisted in the file
// header — so a realistic fixture has to carry it.
const activity = new Database(join(OUT, "activity.db"))
activity.pragma("journal_mode = WAL")
activity.exec(`
	CREATE TABLE IF NOT EXISTS events (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		ts INTEGER NOT NULL,
		type TEXT NOT NULL,
		order_id TEXT,
		chain_id INTEGER,
		strategy TEXT,
		success INTEGER,
		reason TEXT,
		volume_usd REAL,
		profit_usd REAL,
		tx_hash TEXT
	);
	CREATE INDEX IF NOT EXISTS idx_events_id ON events(id);
	CREATE TABLE IF NOT EXISTS wallet_txs (
		id INTEGER PRIMARY KEY AUTOINCREMENT,
		ts INTEGER NOT NULL,
		kind TEXT NOT NULL,
		chain_id INTEGER,
		token TEXT,
		amount TEXT,
		to_address TEXT,
		tx_hash TEXT NOT NULL,
		sponsored INTEGER
	);
`)
const insertEvent = activity.prepare(`
	INSERT INTO events (ts, type, order_id, chain_id, strategy, success, reason, volume_usd, profit_usd, tx_hash)
	VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
`)
insertEvent.run(1735786800000, "bid", "0xorder1", 1, "basic", 1, null, 250.5, 1.25, `0x${"66".repeat(32)}`)
insertEvent.run(1735786900000, "filled", "0xorder1", 42161, "basic", 1, null, null, null, `0x${"77".repeat(32)}`)
insertEvent.run(1735787000000, "lost", "0xorder2", 8453, "fx", 0, "outbid", null, null, null)
activity
	.prepare(`INSERT INTO wallet_txs (ts, kind, chain_id, token, amount, to_address, tx_hash, sponsored) VALUES (?, ?, ?, ?, ?, ?, ?, ?)`)
	.run(1735787100000, "sweep", 1, null, null, null, `0x${"88".repeat(32)}`, 1)
activity.close()

console.log(`wrote legacy fixtures to ${OUT}`)
