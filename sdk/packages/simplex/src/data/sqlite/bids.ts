import type { Database as DatabaseType } from "better-sqlite3"
import { getLogger } from "@/services/Logger"
import { sqliteDatetime } from "@/data/memory"
import type { BidInsert, BidStats, BidStore, StoredBid } from "@/data/types"

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
 * SQLite-backed {@link BidStore}.
 *
 * better-sqlite3 is synchronous, so every method here resolves immediately —
 * the promises exist to satisfy the interface, not because work is deferred.
 * That also means `store` is durable the moment it resolves, which is what the
 * retraction sweep relies on.
 */
export class SqliteBidStore implements BidStore {
	private logger = getLogger("bid-storage")

	constructor(private db: DatabaseType) {
		this.initializeSchema()
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
	}

	// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
	private toStoredBid(row: any): StoredBid {
		return {
			...row,
			success: Boolean(row.success),
			retracted: Boolean(row.retracted),
			dead: Boolean(row.dead),
		}
	}

	async store(bid: BidInsert): Promise<void> {
		const result = this.db
			.prepare(`
				INSERT INTO bids (commitment, extrinsic_hash, block_hash, success, error)
				VALUES (?, ?, ?, ?, ?)
			`)
			.run(
				bid.commitment,
				bid.extrinsicHash || null,
				bid.blockHash || null,
				bid.success ? 1 : 0,
				bid.error || null,
			)

		this.logger.debug({ id: result.lastInsertRowid, commitment: bid.commitment, success: bid.success }, "Bid stored")
	}

	async byCommitment(commitment: string): Promise<StoredBid | null> {
		const row = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids WHERE commitment = ? ORDER BY created_at DESC LIMIT 1`)
			.get(commitment)
		return row ? this.toStoredBid(row) : null
	}

	async unretractedSuccessful(): Promise<StoredBid[]> {
		const rows = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids WHERE success = 1 AND retracted = 0 ORDER BY created_at ASC`)
			.all()
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => this.toStoredBid(row))
	}

	async expiredUnretracted(maxAgeMs: number): Promise<StoredBid[]> {
		// Cutoff must match SQLite's datetime('now') format — see sqliteDatetime.
		const cutoff = sqliteDatetime(new Date(Date.now() - maxAgeMs))
		const rows = this.db
			.prepare(`
				SELECT ${BID_COLUMNS}
				FROM bids
				WHERE success = 1 AND retracted = 0 AND (dead = 1 OR created_at < ?)
				ORDER BY created_at ASC
			`)
			.all(cutoff)
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => this.toStoredBid(row))
	}

	async byDateRange(from: Date, to: Date): Promise<StoredBid[]> {
		const rows = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids WHERE created_at BETWEEN ? AND ? ORDER BY created_at DESC`)
			.all(sqliteDatetime(from), sqliteDatetime(to))
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => this.toStoredBid(row))
	}

	async markRetracted(commitment: string, retractExtrinsicHash: string | null): Promise<boolean> {
		const result = this.db
			.prepare(`
				UPDATE bids
				SET retracted = 1, retracted_at = datetime('now'), retract_extrinsic_hash = ?
				WHERE commitment = ? AND retracted = 0
			`)
			.run(retractExtrinsicHash, commitment)

		if (result.changes > 0) {
			this.logger.info({ commitment, retractExtrinsicHash }, "Bid marked as retracted")
			return true
		}
		return false
	}

	async markDead(commitment: string): Promise<boolean> {
		const result = this.db
			.prepare("UPDATE bids SET dead = 1 WHERE commitment = ? AND retracted = 0 AND dead = 0")
			.run(commitment)

		if (result.changes > 0) {
			this.logger.debug({ commitment }, "Bid marked as dead (order filled on-chain)")
			return true
		}
		return false
	}

	async stats(): Promise<BidStats> {
		const stats = this.db
			.prepare(`
				SELECT
					COUNT(*) as total,
					SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
					SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed,
					SUM(CASE WHEN retracted = 1 THEN 1 ELSE 0 END) as retracted,
					SUM(CASE WHEN success = 1 AND retracted = 0 THEN 1 ELSE 0 END) as pendingRetraction
				FROM bids
			`)
			// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
			.get() as any

		return {
			total: stats.total || 0,
			successful: stats.successful || 0,
			failed: stats.failed || 0,
			retracted: stats.retracted || 0,
			pendingRetraction: stats.pendingRetraction || 0,
		}
	}

	async failed(limit = 100): Promise<StoredBid[]> {
		const rows = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids WHERE success = 0 ORDER BY created_at DESC LIMIT ?`)
			.all(limit)
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => this.toStoredBid(row))
	}

	async recent(limit = 100): Promise<StoredBid[]> {
		const rows = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids ORDER BY id DESC LIMIT ?`)
			.all(Math.min(Math.max(limit, 1), 500))
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => this.toStoredBid(row))
	}
}
