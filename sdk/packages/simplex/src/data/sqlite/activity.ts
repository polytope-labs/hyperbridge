import type { Database as DatabaseType } from "better-sqlite3"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { ActivityEvent, ActivityInsert, ActivityStore, OrderHistoryPage, OrderSummary, WalletTx } from "@/data/types"

const MAX_ROWS = 10_000
const PRUNE_EVERY = 500

// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
function toActivityEvent(row: any): ActivityEvent {
	return {
		id: row.id,
		ts: row.ts,
		type: row.type,
		orderId: row.order_id,
		chainId: row.chain_id,
		strategy: row.strategy,
		success: row.success === null ? null : row.success === 1,
		reason: row.reason,
		volumeUsd: row.volume_usd,
		profitUsd: row.profit_usd,
		txHash: row.tx_hash,
		order: parseOrder(row.order_json),
	}
}

function parseOrder(json: string | null): OrderSummary | null {
	if (!json) return null
	try {
		return JSON.parse(json) as OrderSummary
	} catch {
		return null
	}
}

function capLimit(limit: number): number {
	return Math.min(Math.max(limit, 1), 500)
}

/**
 * SQLite-backed {@link ActivityStore}.
 *
 * Unlike the bid store this is observability rather than correctness, so it
 * prunes: every {@link PRUNE_EVERY} inserts the oldest rows beyond
 * {@link MAX_ROWS} are dropped from both tables. A prune failure is logged and
 * swallowed — losing the trim must never fail the write that triggered it.
 */
export class SqliteActivityStore implements ActivityStore {
	private logger: Logger
	private insertsSincePrune = 0

	constructor(
		private db: DatabaseType,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("activity")
		this.db.exec(`
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
		// Added after the first release: rows written before it have no order summary.
		const columns = new Set((this.db.prepare("PRAGMA table_info(events)").all() as any[]).map((c) => c.name))
		if (!columns.has("order_json")) {
			this.db.exec("ALTER TABLE events ADD COLUMN order_json TEXT")
			this.logger.info({ column: "order_json" }, "Migrated activity schema")
		}
	}

	async record(event: ActivityInsert): Promise<ActivityEvent> {
		const ts = Date.now()
		const result = this.db
			.prepare(`
				INSERT INTO events (ts, type, order_id, chain_id, strategy, success, reason, volume_usd, profit_usd, tx_hash, order_json)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			`)
			.run(
				ts,
				event.type,
				event.orderId ?? null,
				event.chainId ?? null,
				event.strategy ?? null,
				event.success === undefined || event.success === null ? null : event.success ? 1 : 0,
				event.reason ?? null,
				event.volumeUsd ?? null,
				event.profitUsd ?? null,
				event.txHash ?? null,
				event.order ? JSON.stringify(event.order) : null,
			)

		if (++this.insertsSincePrune >= PRUNE_EVERY) {
			this.insertsSincePrune = 0
			try {
				this.db.prepare("DELETE FROM events WHERE id <= (SELECT MAX(id) FROM events) - ?").run(MAX_ROWS)
				this.db.prepare("DELETE FROM wallet_txs WHERE id <= (SELECT MAX(id) FROM wallet_txs) - ?").run(MAX_ROWS)
			} catch (err) {
				this.logger.warn({ err }, "Activity log prune failed")
			}
		}

		return {
			id: Number(result.lastInsertRowid),
			ts,
			type: event.type,
			orderId: event.orderId ?? null,
			chainId: event.chainId ?? null,
			strategy: event.strategy ?? null,
			success: event.success ?? null,
			reason: event.reason ?? null,
			volumeUsd: event.volumeUsd ?? null,
			profitUsd: event.profitUsd ?? null,
			txHash: event.txHash ?? null,
			order: event.order ?? null,
		}
	}

	async recent(limit = 100, beforeId?: number): Promise<ActivityEvent[]> {
		const capped = capLimit(limit)
		const rows = beforeId
			? this.db.prepare("SELECT * FROM events WHERE id < ? ORDER BY id DESC LIMIT ?").all(beforeId, capped)
			: this.db.prepare("SELECT * FROM events ORDER BY id DESC LIMIT ?").all(capped)
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map(toActivityEvent)
	}

	async fills(limit = 100): Promise<ActivityEvent[]> {
		const rows = this.db
			.prepare(
				"SELECT * FROM events WHERE type = 'filled' AND tx_hash IS NOT NULL AND chain_id IS NOT NULL ORDER BY id DESC LIMIT ?",
			)
			.all(capLimit(limit))
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map(toActivityEvent)
	}

	async orderIdsMissingSummary(limit = 500): Promise<string[]> {
		const rows = this.db
			.prepare(
				"SELECT order_id FROM events WHERE order_id IS NOT NULL AND order_json IS NULL GROUP BY order_id ORDER BY MAX(id) DESC LIMIT ?",
			)
			.all(capLimit(limit))
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => row.order_id as string)
	}

	async attachOrder(orderId: string, order: OrderSummary): Promise<ActivityEvent[]> {
		const json = JSON.stringify(order)
		const changed = this.db.transaction(() => {
			const ids = this.db
				.prepare("SELECT id FROM events WHERE order_id = ? AND order_json IS NULL")
				.all(orderId)
				// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
				.map((row: any) => row.id as number)
			if (ids.length === 0) return []
			this.db.prepare("UPDATE events SET order_json = ? WHERE order_id = ? AND order_json IS NULL").run(json, orderId)
			return ids
		})()
		if (changed.length === 0) return []
		const rows = this.db
			.prepare(`SELECT * FROM events WHERE id IN (${changed.map(() => "?").join(",")}) ORDER BY id`)
			.all(...changed)
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map(toActivityEvent)
	}

	async unsettledOrders(limit = 500): Promise<string[]> {
		const rows = this.db
			.prepare(
				`SELECT order_id FROM events
				 WHERE order_id IS NOT NULL
				 GROUP BY order_id
				 HAVING SUM(type = 'bid' OR (type = 'filled' AND volume_usd IS NOT NULL)) > 0
				    AND SUM(type = 'lost' OR (type = 'filled' AND volume_usd IS NULL)) = 0
				 ORDER BY MAX(id) DESC LIMIT ?`,
			)
			.all(capLimit(limit)) as Array<{ order_id: string }>
		return rows.map((row) => row.order_id)
	}

	async retypeLegacyBid(orderId: string): Promise<ActivityEvent[]> {
		const ids = (
			this.db
				.prepare("SELECT id FROM events WHERE order_id = ? AND type = 'filled' AND volume_usd IS NOT NULL")
				.all(orderId) as Array<{ id: number }>
		).map((row) => row.id)
		if (ids.length === 0) return []
		this.db
			.prepare("UPDATE events SET type = 'bid' WHERE order_id = ? AND type = 'filled' AND volume_usd IS NOT NULL")
			.run(orderId)
		const rows = this.db
			.prepare(`SELECT * FROM events WHERE id IN (${ids.map(() => "?").join(",")}) ORDER BY id`)
			.all(...ids)
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map(toActivityEvent)
	}

	async knowsOrder(orderId: string): Promise<boolean> {
		return this.db.prepare("SELECT 1 FROM events WHERE order_id = ? LIMIT 1").get(orderId) !== undefined
	}

	async orderHistory(page: number, pageSize: number): Promise<OrderHistoryPage> {
		const size = capLimit(pageSize)
		const current = Math.max(1, Math.floor(page))
		const total = (
			this.db.prepare("SELECT COUNT(DISTINCT order_id) AS n FROM events WHERE order_id IS NOT NULL").get() as {
				n: number
			}
		).n
		const heads = this.db
			.prepare(
				"SELECT order_id FROM events WHERE order_id IS NOT NULL GROUP BY order_id ORDER BY MAX(id) DESC LIMIT ? OFFSET ?",
			)
			.all(size, (current - 1) * size) as Array<{ order_id: string }>
		const ids = heads.map((head) => head.order_id)
		const byOrder = new Map<string, ActivityEvent[]>(ids.map((id) => [id, []]))
		if (ids.length > 0) {
			const rows = this.db
				.prepare(`SELECT * FROM events WHERE order_id IN (${ids.map(() => "?").join(",")}) ORDER BY id DESC`)
				.all(...ids)
			// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
			for (const row of (rows as any[]).map(toActivityEvent)) byOrder.get(row.orderId as string)?.push(row)
		}
		return {
			page: current,
			pageSize: size,
			total,
			orders: ids.map((orderId) => ({ orderId, events: byOrder.get(orderId) ?? [] })),
		}
	}

	async recordWalletTx(tx: Omit<WalletTx, "id" | "ts">): Promise<void> {
		this.db
			.prepare(`
				INSERT INTO wallet_txs (ts, kind, chain_id, token, amount, to_address, tx_hash, sponsored)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?)
			`)
			.run(
				Date.now(),
				tx.kind,
				tx.chainId,
				tx.token,
				tx.amount,
				tx.to,
				tx.txHash,
				tx.sponsored === null ? null : tx.sponsored ? 1 : 0,
			)
	}

	async walletTxs(limit = 100): Promise<WalletTx[]> {
		const rows = this.db.prepare("SELECT * FROM wallet_txs ORDER BY id DESC LIMIT ?").all(capLimit(limit))
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => ({
			id: row.id,
			ts: row.ts,
			kind: row.kind,
			chainId: row.chain_id,
			token: row.token,
			amount: row.amount,
			to: row.to_address,
			txHash: row.tx_hash,
			sponsored: row.sponsored === null ? null : row.sponsored === 1,
		}))
	}
}
