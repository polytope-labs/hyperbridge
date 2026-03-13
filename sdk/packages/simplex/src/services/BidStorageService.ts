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
}

export interface BidInsertData {
	commitment: HexString
	extrinsicHash?: HexString
	blockHash?: HexString
	success: boolean
	error?: string
}

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
				retract_extrinsic_hash TEXT
			);

			CREATE INDEX IF NOT EXISTS idx_bids_commitment ON bids(commitment);
			CREATE INDEX IF NOT EXISTS idx_bids_success ON bids(success);
			CREATE INDEX IF NOT EXISTS idx_bids_retracted ON bids(retracted);
			CREATE INDEX IF NOT EXISTS idx_bids_created_at ON bids(created_at);

			CREATE TABLE IF NOT EXISTS dashboard_stats (
				key TEXT PRIMARY KEY,
				value INTEGER NOT NULL DEFAULT 0
			);

			CREATE TABLE IF NOT EXISTS balance_history (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				timestamp INTEGER NOT NULL,
				usdc REAL NOT NULL DEFAULT 0,
				usdt REAL NOT NULL DEFAULT 0,
				exotics TEXT NOT NULL DEFAULT '{}'
			);

			CREATE INDEX IF NOT EXISTS idx_balance_history_ts ON balance_history(timestamp);
		`)

		this.logger.debug("Bid storage schema initialized")
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
			SELECT 
				id,
				commitment,
				extrinsic_hash as extrinsicHash,
				block_hash as blockHash,
				success,
				error,
				created_at as createdAt,
				retracted,
				retracted_at as retractedAt,
				retract_extrinsic_hash as retractExtrinsicHash
			FROM bids 
			WHERE commitment = ?
			ORDER BY created_at DESC
			LIMIT 1
		`)

		const row = stmt.get(commitment) as any

		if (!row) return null

		return {
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
		}
	}

	/**
	 * Retrieves all successful bids that haven't been retracted
	 * These are candidates for fund recovery
	 */
	getUnretractedSuccessfulBids(): StoredBid[] {
		const stmt = this.db.prepare(`
			SELECT 
				id,
				commitment,
				extrinsic_hash as extrinsicHash,
				block_hash as blockHash,
				success,
				error,
				created_at as createdAt,
				retracted,
				retracted_at as retractedAt,
				retract_extrinsic_hash as retractExtrinsicHash
			FROM bids 
			WHERE success = 1 AND retracted = 0
			ORDER BY created_at ASC
		`)

		const rows = stmt.all() as any[]

		return rows.map((row) => ({
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
		}))
	}

	/**
	 * Retrieves successful, unretracted bids older than the given age.
	 * Used by the periodic sweep to retract stale bids after a TTL.
	 */
	getExpiredUnretractedBids(maxAgeMs: number): StoredBid[] {
		const cutoff = new Date(Date.now() - maxAgeMs).toISOString()
		const stmt = this.db.prepare(`
			SELECT 
				id,
				commitment,
				extrinsic_hash as extrinsicHash,
				block_hash as blockHash,
				success,
				error,
				created_at as createdAt,
				retracted,
				retracted_at as retractedAt,
				retract_extrinsic_hash as retractExtrinsicHash
			FROM bids 
			WHERE success = 1 AND retracted = 0 AND created_at < ?
			ORDER BY created_at ASC
		`)

		const rows = stmt.all(cutoff) as any[]

		return rows.map((row) => ({
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
		}))
	}

	/**
	 * Retrieves all bids within a date range
	 */
	getBidsByDateRange(startDate: Date, endDate: Date): StoredBid[] {
		const stmt = this.db.prepare(`
			SELECT 
				id,
				commitment,
				extrinsic_hash as extrinsicHash,
				block_hash as blockHash,
				success,
				error,
				created_at as createdAt,
				retracted,
				retracted_at as retractedAt,
				retract_extrinsic_hash as retractExtrinsicHash
			FROM bids 
			WHERE created_at BETWEEN ? AND ?
			ORDER BY created_at DESC
		`)

		const rows = stmt.all(startDate.toISOString(), endDate.toISOString()) as any[]

		return rows.map((row) => ({
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
		}))
	}

	/**
	 * Marks a bid as retracted after successful fund recovery
	 */
	markBidAsRetracted(commitment: HexString, retractExtrinsicHash: HexString): boolean {
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
			SELECT 
				id,
				commitment,
				extrinsic_hash as extrinsicHash,
				block_hash as blockHash,
				success,
				error,
				created_at as createdAt,
				retracted,
				retracted_at as retractedAt,
				retract_extrinsic_hash as retractExtrinsicHash
			FROM bids 
			WHERE success = 0
			ORDER BY created_at DESC
			LIMIT ?
		`)

		const rows = stmt.all(limit) as any[]

		return rows.map((row) => ({
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
		}))
	}

	// ─── Dashboard Stats ─────────────────────────────────────────────────────────

	incrementDashboardStat(key: string, by = 1): void {
		this.db
			.prepare(
				`INSERT INTO dashboard_stats (key, value) VALUES (?, ?)
				 ON CONFLICT(key) DO UPDATE SET value = value + excluded.value`,
			)
			.run(key, by)
	}

	getDashboardStats(): Record<string, number> {
		const rows = this.db.prepare("SELECT key, value FROM dashboard_stats").all() as {
			key: string
			value: number
		}[]
		const result: Record<string, number> = {}
		for (const row of rows) result[row.key] = row.value
		return result
	}

	// ─── Balance History ──────────────────────────────────────────────────────────

	insertBalancePoint(point: { timestamp: number; usdc: number; usdt: number; exotics: Record<string, number> }): void {
		this.db
			.prepare(`INSERT INTO balance_history (timestamp, usdc, usdt, exotics) VALUES (?, ?, ?, ?)`)
			.run(point.timestamp, point.usdc, point.usdt, JSON.stringify(point.exotics))

		// Trim to last 50 000 rows (~35 days at 1-min intervals)
		this.db
			.prepare(
				`DELETE FROM balance_history WHERE id NOT IN
				 (SELECT id FROM balance_history ORDER BY timestamp DESC LIMIT 50000)`,
			)
			.run()
	}

	getBalanceHistorySince(
		since: number,
	): Array<{ timestamp: number; usdc: number; usdt: number; exotics: Record<string, number> }> {
		const rows = this.db
			.prepare(
				`SELECT timestamp, usdc, usdt, exotics
				 FROM balance_history
				 WHERE timestamp >= ?
				 ORDER BY timestamp ASC`,
			)
			.all(since) as { timestamp: number; usdc: number; usdt: number; exotics: string }[]

		return rows.map((row) => ({
			timestamp: row.timestamp,
			usdc: row.usdc,
			usdt: row.usdt,
			exotics: JSON.parse(row.exotics) as Record<string, number>,
		}))
	}

	getRecentBalanceHistory(
		limit = 200,
	): Array<{ timestamp: number; usdc: number; usdt: number; exotics: Record<string, number> }> {
		const rows = this.db
			.prepare(
				`SELECT timestamp, usdc, usdt, exotics
				 FROM balance_history
				 ORDER BY timestamp DESC
				 LIMIT ?`,
			)
			.all(limit) as { timestamp: number; usdc: number; usdt: number; exotics: string }[]

		return rows.reverse().map((row) => ({
			timestamp: row.timestamp,
			usdc: row.usdc,
			usdt: row.usdt,
			exotics: JSON.parse(row.exotics) as Record<string, number>,
		}))
	}

	/**
	 * Closes the database connection
	 */
	close(): void {
		this.db.close()
		this.logger.debug("Bid storage database closed")
	}
}
