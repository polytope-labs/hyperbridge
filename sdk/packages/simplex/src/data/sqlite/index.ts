import { existsSync, mkdirSync } from "node:fs"
import { join } from "node:path"
import { DatabaseSync } from "node:sqlite"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { ActivityStore, BidStore, SimplexDataStore, StateStore } from "@/data/types"
import { SqliteActivityStore } from "./activity"
import { SqliteBidStore } from "./bids"
import { SqliteStateStore } from "./state"

export { SqliteActivityStore } from "./activity"
export { SqliteBidStore } from "./bids"
export { SqliteStateStore } from "./state"

/**
 * How long a write waits for another connection's lock before giving up.
 * Matches the default better-sqlite3 applied for us; `node:sqlite` defaults to 0,
 * which turns any momentary contention into an instant SQLITE_BUSY throw — and a
 * lost bid write is a deposit the retraction sweep can no longer find.
 *
 * Set through the PRAGMA rather than `DatabaseSync`'s `timeout` option on purpose.
 * That option only exists in Node >=22.18 and >=24 (`@types/node` says 22.16, which
 * is wrong — the version table on nodejs.org is authoritative), so it is silently
 * ignored on 22.16, 22.17 and the whole 23 line, all of which `engines.node`
 * admits. The PRAGMA is plain SQLite and works wherever `node:sqlite` does.
 */
const BUSY_TIMEOUT_MS = 5_000

/**
 * File-backed {@link SimplexDataStore} — the durable default, and what the
 * `simplex` CLI uses.
 *
 * Keeps bids and activity in separate database files (`bids.db`,
 * `activity.db`) so an existing data directory written by an earlier version is
 * picked up unchanged. Operator state rides in `bids.db` beside the bids; a
 * `runtime-state.json` from before that is imported once and deleted.
 *
 * Built on `node:sqlite`, so there is nothing to install and nothing to
 * compile — the engine ships inside the Node runtime. That is why the package
 * has no native dependency and why `engines.node` names a version new enough to
 * have `node:sqlite` unflagged.
 */
export class SqliteDataStore implements SimplexDataStore {
	readonly bids: BidStore
	readonly activity: ActivityStore
	readonly state: StateStore

	private databases: DatabaseSync[]
	private logger: Logger

	constructor(dataDir: string, loggers: LoggerContext = defaultLoggerContext()) {
		this.logger = loggers.get("data-store")
		if (!existsSync(dataDir)) mkdirSync(dataDir, { recursive: true })

		const bidsDb = new DatabaseSync(join(dataDir, "bids.db"))
		const activityDb = new DatabaseSync(join(dataDir, "activity.db"))
		// `node:sqlite` has no .pragma(); exec() discards the rows these return.
		for (const db of [bidsDb, activityDb]) db.exec(`PRAGMA busy_timeout = ${BUSY_TIMEOUT_MS}`)
		activityDb.exec("PRAGMA journal_mode = WAL")

		this.databases = [bidsDb, activityDb]
		this.bids = new SqliteBidStore(bidsDb, loggers)
		this.activity = new SqliteActivityStore(activityDb, loggers)
		this.state = new SqliteStateStore(bidsDb, dataDir, loggers)

		this.logger.info({ dataDir }, "SQLite data store opened")
	}

	/**
	 * Closes both databases. Called by `Simplex.stop()` when Simplex opened the
	 * store itself, and by whoever passed it in otherwise — `bootFiller` only
	 * closes a store it owns, since a caller's may be shared with another solver.
	 *
	 * Deliberately not wired to a `process.on("exit")` hook: the old services did
	 * that in their constructors, which leaked a listener per instance and made
	 * two fillers in one process race each other's teardown. Both databases are
	 * crash-safe without an explicit close.
	 *
	 * `isOpen` guards the second call: unlike better-sqlite3's no-op close,
	 * `DatabaseSync.close()` throws on an already-closed handle, and `stop()`
	 * being called twice is not an error worth logging. Compared against `false`
	 * rather than truthiness so a runtime without the property (it landed in
	 * 22.15, below our engines floor but `engines` is only advisory) still
	 * attempts the close instead of silently skipping every database.
	 */
	async close(): Promise<void> {
		for (const db of this.databases) {
			if (db.isOpen === false) continue
			try {
				db.close()
			} catch (err) {
				this.logger.warn({ err }, "Failed to close database cleanly")
			}
		}
	}
}
