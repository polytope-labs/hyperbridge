import type {
	ActivityEvent,
	ActivityInsert,
	ActivityStore,
	OrderHistoryPage,
	OrderSummary,
	BidInsert,
	BidStats,
	BidStore,
	RuntimeState,
	SimplexDataStore,
	StateStore,
	StoredBid,
	WalletTx,
} from "./types"

/**
 * Rows kept per collection before pruning. Bid rows are pruned settled-first —
 * a row whose deposit is still reclaimable (successful or pending, unretracted)
 * survives any cap, since the retraction sweep finds work by querying this store.
 */
const MAX_ROWS = 10_000

/** Newest-first reads are capped here however large a `limit` the caller passes. */
const MAX_LIMIT = 500

/**
 * "YYYY-MM-DD HH:MM:SS" in UTC — the format SQLite's `datetime('now')` produces.
 * Kept identical across adapters because `StoredBid.createdAt` is compared
 * lexicographically to find expired bids, and `toISOString()`'s "T" separator
 * (ASCII 84) sorts after a space (ASCII 32), which would make every bid look
 * expired.
 */
export function sqliteDatetime(date: Date): string {
	return date.toISOString().replace("T", " ").replace(/\.\d{3}Z$/, "")
}

/** A bid whose deposit may be locked on Hyperbridge: confirmed, or still pooled. */
function reclaimable(row: StoredBid): boolean {
	return row.success || row.pending
}

function capLimit(limit: number): number {
	return Math.min(Math.max(limit, 1), MAX_LIMIT)
}

class MemoryBidStore implements BidStore {
	private rows: StoredBid[] = []
	private nextId = 1

	async store(bid: BidInsert): Promise<void> {
		this.rows.push({
			id: this.nextId++,
			commitment: bid.commitment,
			extrinsicHash: bid.extrinsicHash ?? null,
			blockHash: bid.blockHash ?? null,
			success: bid.success,
			pending: bid.pending === true,
			error: bid.error ?? null,
			createdAt: sqliteDatetime(new Date()),
			retracted: false,
			retractedAt: null,
			retractExtrinsicHash: null,
			dead: false,
		})
		if (this.rows.length > MAX_ROWS) {
			// Only ever drop rows with nothing left to reclaim. A successful or pending
			// bid holds a Hyperbridge deposit that the sweep finds by querying this
			// store, so evicting one by age strands it — and this store is the
			// library's default, which is exactly where that is least survivable.
			const settled = this.rows.filter((row) => row.retracted || (!row.success && !row.pending))
			const excess = this.rows.length - MAX_ROWS
			const drop = new Set(settled.slice(0, excess))
			if (drop.size > 0) this.rows = this.rows.filter((row) => !drop.has(row))
		}
	}

	async byCommitment(commitment: string): Promise<StoredBid | null> {
		// Newest wins: a commitment can be re-bid after a retraction.
		for (let i = this.rows.length - 1; i >= 0; i--) {
			if (this.rows[i].commitment === commitment) return { ...this.rows[i] }
		}
		return null
	}

	async unretractedReclaimable(): Promise<StoredBid[]> {
		return this.rows.filter((row) => reclaimable(row) && !row.retracted).map((row) => ({ ...row }))
	}

	async expiredUnretracted(maxAgeMs: number): Promise<StoredBid[]> {
		const cutoff = sqliteDatetime(new Date(Date.now() - maxAgeMs))
		return this.rows
			.filter((row) => reclaimable(row) && !row.retracted && (row.dead || row.createdAt < cutoff))
			.map((row) => ({ ...row }))
	}

	async markRetracted(commitment: string, retractExtrinsicHash: string | null): Promise<boolean> {
		let changed = false
		for (const row of this.rows) {
			if (row.commitment !== commitment || row.retracted) continue
			row.retracted = true
			row.retractedAt = sqliteDatetime(new Date())
			row.retractExtrinsicHash = retractExtrinsicHash
			changed = true
		}
		return changed
	}

	async markDead(commitment: string): Promise<boolean> {
		let changed = false
		for (const row of this.rows) {
			if (row.commitment !== commitment || row.retracted || row.dead) continue
			row.dead = true
			changed = true
		}
		return changed
	}

	async recent(limit = 100): Promise<StoredBid[]> {
		return this.rows
			.slice(-capLimit(limit))
			.reverse()
			.map((row) => ({ ...row }))
	}

	async byCommitments(commitments: string[]): Promise<StoredBid[]> {
		const wanted = new Set(commitments)
		return this.rows
			.filter((row) => wanted.has(row.commitment))
			.reverse()
			.map((row) => ({ ...row }))
	}

	async failed(limit = 100): Promise<StoredBid[]> {
		return this.rows
			.filter((row) => !row.success)
			.slice(-capLimit(limit))
			.reverse()
			.map((row) => ({ ...row }))
	}

	async byDateRange(from: Date, to: Date): Promise<StoredBid[]> {
		const start = sqliteDatetime(from)
		const end = sqliteDatetime(to)
		return this.rows
			.filter((row) => row.createdAt >= start && row.createdAt <= end)
			.reverse()
			.map((row) => ({ ...row }))
	}

	async stats(): Promise<BidStats> {
		return {
			total: this.rows.length,
			successful: this.rows.filter((row) => row.success).length,
			failed: this.rows.filter((row) => !row.success).length,
			retracted: this.rows.filter((row) => row.retracted).length,
			pendingRetraction: this.rows.filter((row) => reclaimable(row) && !row.retracted).length,
		}
	}
}

class MemoryActivityStore implements ActivityStore {
	private events: ActivityEvent[] = []
	private walletRows: WalletTx[] = []
	private nextEventId = 1
	private nextWalletId = 1

	async record(event: ActivityInsert): Promise<ActivityEvent> {
		const row: ActivityEvent = {
			id: this.nextEventId++,
			ts: Date.now(),
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
		this.events.push(row)
		if (this.events.length > MAX_ROWS) this.events.splice(0, this.events.length - MAX_ROWS)
		return { ...row }
	}

	async recent(limit = 100, beforeId?: number): Promise<ActivityEvent[]> {
		const pool = beforeId === undefined ? this.events : this.events.filter((row) => row.id < beforeId)
		return pool
			.slice(-capLimit(limit))
			.reverse()
			.map((row) => ({ ...row }))
	}

	async fills(limit = 100): Promise<ActivityEvent[]> {
		return this.events
			.filter((row) => row.type === "filled" && row.txHash !== null && row.chainId !== null)
			.slice(-capLimit(limit))
			.reverse()
			.map((row) => ({ ...row }))
	}

	async orderIdsMissingSummary(limit = 500): Promise<string[]> {
		const ids: string[] = []
		for (let i = this.events.length - 1; i >= 0 && ids.length < capLimit(limit); i--) {
			const row = this.events[i]
			if (row.orderId && row.order === null && !ids.includes(row.orderId)) ids.push(row.orderId)
		}
		return ids
	}

	async attachOrder(orderId: string, order: OrderSummary): Promise<ActivityEvent[]> {
		const changed: ActivityEvent[] = []
		for (const row of this.events) {
			if (row.orderId === orderId && row.order === null) {
				row.order = order
				changed.push({ ...row })
			}
		}
		return changed
	}

	async knowsOrder(orderId: string): Promise<boolean> {
		return this.events.some((row) => row.orderId === orderId)
	}

	async unsettledOrders(limit = 500): Promise<string[]> {
		const isBid = (row: ActivityEvent) => row.type === "bid" || (row.type === "filled" && row.volumeUsd !== null)
		const isSettled = (row: ActivityEvent) => row.type === "lost" || (row.type === "filled" && row.volumeUsd === null)
		const ids: string[] = []
		for (let i = this.events.length - 1; i >= 0 && ids.length < capLimit(limit); i--) {
			const orderId = this.events[i].orderId
			if (!orderId || ids.includes(orderId)) continue
			const rows = this.events.filter((row) => row.orderId === orderId)
			if (rows.some(isBid) && !rows.some(isSettled)) ids.push(orderId)
		}
		return ids
	}

	async retypeLegacyBid(orderId: string): Promise<ActivityEvent[]> {
		const changed: ActivityEvent[] = []
		for (const row of this.events) {
			if (row.orderId === orderId && row.type === "filled" && row.volumeUsd !== null) {
				row.type = "bid"
				changed.push({ ...row })
			}
		}
		return changed
	}

	async orderHistory(page: number, pageSize: number): Promise<OrderHistoryPage> {
		const size = capLimit(pageSize)
		const current = Math.max(1, Math.floor(page))
		// Distinct order ids by newest activity, then the page's slice.
		const ordered: string[] = []
		const seen = new Set<string>()
		for (let i = this.events.length - 1; i >= 0; i--) {
			const orderId = this.events[i].orderId
			if (orderId && !seen.has(orderId)) {
				seen.add(orderId)
				ordered.push(orderId)
			}
		}
		const ids = ordered.slice((current - 1) * size, current * size)
		return {
			page: current,
			pageSize: size,
			total: ordered.length,
			orders: ids.map((orderId) => ({
				orderId,
				events: this.events
					.filter((row) => row.orderId === orderId)
					.reverse()
					.map((row) => ({ ...row })),
			})),
		}
	}

	async recordWalletTx(tx: Omit<WalletTx, "id" | "ts">): Promise<void> {
		this.walletRows.push({ ...tx, id: this.nextWalletId++, ts: Date.now() })
		if (this.walletRows.length > MAX_ROWS) this.walletRows.splice(0, this.walletRows.length - MAX_ROWS)
	}

	async walletTxs(limit = 100): Promise<WalletTx[]> {
		return this.walletRows
			.slice(-capLimit(limit))
			.reverse()
			.map((row) => ({ ...row }))
	}
}

class MemoryStateStore implements StateStore {
	private state: RuntimeState = {}

	async get(): Promise<RuntimeState> {
		return { ...this.state }
	}

	async set(state: RuntimeState): Promise<void> {
		this.state = { ...state }
	}
}

/**
 * Process-local data store — the default when `Simplex.start` is given no
 * `data` option.
 *
 * Nothing survives the process. That is fine for tests, short-lived embedded
 * fillers and watch-only observers, but a filler that submits bids should use a
 * durable store: a lost bid record is a Hyperbridge deposit nobody retracts.
 * Settled rows are evicted past a cap; live bids (deposits still reclaimable) are never dropped.
 */
export class MemoryDataStore implements SimplexDataStore {
	readonly bids: BidStore = new MemoryBidStore()
	readonly activity: ActivityStore = new MemoryActivityStore()
	readonly state: StateStore = new MemoryStateStore()
}
