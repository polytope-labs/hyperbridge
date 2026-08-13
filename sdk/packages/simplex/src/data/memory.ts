import type {
	ActivityEvent,
	ActivityInsert,
	ActivityStore,
	BidInsert,
	BidStats,
	BidStore,
	RuntimeState,
	SimplexDataStore,
	StateStore,
	StoredBid,
	WalletTx,
} from "./types"

/** Rows kept per collection before the oldest are dropped. Matches the SQLite adapter. */
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
			error: bid.error ?? null,
			createdAt: sqliteDatetime(new Date()),
			retracted: false,
			retractedAt: null,
			retractExtrinsicHash: null,
			dead: false,
		})
		if (this.rows.length > MAX_ROWS) this.rows.splice(0, this.rows.length - MAX_ROWS)
	}

	async byCommitment(commitment: string): Promise<StoredBid | null> {
		// Newest wins: a commitment can be re-bid after a retraction.
		for (let i = this.rows.length - 1; i >= 0; i--) {
			if (this.rows[i].commitment === commitment) return { ...this.rows[i] }
		}
		return null
	}

	async unretractedSuccessful(): Promise<StoredBid[]> {
		return this.rows.filter((row) => row.success && !row.retracted).map((row) => ({ ...row }))
	}

	async expiredUnretracted(maxAgeMs: number): Promise<StoredBid[]> {
		const cutoff = sqliteDatetime(new Date(Date.now() - maxAgeMs))
		return this.rows
			.filter((row) => row.success && !row.retracted && (row.dead || row.createdAt < cutoff))
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
			pendingRetraction: this.rows.filter((row) => row.success && !row.retracted).length,
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
 * Rows are capped so a long-running process cannot grow unboundedly.
 */
export class MemoryDataStore implements SimplexDataStore {
	readonly bids: BidStore = new MemoryBidStore()
	readonly activity: ActivityStore = new MemoryActivityStore()
	readonly state: StateStore = new MemoryStateStore()
}
