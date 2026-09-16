import type { DatabaseSync } from "node:sqlite"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type {
	LimitOrder,
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
	reserved,
	accepted_sources as acceptedSources,
	ttl_secs as ttlSecs,
	expires_at as expiresAt,
	status,
	commitment,
	order_nonce as orderNonce,
	book_expires_at as bookExpiresAt,
	book_price as bookPrice,
	last_error as lastError,
	created_at as createdAt,
	updated_at as updatedAt
`

/**
 * A guarded write that did not apply because another writer got there first.
 *
 * Only reachable with a second process on one `bids.db`: everything here is
 * synchronous and nothing awaits between the read and the write. It is an error
 * rather than a silent no-op because both writes it guards move real money.
 */
export class LimitOrderWriteError extends Error {}

/**
 * SQLite-backed {@link LimitOrderStore}, sharing `bids.db` with the bid store.
 *
 * Amounts are stored as the decimal strings they arrive as. SQLite's own
 * integers top out at 64 bits, which a 1e18 amount overruns as soon as the size
 * passes about 18 tokens, so they are never arithmetic in SQL.
 */
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
				reserved TEXT NOT NULL DEFAULT '0',
				accepted_sources TEXT NOT NULL,
				ttl_secs INTEGER NOT NULL,
				expires_at TEXT,
				status TEXT NOT NULL,
				commitment TEXT,
				order_nonce TEXT NOT NULL DEFAULT '0',
				book_expires_at TEXT,
				book_price TEXT,
				last_error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				updated_at TEXT NOT NULL DEFAULT (datetime('now'))
			);

			CREATE INDEX IF NOT EXISTS idx_limit_orders_status ON limit_orders(status);
			CREATE INDEX IF NOT EXISTS idx_limit_orders_fill_chain ON limit_orders(fill_chain);
			CREATE INDEX IF NOT EXISTS idx_limit_orders_commitment ON limit_orders(commitment);
		`)
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
					accepted_sources, ttl_secs, expires_at, status
				)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'open')
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

	async setPosting(id: string, posting: LimitOrderPosting): Promise<LimitOrder | null> {
		this.db
			.prepare(`
				UPDATE limit_orders
				SET commitment = ?, book_expires_at = ?, book_price = ?, order_nonce = ?,
				    status = ?, last_error = ?, updated_at = datetime('now')
				WHERE id = ?
			`)
			.run(
				posting.commitment,
				posting.bookExpiresAt,
				posting.bookPrice,
				posting.orderNonce,
				posting.status,
				posting.lastError,
				id,
			)
		return this.read(id)
	}

	async setStatus(id: string, status: LimitOrderStatus, lastError: string | null = null): Promise<LimitOrder | null> {
		this.db
			.prepare("UPDATE limit_orders SET status = ?, last_error = ?, updated_at = datetime('now') WHERE id = ?")
			.run(status, lastError, id)
		return this.read(id)
	}

	async reserve(id: string, amount: string): Promise<boolean> {
		const order = this.read(id)
		if (!order || order.status !== "open") return false
		const reserved = BigInt(order.reserved) + BigInt(amount)
		if (reserved > BigInt(order.remaining)) return false

		// Guarded on the `reserved` this decision was read against, so a caller that
		// moved it in between loses here instead of overcommitting the order. The
		// arithmetic cannot happen in SQL: a 1e18 amount overruns a 64-bit integer.
		const result = this.db
			.prepare(`
				UPDATE limit_orders SET reserved = ?, updated_at = datetime('now')
				WHERE id = ? AND reserved = ? AND status = 'open'
			`)
			.run(reserved.toString(), id, order.reserved)
		return result.changes === 1
	}

	async drawDown(id: string, amount: string): Promise<LimitOrder | null> {
		const order = this.read(id)
		if (!order) return null
		// Floored: a fill that somehow delivered more than the order had left has
		// nothing further to give, and a negative remaining would read as capacity.
		const remaining = BigInt(order.remaining) - BigInt(amount)
		const result = this.db
			.prepare("UPDATE limit_orders SET remaining = ?, updated_at = datetime('now') WHERE id = ? AND remaining = ?")
			.run((remaining > 0n ? remaining : 0n).toString(), id, order.remaining)
		// A guard that fails is another writer moving `remaining` between the read
		// and the write, which only a second process on one database can do. Losing
		// it quietly would leave the order advertising output it has already paid,
		// so it is reported rather than swallowed.
		if (result.changes !== 1) throw new LimitOrderWriteError(`Another writer moved limit order '${id}' mid draw-down`)
		return this.read(id)
	}

	async release(id: string, amount: string): Promise<void> {
		const order = this.read(id)
		if (!order) return
		// Floored at zero: a double release would otherwise leave a negative
		// reservation, which hands out capacity the order does not have.
		const reserved = BigInt(order.reserved) - BigInt(amount)
		const result = this.db
			.prepare("UPDATE limit_orders SET reserved = ?, updated_at = datetime('now') WHERE id = ? AND reserved = ?")
			.run((reserved > 0n ? reserved : 0n).toString(), id, order.reserved)
		if (result.changes !== 1) throw new LimitOrderWriteError(`Another writer moved limit order '${id}' mid release`)
	}
}
