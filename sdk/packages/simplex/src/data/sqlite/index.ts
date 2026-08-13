import Database, { type Database as DatabaseType } from "better-sqlite3"
import { existsSync, mkdirSync } from "node:fs"
import { join } from "node:path"
import { getLogger } from "@/services/Logger"
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
 * Requires the optional `better-sqlite3` native module. It is not a dependency
 * of the library entry point: importing `@hyperbridge/simplex` alone pulls in no
 * native code, and only `@hyperbridge/simplex/sqlite` needs it installed.
 */
export class SqliteDataStore implements SimplexDataStore {
	readonly bids: BidStore
	readonly activity: ActivityStore
	readonly state: StateStore

	private databases: DatabaseType[]
	private logger = getLogger("data-store")

	constructor(dataDir: string) {
		if (!existsSync(dataDir)) mkdirSync(dataDir, { recursive: true })

		const bidsDb = new Database(join(dataDir, "bids.db"))
		const activityDb = new Database(join(dataDir, "activity.db"))
		activityDb.pragma("journal_mode = WAL")

		this.databases = [bidsDb, activityDb]
		this.bids = new SqliteBidStore(bidsDb)
		this.activity = new SqliteActivityStore(activityDb)
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
	 */
	async close(): Promise<void> {
		for (const db of this.databases) {
			try {
				db.close()
			} catch (err) {
				this.logger.warn({ err }, "Failed to close database cleanly")
			}
		}
	}
}
