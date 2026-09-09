import { existsSync, mkdirSync } from "node:fs"
import { join } from "node:path"
import { DatabaseSync } from "node:sqlite"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { ActivityStore, BidStore, SimplexDataStore, StateStore } from "@/data/types"
import { SqliteActivityStore } from "./activity"
import { SqliteBidStore } from "./bids"
import { FileStateStore } from "./state"

export { SqliteActivityStore } from "./activity"
export { SqliteBidStore } from "./bids"
export { FileStateStore } from "./state"

/**
 * File-backed {@link SimplexDataStore} — the durable default, and what the
 * `simplex` CLI uses.
 *
 * Keeps bids and activity in separate database files (`bids.db`,
 * `activity.db`) so an existing data directory written by an earlier version is
 * picked up unchanged, plus `runtime-state.json` for operator state.
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
		// `node:sqlite` has no .pragma(); exec() discards the row this one returns.
		activityDb.exec("PRAGMA journal_mode = WAL")

		this.databases = [bidsDb, activityDb]
		this.bids = new SqliteBidStore(bidsDb, loggers)
		this.activity = new SqliteActivityStore(activityDb, loggers)
		this.state = new FileStateStore(dataDir)

		this.logger.info({ dataDir }, "SQLite data store opened")
	}

	/**
	 * Closes both databases. Called by `Simplex.stop()`.
	 *
	 * Deliberately not wired to a `process.on("exit")` hook: the old services did
	 * that in their constructors, which leaked a listener per instance and made
	 * two fillers in one process race each other's teardown. Both databases are
	 * crash-safe without an explicit close.
	 *
	 * `isOpen` guards the second call: unlike better-sqlite3's no-op close,
	 * `DatabaseSync.close()` throws on an already-closed handle, and `stop()`
	 * being called twice is not an error worth logging.
	 */
	async close(): Promise<void> {
		for (const db of this.databases) {
			if (!db.isOpen) continue
			try {
				db.close()
			} catch (err) {
				this.logger.warn({ err }, "Failed to close database cleanly")
			}
		}
	}
}
