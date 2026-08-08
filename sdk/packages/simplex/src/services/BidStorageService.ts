import Database, { Database as DatabaseType } from "better-sqlite3"
import { HexString } from "@hyperbridge/sdk"
import { getLogger } from "./Logger"
import { existsSync, mkdirSync } from "fs"
import { join } from "path"

export interface StoredBid {
	id: number
	commitment: HexString
	extrinsicHash: HexString | null
	blockHash: HexString | null
	success: boolean
	error: string | null
	createdAt: string
	retracted: boolean
	retractedAt: string | null
	retractExtrinsicHash: HexString | null
	/** The order was observed filled on-chain, so this bid can never win and should be retracted ASAP. */
	dead: boolean
}

export interface BidInsertData {
	commitment: HexString
	extrinsicHash?: HexString
	blockHash?: HexString
	success: boolean
	error?: string
}

/** Column list shared by every SELECT that returns a StoredBid. */
const BID_COLUMNS = `
	id,
	commitment,
	extrinsic_hash as extrinsicHash,
	block_hash as blockHash,
	success,
	error,
	created_at as createdAt,
	retracted,
	retracted_at as retractedAt,
	retract_extrinsic_hash as retractExtrinsicHash,
	dead
`

/**
 * Service for persistent storage of Hyperbridge bid submissions.
 * Stores both successful and failed bid transaction hashes for later
 * cleanup and fund recovery.
 *
 * Uses SQLite for lightweight, file-based persistence.
 */
export class BidStorageService {
	private db: DatabaseType
	private logger = getLogger("bid-storage")

	constructor(dataDir?: string) {
		const dbPath = this.resolveDatabasePath(dataDir)
		this.logger.info({ dbPath }, "Initializing bid storage database")

		this.db = new Database(dbPath)
		this.initializeSchema()

		process.on("exit", () => this.close())
	}

	private resolveDatabasePath(dataDir?: string): string {
		const dir = dataDir || join(process.cwd(), ".simplex-data")

		// Ensure directory exists
		if (!existsSync(dir)) {
			mkdirSync(dir, { recursive: true })
		}

		return join(dir, "bids.db")
	}

	private initializeSchema(): void {
		this.db.exec(`
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
				retract_extrinsic_hash TEXT,
				dead INTEGER NOT NULL DEFAULT 0
			);

			CREATE INDEX IF NOT EXISTS idx_bids_commitment ON bids(commitment);
			CREATE INDEX IF NOT EXISTS idx_bids_success ON bids(success);
			CREATE INDEX IF NOT EXISTS idx_bids_retracted ON bids(retracted);
			CREATE INDEX IF NOT EXISTS idx_bids_created_at ON bids(created_at);

		`)

		// Databases created before the column existed need it added in place.
		const columns = this.db.pragma("table_info(bids)") as { name: string }[]
		if (!columns.some((column) => column.name === "dead")) {
			this.db.exec("ALTER TABLE bids ADD COLUMN dead INTEGER NOT NULL DEFAULT 0")
			this.logger.info("Migrated bid storage schema: added 'dead' column")
		}

		this.logger.debug("Bid storage schema initialized")
	}

	private toStoredBid(row: any): StoredBid {
		return {
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
			dead: Boolean(row.dead),
		}
	}

	/**
	 * Stores a bid submission result (success or failure)
	 */
	storeBid(data: BidInsertData): number {
		const stmt = this.db.prepare(`
			INSERT INTO bids (commitment, extrinsic_hash, block_hash, success, error)
			VALUES (?, ?, ?, ?, ?)
		`)

		const result = stmt.run(
			data.commitment,
			data.extrinsicHash || null,
			data.blockHash || null,
			data.success ? 1 : 0,
			data.error || null,
		)

		this.logger.debug(
			{
				id: result.lastInsertRowid,
				commitment: data.commitment,
				success: data.success,
			},
			"Bid stored",
		)

		return result.lastInsertRowid as number
	}

	/**
	 * Retrieves a bid by commitment hash
	 */
	getBidByCommitment(commitment: HexString): StoredBid | null {
		const stmt = this.db.prepare(`
			SELECT ${BID_COLUMNS}
			FROM bids
			WHERE commitment = ?
			ORDER BY created_at DESC
			LIMIT 1
		`)

		const row = stmt.get(commitment) as any

		if (!row) return null

		return this.toStoredBid(row)
	}

	/**
	 * Retrieves all successful bids that haven't been retracted
	 * These are candidates for fund recovery
	 */
	getUnretractedSuccessfulBids(): StoredBid[] {
		const stmt = this.db.prepare(`
			SELECT ${BID_COLUMNS}
			FROM bids
			WHERE success = 1 AND retracted = 0
			ORDER BY created_at ASC
		`)

		const rows = stmt.all() as any[]

		return rows.map((row) => this.toStoredBid(row))
	}

	/**
	 * Retrieves successful, unretracted bids that are due for retraction: bids older than the
	 * given age, plus dead bids (order already filled on-chain) regardless of age — a dead bid's
	 * deposit is reclaimable now, so a failed retraction attempt is retried on the next sweep
	 * cycle instead of waiting out the TTL.
	 */
	getExpiredUnretractedBids(maxAgeMs: number): StoredBid[] {
		// Format must match SQLite's datetime('now'): "YYYY-MM-DD HH:MM:SS"
		// Using .toISOString() produces "YYYY-MM-DDTHH:MM:SS.sssZ" which breaks
		// lexicographic comparison because space (ASCII 32) < 'T' (ASCII 84),
		// causing ALL bids to appear expired.
		const cutoff = new Date(Date.now() - maxAgeMs)
			.toISOString()
			.replace("T", " ")
			.replace(/\.\d{3}Z$/, "")
		const stmt = this.db.prepare(`
			SELECT ${BID_COLUMNS}
			FROM bids
			WHERE success = 1 AND retracted = 0 AND (dead = 1 OR created_at < ?)
			ORDER BY created_at ASC
		`)

		const rows = stmt.all(cutoff) as any[]

		return rows.map((row) => this.toStoredBid(row))
	}

	/**
	 * Retrieves all bids within a date range
	 */
	getBidsByDateRange(startDate: Date, endDate: Date): StoredBid[] {
		const stmt = this.db.prepare(`
			SELECT ${BID_COLUMNS}
			FROM bids
			WHERE created_at BETWEEN ? AND ?
			ORDER BY created_at DESC
		`)

		const toSqliteDatetime = (d: Date) =>
			d
				.toISOString()
				.replace("T", " ")
				.replace(/\.\d{3}Z$/, "")
		const rows = stmt.all(toSqliteDatetime(startDate), toSqliteDatetime(endDate)) as any[]

		return rows.map((row) => this.toStoredBid(row))
	}

	/**
	 * Marks a bid as retracted. Called after successful fund recovery, and also when the chain
	 * reports `BidNotFound` — bids only leave the pallet by retraction, so "no bid" means there
	 * is nothing left to reclaim (an earlier duplicate landed, or the bid never did) and no hash
	 * is available.
	 */
	markBidAsRetracted(commitment: HexString, retractExtrinsicHash: HexString | null): boolean {
		const stmt = this.db.prepare(`
			UPDATE bids
			SET retracted = 1,
				retracted_at = datetime('now'),
				retract_extrinsic_hash = ?
			WHERE commitment = ? AND retracted = 0
		`)

		const result = stmt.run(retractExtrinsicHash, commitment)

		if (result.changes > 0) {
			this.logger.info({ commitment, retractExtrinsicHash }, "Bid marked as retracted")
			return true
		}

		return false
	}

	/**
	 * Marks a bid as dead: its order was observed filled on-chain, so the bid can never win and
	 * its deposit should be reclaimed on the next retraction sweep regardless of the bid's age.
	 */
	markBidAsDead(commitment: HexString): boolean {
		const stmt = this.db.prepare(`
			UPDATE bids
			SET dead = 1
			WHERE commitment = ? AND retracted = 0 AND dead = 0
		`)

		const result = stmt.run(commitment)

		if (result.changes > 0) {
			this.logger.debug({ commitment }, "Bid marked as dead (order filled on-chain)")
			return true
		}

		return false
	}

	/**
	 * Gets statistics about stored bids
	 */
	getStats(): {
		total: number
		successful: number
		failed: number
		retracted: number
		pendingRetraction: number
	} {
		const stats = this.db
			.prepare(
				`
			SELECT
				COUNT(*) as total,
				SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
				SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed,
				SUM(CASE WHEN retracted = 1 THEN 1 ELSE 0 END) as retracted,
				SUM(CASE WHEN success = 1 AND retracted = 0 THEN 1 ELSE 0 END) as pendingRetraction
			FROM bids
		`,
			)
			.get() as any

		return {
			total: stats.total || 0,
			successful: stats.successful || 0,
			failed: stats.failed || 0,
			retracted: stats.retracted || 0,
			pendingRetraction: stats.pendingRetraction || 0,
		}
	}

	/**
	 * Retrieves all failed bids for analysis/debugging
	 */
	getFailedBids(limit = 100): StoredBid[] {
		const stmt = this.db.prepare(`
			SELECT ${BID_COLUMNS}
			FROM bids
			WHERE success = 0
			ORDER BY created_at DESC
			LIMIT ?
		`)

		const rows = stmt.all(limit) as any[]

		return rows.map((row) => this.toStoredBid(row))
	}

	/**
	 * Newest bids first, for the operator activity feed.
	 */
	getRecentBids(limit = 100): StoredBid[] {
		const stmt = this.db.prepare(`
			SELECT ${BID_COLUMNS}
			FROM bids
			ORDER BY id DESC
			LIMIT ?
		`)

		const rows = stmt.all(Math.min(Math.max(limit, 1), 500)) as any[]

		return rows.map((row) => this.toStoredBid(row))
	}

	/**
	 * Closes the database connection
	 */
	close(): void {
		this.db.close()
		this.logger.debug("Bid storage database closed")
	}
}
