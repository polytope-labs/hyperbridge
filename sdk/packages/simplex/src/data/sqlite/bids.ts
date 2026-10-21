import type { DatabaseSync } from "node:sqlite"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { sqliteDatetime } from "@/data/memory"
import type { BidInsert, BidPricing, BidStats, BidStore, StoredBid } from "@/data/types"
import { columnNames } from "./schema"

/** Column list shared by every SELECT that returns a StoredBid. */
const BID_COLUMNS = `
	id,
	commitment,
	bid,
	extrinsic_hash as extrinsicHash,
	block_hash as blockHash,
	success,
	pending,
	error,
	created_at as createdAt,
	retracted,
	retracted_at as retractedAt,
	retract_extrinsic_hash as retractExtrinsicHash,
	dead,
	on_orderbook as onOrderbook,
	priced_by as pricedBy,
	reservations
`

/**
 * SQLite-backed {@link BidStore}.
 *
 * `node:sqlite` is synchronous, so every method here resolves immediately —
 * the promises exist to satisfy the interface, not because work is deferred.
 * That also means `store` is durable the moment it resolves, which is what the
 * retraction sweep relies on.
 */
function parseJson(raw: string | null): unknown {
	if (!raw) return null
	try {
		return JSON.parse(raw)
	} catch {
		return null
	}
}

/**
 * The limit order that priced a bid row.
 *
 * Read from `priced_by`, or, for a row written before that column existed, from the
 * first entry of the `reservations` list that build kept, whose `amount` is the payout.
 * A row whose reservation was already settled has none left to read, and needs none.
 */
function pricedBy(row: { pricedBy: string | null; reservations: string | null }): BidPricing | null {
	const current = parseJson(row.pricedBy) as BidPricing | null
	if (current && typeof current === "object") return current
	const legacy = parseJson(row.reservations)
	const first = Array.isArray(legacy) ? legacy[0] : null
	if (!first || typeof first !== "object" || typeof first.limitOrderId !== "string") return null
	return {
		limitOrderId: first.limitOrderId,
		payout: String(first.payout ?? first.amount),
		...(first.take !== undefined ? { take: String(first.take) } : {}),
		...(first.takeDecimals !== undefined ? { takeDecimals: Number(first.takeDecimals) } : {}),
	}
}

export class SqliteBidStore implements BidStore {
	private logger: Logger

	constructor(
		private db: DatabaseSync,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("bid-storage")
		this.initializeSchema()
	}

	private initializeSchema(): void {
		this.db.exec(`
			CREATE TABLE IF NOT EXISTS bids (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				commitment TEXT NOT NULL,
				bid TEXT,
				extrinsic_hash TEXT,
				block_hash TEXT,
				success INTEGER NOT NULL,
				pending INTEGER NOT NULL DEFAULT 0,
				error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				retracted INTEGER NOT NULL DEFAULT 0,
				retracted_at TEXT,
				retract_extrinsic_hash TEXT,
				dead INTEGER NOT NULL DEFAULT 0,
				on_orderbook INTEGER NOT NULL DEFAULT 0,
				-- Only read, as the provenance of a bid written before priced_by. Kept, and
				-- never written, so a rollback to a build that still reads it finds the column.
				reservations TEXT,
				priced_by TEXT
			);

			CREATE INDEX IF NOT EXISTS idx_bids_commitment ON bids(commitment);
			CREATE INDEX IF NOT EXISTS idx_bids_success ON bids(success);
			CREATE INDEX IF NOT EXISTS idx_bids_retracted ON bids(retracted);
			CREATE INDEX IF NOT EXISTS idx_bids_created_at ON bids(created_at);
		`)

		// Databases created before a column existed need it added in place.
		const columns = columnNames(this.db, "bids")
		for (const column of ["dead", "pending", "on_orderbook"] as const) {
			if (columns.has(column)) continue
			this.db.exec(`ALTER TABLE bids ADD COLUMN ${column} INTEGER NOT NULL DEFAULT 0`)
			this.logger.info({ column }, "Migrated bid storage schema")
		}
		// A row written before bids carried an identifier has none, and nothing to retract through
		// the current calls. A row written before `priced_by` reads its provenance from
		// `reservations` instead.
		for (const column of ["bid", "reservations", "priced_by"] as const) {
			if (columns.has(column)) continue
			this.db.exec(`ALTER TABLE bids ADD COLUMN ${column} TEXT`)
			this.logger.info({ column }, "Migrated bid storage schema")
		}

		// After the migration above, not with the other indexes: a database created
		// before `bid` existed has no such column until the ALTER runs.
		this.db.exec("CREATE INDEX IF NOT EXISTS idx_bids_bid ON bids(commitment, bid)")
	}

	// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
	private toStoredBid(row: any): StoredBid {
		const { reservations: _legacy, ...rest } = row
		return {
			...rest,
			success: Boolean(row.success),
			pending: Boolean(row.pending),
			retracted: Boolean(row.retracted),
			dead: Boolean(row.dead),
			onOrderbook: Boolean(row.onOrderbook),
			pricedBy: pricedBy(row),
		}
	}

	async store(bid: BidInsert): Promise<void> {
		const result = this.db
			.prepare(`
				INSERT INTO bids (commitment, bid, extrinsic_hash, block_hash, success, pending, error, on_orderbook, priced_by)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			`)
			.run(
				bid.commitment,
				bid.bid ?? null,
				bid.extrinsicHash || null,
				bid.blockHash || null,
				bid.success ? 1 : 0,
				bid.pending ? 1 : 0,
				bid.error || null,
				bid.onOrderbook ? 1 : 0,
				bid.pricedBy ? JSON.stringify(bid.pricedBy) : null,
			)

		this.logger.debug({ id: result.lastInsertRowid, commitment: bid.commitment, success: bid.success }, "Bid stored")
	}

	async byCommitment(commitment: string): Promise<StoredBid | null> {
		const row = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids WHERE commitment = ? ORDER BY created_at DESC, id DESC LIMIT 1`)
			.get(commitment)
		return row ? this.toStoredBid(row) : null
	}

	async unretractedReclaimable(): Promise<StoredBid[]> {
		const rows = this.db
			.prepare(`SELECT ${BID_COLUMNS} FROM bids WHERE (success = 1 OR pending = 1) AND retracted = 0 ORDER BY created_at ASC`)
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
				WHERE (success = 1 OR pending = 1) AND retracted = 0 AND (dead = 1 OR created_at < ?)
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

	async markRetracted(
		commitment: string,
		retractExtrinsicHash: string | null,
		rows?: readonly number[],
	): Promise<boolean> {
		if (rows?.length === 0) return false
		const result = this.db
			.prepare(`
				UPDATE bids
				SET retracted = 1, retracted_at = datetime('now'), retract_extrinsic_hash = ?
				WHERE commitment = ? AND retracted = 0${rows ? ` AND id IN (${rows.map(() => "?").join(",")})` : ""}
			`)
			.run(retractExtrinsicHash, commitment, ...(rows ?? []))

		if (result.changes > 0) {
			this.logger.info({ commitment, retractExtrinsicHash }, "Bid marked as retracted")
			return true
		}
		return false
	}

	async byLimitOrder(limitOrderId: string, limit = 100): Promise<StoredBid[]> {
		// Matched in SQL on the id inside the JSON, then filtered exactly here: the
		// LIKE narrows the scan, and the parse is what decides. `reservations` is
		// searched too, for bids written before `priced_by`.
		const pattern = `%${limitOrderId}%`
		const rows = this.db
			.prepare(
				`SELECT ${BID_COLUMNS} FROM bids WHERE priced_by LIKE ? OR (priced_by IS NULL AND reservations LIKE ?) ORDER BY id DESC LIMIT ?`,
			)
			.all(pattern, pattern, Math.min(Math.max(limit, 1), 500))
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[])
			.map((row) => this.toStoredBid(row))
			.filter((bid) => bid.pricedBy?.limitOrderId === limitOrderId)
	}

	async markDead(commitment: string, rows?: readonly number[]): Promise<boolean> {
		if (rows?.length === 0) return false
		const result = this.db
			.prepare(
				`UPDATE bids SET dead = 1 WHERE commitment = ? AND retracted = 0 AND dead = 0${rows ? ` AND id IN (${rows.map(() => "?").join(",")})` : ""}`,
			)
			.run(commitment, ...(rows ?? []))

		if (result.changes > 0) {
			this.logger.debug({ commitment }, "Bid marked dead, due for retraction on the next sweep")
			return true
		}
		return false
	}

	async stats(): Promise<BidStats> {
		// COUNT is always a number; every SUM is null when the table is empty,
		// which is what the `|| 0`s below are for.
		const stats = this.db
			.prepare(`
				SELECT
					COUNT(*) as total,
					SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
					SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed,
					SUM(CASE WHEN retracted = 1 THEN 1 ELSE 0 END) as retracted,
					SUM(CASE WHEN (success = 1 OR pending = 1) AND retracted = 0 THEN 1 ELSE 0 END) as pendingRetraction
				FROM bids
			`)
			.get() as unknown as Record<keyof BidStats, number | null>

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

	async byCommitments(commitments: string[]): Promise<StoredBid[]> {
		if (commitments.length === 0) return []
		const rows = this.db
			.prepare(
				`SELECT ${BID_COLUMNS} FROM bids WHERE commitment IN (${commitments.map(() => "?").join(",")}) ORDER BY id DESC`,
			)
			.all(...commitments)
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
