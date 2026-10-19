import type { DatabaseSync } from "node:sqlite"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import { columnNames } from "./schema"
import type {
	LimitOrder,
	LimitOrderFill,
	LimitOrderFillInsert,
	LimitOrderFillRecord,
	LimitOrderFilter,
	LimitOrderInsert,
	LimitOrderPosting,
	LimitOrderStatus,
	LimitOrderStore,
} from "@/data/types"

/** Column list shared by every SELECT that returns a LimitOrder. */
const LIMIT_ORDER_COLUMNS = `
	id,
	book,
	base,
	quote,
	side,
	fill_chain as fillChain,
	price,
	size,
	remaining,
	accepted_sources as acceptedSources,
	ttl_secs as ttlSecs,
	expires_at as expiresAt,
	status,
	commitment,
	order_nonce as orderNonce,
	entry_point as entryPoint,
	book_expires_at as bookExpiresAt,
	book_price as bookPrice,
	last_error as lastError,
	created_at as createdAt,
	updated_at as updatedAt
`

/**
 * SQLite-backed {@link LimitOrderStore}, sharing `bids.db` with the bid store.
 *
 * Amounts are stored as the decimal strings they arrive as. SQLite's own
 * integers top out at 64 bits, which a 1e18 amount overruns as soon as the size
 * passes about 18 tokens, so they are never arithmetic in SQL.
 */
/** `AND status IN (...)` for a guarded write, or nothing when the caller did not ask. */
function statusGuard(only?: readonly LimitOrderStatus[]): string {
	return only && only.length > 0 ? ` AND status IN (${only.map(() => "?").join(", ")})` : ""
}

export class SqliteLimitOrderStore implements LimitOrderStore {
	private logger: Logger

	constructor(
		private db: DatabaseSync,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("limit-order-store")
		this.initializeSchema()
	}

	private initializeSchema(): void {
		this.db.exec(`
			CREATE TABLE IF NOT EXISTS limit_orders (
				id TEXT PRIMARY KEY,
				book TEXT NOT NULL,
				base TEXT NOT NULL,
				quote TEXT NOT NULL,
				side TEXT NOT NULL,
				fill_chain TEXT NOT NULL,
				price TEXT NOT NULL,
				size TEXT NOT NULL,
				remaining TEXT NOT NULL,
				-- Never read or written: limit orders keep no local holds. Kept, and reset to
				-- '0' on every start, so a rollback to a build that still reads it finds the
				-- column and no stale hold.
				reserved TEXT NOT NULL DEFAULT '0',
				accepted_sources TEXT NOT NULL,
				ttl_secs INTEGER NOT NULL,
				expires_at TEXT,
				status TEXT NOT NULL,
				commitment TEXT,
				order_nonce TEXT NOT NULL DEFAULT '0',
				entry_point TEXT,
				book_expires_at TEXT,
				book_price TEXT,
				last_error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				updated_at TEXT NOT NULL DEFAULT (datetime('now'))
			);

			CREATE INDEX IF NOT EXISTS idx_limit_orders_status ON limit_orders(status);
			CREATE INDEX IF NOT EXISTS idx_limit_orders_fill_chain ON limit_orders(fill_chain);
			CREATE INDEX IF NOT EXISTS idx_limit_orders_commitment ON limit_orders(commitment);

			CREATE TABLE IF NOT EXISTS limit_order_fills (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				limit_order_id TEXT NOT NULL,
				commitment TEXT NOT NULL,
				bid TEXT,
				amount TEXT NOT NULL,
				amount_in TEXT,
				transaction_hash TEXT,
				log_index INTEGER,
				filled_at TEXT NOT NULL DEFAULT (datetime('now'))
			);

			CREATE INDEX IF NOT EXISTS idx_limit_order_fills_order ON limit_order_fills(limit_order_id);
		`)

		// A build that kept holds left some in this column, and this one never releases the
		// holds of bids it retracts. Cleared on every start, so a rollback to that build
		// does not inherit them: it adds `reserved` to the room it clamps `remaining` to,
		// and a stale figure would keep sizing bids past what the chain allows.
		// A read first, so a start with nothing to clear takes no write lock.
		if (
			columnNames(this.db, "limit_orders").has("reserved") &&
			this.db.prepare("SELECT 1 FROM limit_orders WHERE reserved <> '0' LIMIT 1").get()
		) {
			this.db.exec("UPDATE limit_orders SET reserved = '0' WHERE reserved <> '0'")
		}

		// A database written before postings recorded their EntryPoint gets the
		// column in place. Its rows read null, which is v0.8.
		if (!columnNames(this.db, "limit_orders").has("entry_point")) {
			this.db.exec("ALTER TABLE limit_orders ADD COLUMN entry_point TEXT")
			this.logger.info({ column: "entry_point" }, "Migrated limit order storage schema")
		}

		// A database created before fills kept what they took in needs the column added in place.
		// Its existing rows stay null, and are priced at their order's rate when read. The column
		// is nullable with no default, so a solver still on the earlier version can go on writing
		// fills to the same file.
		if (!columnNames(this.db, "limit_order_fills").has("amount_in")) {
			this.db.exec("ALTER TABLE limit_order_fills ADD COLUMN amount_in TEXT")
			this.logger.info({ column: "amount_in" }, "Migrated limit order fill schema")
		}

		// The same for the log a fill came from. Nullable with no default, so a solver still on the
		// earlier version can go on writing fills to the same file; its rows read null.
		if (!columnNames(this.db, "limit_order_fills").has("log_index")) {
			this.db.exec("ALTER TABLE limit_order_fills ADD COLUMN log_index INTEGER")
			this.logger.info({ column: "log_index" }, "Migrated limit order fill schema")
		}
	}

	// biome-ignore lint/suspicious/noExplicitAny: raw sqlite row
	private toLimitOrder(row: any): LimitOrder {
		return { ...row, acceptedSources: JSON.parse(row.acceptedSources) }
	}

	private read(id: string): LimitOrder | null {
		const row = this.db.prepare(`SELECT ${LIMIT_ORDER_COLUMNS} FROM limit_orders WHERE id = ?`).get(id)
		return row ? this.toLimitOrder(row) : null
	}

	async create(order: LimitOrderInsert): Promise<LimitOrder> {
		this.db
			.prepare(`
				INSERT INTO limit_orders (
					id, book, base, quote, side, fill_chain, price, size, remaining,
					accepted_sources, ttl_secs, expires_at, order_nonce, status
				)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'open')
			`)
			.run(
				order.id,
				order.book,
				order.base,
				order.quote,
				order.side,
				order.fillChain,
				order.price,
				order.size,
				order.size,
				JSON.stringify(order.acceptedSources),
				order.ttlSecs,
				order.expiresAt ?? null,
				order.orderNonce ?? "0",
			)
		this.logger.info({ id: order.id, book: order.book, side: order.side }, "Limit order created")
		return this.read(order.id)!
	}

	async get(id: string): Promise<LimitOrder | null> {
		return this.read(id)
	}

	async list(filter: LimitOrderFilter = {}): Promise<LimitOrder[]> {
		const clauses: string[] = []
		const args: string[] = []
		for (const [column, value] of [
			["status", filter.status],
			["fill_chain", filter.fillChain],
			["book", filter.book],
		] as const) {
			if (value === undefined) continue
			clauses.push(`${column} = ?`)
			args.push(value)
		}
		const where = clauses.length > 0 ? `WHERE ${clauses.join(" AND ")}` : ""
		const rows = this.db
			.prepare(`SELECT ${LIMIT_ORDER_COLUMNS} FROM limit_orders ${where} ORDER BY created_at DESC`)
			.all(...args)
		return rows.map((row) => this.toLimitOrder(row))
	}

	async open(): Promise<LimitOrder[]> {
		return this.list({ status: "open" })
	}

	async setPosting(
		id: string,
		posting: LimitOrderPosting,
		only?: readonly LimitOrderStatus[],
	): Promise<LimitOrder | null> {
		const result = this.db
			.prepare(`
				UPDATE limit_orders
				SET commitment = ?, book_expires_at = ?, book_price = ?, order_nonce = ?,
				    entry_point = ?, status = ?, last_error = ?, updated_at = datetime('now')
				WHERE id = ?${statusGuard(only)}
			`)
			.run(
				posting.commitment,
				posting.bookExpiresAt,
				posting.bookPrice,
				posting.orderNonce,
				posting.entryPoint,
				posting.status,
				posting.lastError,
				id,
				...(only ?? []),
			)
		return result.changes === 1 ? this.read(id) : null
	}

	async setStatus(
		id: string,
		status: LimitOrderStatus,
		lastError: string | null = null,
		only?: readonly LimitOrderStatus[],
	): Promise<LimitOrder | null> {
		const result = this.db
			.prepare(
				`UPDATE limit_orders SET status = ?, last_error = ?, updated_at = datetime('now') WHERE id = ?${statusGuard(only)}`,
			)
			.run(status, lastError, id, ...(only ?? []))
		return result.changes === 1 ? this.read(id) : null
	}

	async recordFill(fill: LimitOrderFillInsert): Promise<void> {
		this.db
			.prepare(
				"INSERT INTO limit_order_fills (limit_order_id, commitment, bid, amount, amount_in, transaction_hash, log_index) VALUES (?, ?, ?, ?, ?, ?, ?)",
			)
			.run(
				fill.limitOrderId,
				fill.commitment,
				fill.bid ?? null,
				fill.amount,
				fill.amountIn ?? null,
				fill.transactionHash ?? null,
				fill.logIndex ?? null,
			)
	}

	async fillHistory(): Promise<LimitOrderFillRecord[]> {
		// Joined rather than left-joined: a fill is only written against an order that exists, and
		// one without its order's side and rate could not be priced anyway.
		return this.db
			.prepare(
				`SELECT f.id, f.limit_order_id as limitOrderId, o.book, o.base, o.quote, o.side, o.price,
					f.amount, f.amount_in as amountIn, f.filled_at as filledAt
				FROM limit_order_fills f JOIN limit_orders o ON o.id = f.limit_order_id
				ORDER BY f.filled_at, f.id`,
			)
			.all() as unknown as LimitOrderFillRecord[]
	}

	async fills(limitOrderId: string, limit = 100): Promise<LimitOrderFill[]> {
		return this.db
			.prepare(
				`SELECT id, limit_order_id as limitOrderId, commitment, bid, amount, amount_in as amountIn, transaction_hash as transactionHash, log_index as logIndex,
					filled_at as filledAt
				FROM limit_order_fills WHERE limit_order_id = ? ORDER BY id DESC LIMIT ?`,
			)
			.all(limitOrderId, Math.min(Math.max(limit, 1), 500)) as unknown as LimitOrderFill[]
	}

	async clampRemaining(id: string, room: string): Promise<LimitOrder | null> {
		const order = this.read(id)
		if (!order || (order.status !== "open" && order.status !== "resizing")) return null
		const target = BigInt(room) > 0n ? BigInt(room) : 0n
		if (target >= BigInt(order.remaining)) return null

		// Guarded on everything the decision was read against. A guard that fails
		// leaves the row as the other writer left it, which is no worse than not
		// having clamped, so it answers null and the next pass decides afresh.
		const result = this.db
			.prepare(`
				UPDATE limit_orders SET remaining = ?, updated_at = datetime('now')
				WHERE id = ? AND remaining = ? AND status IN ('open', 'resizing')
			`)
			.run(target.toString(), id, order.remaining)
		return result.changes === 1 ? this.read(id) : null
	}
}
