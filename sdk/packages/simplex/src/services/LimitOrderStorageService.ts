import Database, { Database as DatabaseType } from "better-sqlite3"
import { type Order, type HexString } from "@hyperbridge/sdk"
import { getLogger } from "./Logger"
import { existsSync, mkdirSync } from "fs"
import { join } from "path"

export interface StoredLimitOrder {
	orderId: string
	orderJson: string
	strategyName: string
	destinationChain: string
	deadlineBlock: string
	createdAt: string
}

function bigintReplacer(_key: string, value: unknown): unknown {
	return typeof value === "bigint" ? { __bigint: value.toString() } : value
}

function bigintReviver(_key: string, value: unknown): unknown {
	if (value && typeof value === "object" && "__bigint" in (value as Record<string, unknown>)) {
		return BigInt((value as { __bigint: string }).__bigint)
	}
	return value
}

/**
 * Service for persistent storage of limit orders — orders that the FXFiller
 * strategy can fill but are not currently profitable at the prevailing rate.
 *
 * A background sweep periodically re-evaluates pending limit orders and
 * either executes them (when profitable) or expires them (when the on-chain
 * deadline has passed).
 *
 * Uses the same SQLite database file as BidStorageService.
 */
export class LimitOrderStorageService {
	private db: DatabaseType
	private logger = getLogger("limit-order-storage")

	constructor(dataDir?: string) {
		const dbPath = this.resolveDatabasePath(dataDir)
		this.logger.info({ dbPath }, "Initializing limit order storage")

		this.db = new Database(dbPath)
		this.initializeSchema()

		process.on("exit", () => this.close())
	}

	private resolveDatabasePath(dataDir?: string): string {
		const dir = dataDir || join(process.cwd(), ".simplex-data")

		if (!existsSync(dir)) {
			mkdirSync(dir, { recursive: true })
		}

		return join(dir, "bids.db")
	}

	private initializeSchema(): void {
		this.db.exec(`
			CREATE TABLE IF NOT EXISTS limit_orders (
				order_id TEXT PRIMARY KEY NOT NULL,
				order_json TEXT NOT NULL,
				strategy_name TEXT NOT NULL,
				destination_chain TEXT NOT NULL,
				deadline_block TEXT NOT NULL,
				created_at TEXT NOT NULL DEFAULT (datetime('now'))
			);

			CREATE TABLE IF NOT EXISTS filler_metadata (
				key TEXT PRIMARY KEY NOT NULL,
				value TEXT NOT NULL
			);
		`)

		this.logger.debug("Limit order storage schema initialized")
	}

	/**
	 * Stores an order as a limit order for future re-evaluation.
	 * Idempotent — duplicate order IDs are silently ignored.
	 */
	storeLimitOrder(order: Order, strategyName: string): void {
		const orderId = order.id
		if (!orderId) {
			this.logger.warn("Cannot store limit order without order.id")
			return
		}

		const orderJson = JSON.stringify(order, bigintReplacer)

		const stmt = this.db.prepare(`
			INSERT OR IGNORE INTO limit_orders (order_id, order_json, strategy_name, destination_chain, deadline_block)
			VALUES (?, ?, ?, ?, ?)
		`)

		const result = stmt.run(orderId, orderJson, strategyName, order.destination, order.deadline.toString())

		if (result.changes > 0) {
			this.logger.debug(
				{ orderId, strategyName, deadline: order.deadline.toString() },
				"Limit order stored",
			)
		}
	}

	/**
	 * Retrieves all pending limit orders for re-evaluation.
	 */
	getPendingLimitOrders(): StoredLimitOrder[] {
		const stmt = this.db.prepare(`
			SELECT
				order_id as orderId,
				order_json as orderJson,
				strategy_name as strategyName,
				destination_chain as destinationChain,
				deadline_block as deadlineBlock,
				created_at as createdAt
			FROM limit_orders
			ORDER BY created_at ASC
		`)

		return stmt.all() as StoredLimitOrder[]
	}

	/**
	 * Deletes a limit order from the database.
	 * Used both when an order becomes profitable (executed) and when its deadline passes (expired).
	 */
	deleteLimitOrder(orderId: string): void {
		const stmt = this.db.prepare(`
			DELETE FROM limit_orders WHERE order_id = ?
		`)
		stmt.run(orderId)
	}

	/**
	 * Deserializes a stored order JSON string back into an Order object,
	 * correctly restoring bigint fields.
	 */
	deserializeOrder(json: string): Order {
		return JSON.parse(json, bigintReviver) as Order
	}

	/**
	 * Returns the last recorded shutdown timestamp, or null if never recorded.
	 */
	getLastShutdownTime(): string | null {
		const stmt = this.db.prepare(`SELECT value FROM filler_metadata WHERE key = ?`)
		const row = stmt.get("last_shutdown_at") as { value: string } | undefined
		return row?.value ?? null
	}

	/**
	 * Records the filler shutdown timestamp for missed order recovery on next startup.
	 */
	setLastShutdownTime(isoTimestamp: string): void {
		const stmt = this.db.prepare(`
			INSERT INTO filler_metadata (key, value) VALUES (?, ?)
			ON CONFLICT(key) DO UPDATE SET value = excluded.value
		`)
		stmt.run("last_shutdown_at", isoTimestamp)
	}

	close(): void {
		this.db.close()
		this.logger.debug("Limit order storage database closed")
	}
}
