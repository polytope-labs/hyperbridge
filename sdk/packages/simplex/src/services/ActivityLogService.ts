import Database, { Database as DatabaseType } from "better-sqlite3"
import { EventEmitter } from "node:events"
import { existsSync, mkdirSync } from "fs"
import { join } from "path"
import type { EventMonitor } from "@/core/event-monitor"
import { getLogger } from "./Logger"

export type ActivityType = "detected" | "filled" | "executed" | "skipped" | "rebalance"

export interface ActivityEvent {
	id: number
	ts: number
	type: ActivityType
	orderId: string | null
	chainId: number | null
	strategy: string | null
	success: boolean | null
	/** Skip reason or execution error. */
	reason: string | null
	volumeUsd: number | null
	profitUsd: number | null
	txHash: string | null
}

export type ActivityInsert = Partial<Omit<ActivityEvent, "id" | "ts" | "type">> & { type: ActivityType }

const MAX_ROWS = 10_000
const PRUNE_EVERY = 500

/**
 * Persistent order-activity feed for the operator UI: every detected, filled,
 * executed and skipped order lands in SQLite, and live inserts are re-emitted
 * (`event`) for the SSE stream.
 */
export class ActivityLogService extends EventEmitter {
	private db: DatabaseType
	private logger = getLogger("activity")
	private insertsSincePrune = 0

	constructor(dataDir?: string) {
		super()
		const dir = dataDir || join(process.cwd(), ".simplex-data")
		if (!existsSync(dir)) {
			mkdirSync(dir, { recursive: true })
		}
		this.db = new Database(join(dir, "activity.db"))
		this.db.pragma("journal_mode = WAL")
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
		`)
	}

	/** Subscribes to the filler's order lifecycle events. */
	attach(monitor: EventMonitor): void {
		monitor.on("newOrder", ({ order }) => {
			this.record({ type: "detected", orderId: order.id })
		})
		monitor.on(
			"orderFilled",
			({ orderId, hash, volumeUsd, profitUsd, chainId }: Record<string, string | number | undefined>) => {
				this.record({
					type: "filled",
					orderId: orderId as string,
					txHash: (hash as string) ?? null,
					volumeUsd: (volumeUsd as number) ?? null,
					profitUsd: (profitUsd as number) ?? null,
					chainId: (chainId as number) ?? null,
				})
			},
		)
		monitor.on(
			"orderExecuted",
			({ orderId, success, txHash, strategy, error }: Record<string, string | boolean | undefined>) => {
				this.record({
					type: "executed",
					orderId: orderId as string,
					success: Boolean(success),
					strategy: (strategy as string) ?? null,
					txHash: (txHash as string) ?? null,
					reason: (error as string) ?? null,
				})
			},
		)
		monitor.on("orderSkipped", ({ orderId, reason }: Record<string, string | undefined>) => {
			this.record({ type: "skipped", orderId: orderId ?? null, reason: reason ?? null })
		})
		monitor.on(
			"rebalanceExecuted",
			({ transferCount, executedCount, success, error }: Record<string, number | boolean | string | undefined>) => {
				this.record({
					type: "rebalance",
					success: Boolean(success),
					reason:
						(error as string) ??
						(transferCount !== undefined ? `${executedCount}/${transferCount} transfers executed` : null),
				})
			},
		)
	}

	record(event: ActivityInsert): ActivityEvent {
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

		const row: ActivityEvent = {
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
		this.emit("event", row)

		if (++this.insertsSincePrune >= PRUNE_EVERY) {
			this.insertsSincePrune = 0
			try {
				this.db
					.prepare("DELETE FROM events WHERE id <= (SELECT MAX(id) FROM events) - ?")
					.run(MAX_ROWS)
			} catch (err) {
				this.logger.warn({ err }, "Activity log prune failed")
			}
		}
		return row
	}

	/** Newest first; pass `beforeId` for older pages. */
	getRecent(limit = 100, beforeId?: number): ActivityEvent[] {
		const capped = Math.min(Math.max(limit, 1), 500)
		const rows = beforeId
			? this.db
					.prepare("SELECT * FROM events WHERE id < ? ORDER BY id DESC LIMIT ?")
					.all(beforeId, capped)
			: this.db.prepare("SELECT * FROM events ORDER BY id DESC LIMIT ?").all(capped)
		// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
		return (rows as any[]).map((row) => ({
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
		}))
	}

	close(): void {
		this.db.close()
	}
}
