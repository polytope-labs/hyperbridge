import type { Database as DatabaseType } from "better-sqlite3"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { ActivityEvent, ActivityInsert, ActivityStore, WalletTx } from "@/data/types"

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
	}

	async record(event: ActivityInsert): Promise<ActivityEvent> {
		const ts = Date.now()
		const result = this.db
			.prepare(`
				INSERT INTO events (ts, type, order_id, chain_id, strategy, success, reason, volume_usd, profit_usd, tx_hash)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
