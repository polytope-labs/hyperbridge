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
	/**
	 * The submission timed out with the extrinsic still in Hyperbridge's pool. It
	 * may yet land and reserve a deposit, so the sweep treats it as reclaimable
	 * even though `success` is false.
	 */
	pending: boolean
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
	/** Submission is still in the pool; see {@link StoredBid.pending}. */
	pending?: boolean
	error?: string
}

export interface BidStats {
	total: number
	successful: number
	failed: number
	retracted: number
	/** Deposits still locked: successful or pending, and not yet retracted. */
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
	/** Every bid for any of `commitments`, newest first. Empty input resolves empty. */
	byCommitments(commitments: string[]): Promise<StoredBid[]>
	/** Unretracted bids whose deposit may still be locked: confirmed or pooled. */
	unretractedReclaimable(): Promise<StoredBid[]>
	/**
	 * Bids due for retraction: successful *or still pending*, unretracted, and
	 * either older than `maxAgeMs` or flagged dead. Pending counts because a
	 * pooled extrinsic that later lands reserves a deposit exactly like a
	 * confirmed one. Dead bids ignore the age cut — their deposit is
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

/**
 * `bid`: this filler's bid for the order was accepted by Hyperbridge (txHash is
 * the extrinsic hash). `filled`: this filler filled the order on chain. `lost`:
 * another filler did (reason carries its address). `executed`: a direct fill
 * attempt or a failed bid.
 */
export type ActivityType = "detected" | "bid" | "filled" | "lost" | "executed" | "skipped" | "rebalance"

/** One leg of an order as the activity feed records it. */
export interface OrderLeg {
	/** 20-byte token address; the zero address is the chain's native asset. */
	token: string
	/** Raw on-chain amount as a decimal string (bigint-safe). */
	amount: string
	/** Registry symbol when the token is known to this filler, else null. */
	symbol: string | null
	/** ERC-20 decimals when they could be read, else null (the UI then shows the raw amount). */
	decimals: number | null
}

/**
 * What the activity feed knows about an order, captured when it was detected
 * and attached to every later event for that order so each row stands alone.
 */
export interface OrderSummary {
	user: string
	/** Source chain state machine id, e.g. "EVM-8453". */
	source: string
	destination: string
	/** Hash of the transaction that placed the order on the source chain. */
	placedTxHash: string | null
	/** 20-byte referrer from the order's graffiti tag; null when unattributed. */
	referrer: string | null
	inputs: OrderLeg[]
	outputs: OrderLeg[]
	/** Order deadline (block number) as a decimal string. */
	deadline: string
}

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
	/** The order this event concerns, when known; null for rebalances and legacy rows. */
	order: OrderSummary | null
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
	/**
	 * Distinct order ids among the newest rows that carry no order summary —
	 * rows written before summaries existed. Newest first, at most `limit`.
	 */
	orderIdsMissingSummary(limit?: number): Promise<string[]>
	/** Sets the summary on every row for `orderId` that lacks one; resolves the rows it changed. */
	attachOrder(orderId: string, order: OrderSummary): Promise<ActivityEvent[]>
	/**
	 * Orders (rows sharing an order id) newest-activity first, one page at a
	 * time. `page` is 1-based; `total` counts every distinct order.
	 */
	orderHistory(page: number, pageSize: number): Promise<OrderHistoryPage>
	/** Whether any row exists for `orderId` — i.e. this filler has seen the order. */
	knowsOrder(orderId: string): Promise<boolean>
	/**
	 * Orders this filler bid on that have no on-chain outcome recorded yet: a
	 * `bid` row, or a legacy bid-time `filled` row (those carry `volumeUsd`), and
	 * neither a `lost` row nor an observed `filled` row (no `volumeUsd`). Newest first.
	 */
	unsettledOrders(limit?: number): Promise<string[]>
	/** Retypes an order's legacy bid-time `filled` rows to `bid`; resolves the rows it changed. */
	retypeLegacyBid(orderId: string): Promise<ActivityEvent[]>
}

/** One order's rows, newest first. */
export interface OrderHistoryEntry {
	orderId: string
	events: ActivityEvent[]
}

export interface OrderHistoryPage {
	page: number
	pageSize: number
	total: number
	orders: OrderHistoryEntry[]
}

// ===========================================================================
// Operator state
// ===========================================================================

/** Operator state that must survive a restart. */
export interface RuntimeState {
	/** A pause set by the operator stays set across restarts. */
	paused?: boolean
	/**
	 * The last phantom bid commitment per chain (state machine id) that may still
	 * hold a deposit. The next interval's batch retracts it; without this a restart
	 * forgot the bid and its 0.01 BRIDGE deposit was never reclaimed.
	 */
	phantomBids?: Record<string, string>
}

export interface StateStore {
	get(): Promise<RuntimeState>
	set(state: RuntimeState): Promise<void>
}
