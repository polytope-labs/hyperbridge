/**
 * The persistence contract for a running filler.
 *
 * Everything simplex must remember across a restart lives behind these three
 * stores: submitted bids (so deposits can be reclaimed), the order-activity
 * feed, and a scrap of operator state. Nothing else in the filler touches a
 * database, a file, or a data directory.
 *
 * Every method is async. The bundled SQLite adapter is synchronous underneath
 * and simply returns resolved promises — the async signature exists so a
 * consumer can back these with Postgres, Redis, or a hosted API. Making the
 * interface synchronous would have limited "bring your own backend" to backends
 * that are local and synchronous, which is nearly the same as not having the
 * interface at all.
 */
export interface SimplexDataStore {
	bids: BidStore
	activity: ActivityStore
	state: StateStore
	/** Releases any underlying handles. Called by `Simplex.stop()`. */
	close?(): Promise<void>
}

// ===========================================================================
// Bids
// ===========================================================================

/** A bid submitted to Hyperbridge's solver-selection pallet. */
export interface StoredBid {
	id: number
	commitment: string
	extrinsicHash: string | null
	blockHash: string | null
	success: boolean
	error: string | null
	/** SQLite-style "YYYY-MM-DD HH:MM:SS" in UTC. Sorts lexicographically. */
	createdAt: string
	retracted: boolean
	retractedAt: string | null
	retractExtrinsicHash: string | null
	/** The order was seen filled on-chain, so this bid can never win — reclaim its deposit now. */
	dead: boolean
}

export interface BidInsert {
	commitment: string
	extrinsicHash?: string
	blockHash?: string
	success: boolean
	error?: string
}

export interface BidStats {
	total: number
	successful: number
	failed: number
	retracted: number
	pendingRetraction: number
}

/**
 * Persistent record of every bid submitted to Hyperbridge.
 *
 * This is the money-critical store: a successful bid locks a deposit that is
 * only reclaimable by retracting it, and the retraction sweep finds what to
 * reclaim by querying here. An implementation that loses writes leaks deposits,
 * so `store` must be durable before it resolves — do not buffer it in memory
 * and flush later.
 */
export interface BidStore {
	store(bid: BidInsert): Promise<void>
	/** The most recent bid for a commitment, or null. */
	byCommitment(commitment: string): Promise<StoredBid | null>
	/** Successful bids that have not been retracted — every reclaimable deposit. */
	unretractedSuccessful(): Promise<StoredBid[]>
	/**
	 * Bids due for retraction: successful, unretracted, and either older than
	 * `maxAgeMs` or flagged dead. Dead bids ignore the age cut — their deposit is
	 * reclaimable immediately, so a failed attempt retries on the next sweep
	 * rather than waiting out the TTL.
	 */
	expiredUnretracted(maxAgeMs: number): Promise<StoredBid[]>
	/**
	 * Marks a bid retracted. Also called when the chain reports `BidNotFound`:
	 * bids only leave the pallet by retraction, so "no bid" means there is
	 * nothing left to reclaim and no hash is available.
	 *
	 * Resolves false when no unretracted bid matched (already retracted, or never
	 * stored) — the caller treats that as success, not an error.
	 */
	markRetracted(commitment: string, retractExtrinsicHash: string | null): Promise<boolean>
	/** Flags a bid dead (its order was filled on-chain). False when nothing matched. */
	markDead(commitment: string): Promise<boolean>
	/** Newest first. Implementations should cap `limit` at a few hundred. */
	recent(limit?: number): Promise<StoredBid[]>
	/** Failed bids, newest first — for debugging. */
	failed(limit?: number): Promise<StoredBid[]>
	byDateRange(from: Date, to: Date): Promise<StoredBid[]>
	stats(): Promise<BidStats>
}

// ===========================================================================
// Activity
// ===========================================================================

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

export type WalletTxKind = "send" | "sweep" | "redeem"

/** One outbound transaction from the filler wallet (operator send, vault sweep/redeem). */
export interface WalletTx {
	id: number
	ts: number
	kind: WalletTxKind
	chainId: number | null
	token: string | null
	amount: string | null
	to: string | null
	txHash: string
	sponsored: boolean | null
}

/**
 * Append-only feed of what the filler did, backing the operator dashboard.
 *
 * Unlike {@link BidStore} this is observability, not correctness: dropping a
 * row costs a line in the activity view and nothing else, so an implementation
 * may prune, sample, or cap history freely.
 */
export interface ActivityStore {
	/** Appends an event and resolves the stored row (its `id` and `ts` assigned). */
	record(event: ActivityInsert): Promise<ActivityEvent>
	/** Newest first; pass `beforeId` to page backwards. */
	recent(limit?: number, beforeId?: number): Promise<ActivityEvent[]>
	/** Fill events carrying both a tx hash and a chain, newest first. */
	fills(limit?: number): Promise<ActivityEvent[]>
	recordWalletTx(tx: Omit<WalletTx, "id" | "ts">): Promise<void>
	walletTxs(limit?: number): Promise<WalletTx[]>
}

// ===========================================================================
// Operator state
// ===========================================================================

/** Operator state that must survive a restart. */
export interface RuntimeState {
	/** A pause set by the operator stays set across restarts. */
	paused?: boolean
}

export interface StateStore {
	get(): Promise<RuntimeState>
	set(state: RuntimeState): Promise<void>
}
