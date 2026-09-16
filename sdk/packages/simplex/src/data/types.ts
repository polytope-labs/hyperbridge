/**
 * The persistence contract for a running filler.
 *
 * Everything simplex must remember across a restart lives behind these four
 * stores: submitted bids (so deposits can be reclaimed), the order-activity
 * feed, the operator's limit orders, and a scrap of operator state. Nothing
 * else in the filler touches a database, a file, or a data directory.
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
	limitOrders: LimitOrderStore
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
	/** The identifier Hyperbridge files the bid under; see {@link BidInsert.bid}. */
	bid: string | null
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
	/**
	 * What this bid holds against the operator's limit orders, at 1e18, in the
	 * order the payout draws on them. Empty for a bid placed before limit orders
	 * priced anything, and empty again once the hold has been settled, whether it
	 * was released because the bid lost or converted because the bid filled.
	 *
	 * More than one when a swap drew on several levels: the orderbook quotes a
	 * same-chain swapper across every level that can fill the trade together, so a
	 * bid may be meeting several of the operator's own orders at once.
	 */
	reservations: LimitOrderHold[]
}

/** One limit order and what a bid holds against it, at 1e18. */
export interface LimitOrderHold {
	limitOrderId: string
	amount: string
}

export interface BidInsert {
	commitment: string
	/**
	 * The identifier Hyperbridge files the bid under, `keccak256` of its calldata. It is what
	 * retracting the bid names, so it is recorded here at placement rather than looked up later:
	 * a bid still in Hyperbridge's pool is not in its storage yet, but its retraction, sent after it
	 * from the same account, lands after it and reclaims its deposit all the same.
	 */
	bid?: string
	/** What this bid holds against the limit orders that priced it, best first. */
	reservations?: LimitOrderHold[]
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
	/**
	 * Takes what this bid holds, exactly once, and returns it.
	 *
	 * A losing bid gives its holds back and a winning one converts them into
	 * draw-downs, and both routes end at the same bid row — a bid that won is still
	 * retracted eventually, by the stale sweep, so an unguarded release would undo
	 * a conversion that already happened. Whichever settles first claims them here;
	 * the other gets an empty list and does nothing.
	 */
	claimReservation(commitment: string): Promise<LimitOrderHold[]>
	/**
	 * Every bid that drew on a limit order, newest first. What makes a `remaining`
	 * explicable to the operator: which bids took the difference.
	 */
	byLimitOrder(limitOrderId: string, limit?: number): Promise<StoredBid[]>
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
	/** What left the wallet: symbol and decimal amount (sends, sweeps, redeems). */
	token: string | null
	amount: string | null
	to: string | null
	txHash: string
	sponsored: boolean | null
	/** What came back: vault shares for a sweep, the underlying for a redeem. */
	tokenIn?: string | null
	amountIn?: string | null
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
	/** Sweep and redeem rows recorded without amounts (before the ledger carried them). Newest first. */
	walletTxsWithoutAmounts(limit?: number): Promise<WalletTx[]>
	/** Fills in the amount fields of one ledger row. */
	updateWalletTx(id: number, patch: Pick<WalletTx, "token" | "amount" | "to" | "tokenIn" | "amountIn">): Promise<void>
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
// Limit orders
// ===========================================================================

/** Which way round a limit order trades its book's pair. */
export type LimitOrderSide = "BID" | "ASK"

/**
 * `open`: live, and bids may draw on it. `resizing`: a repost is in flight after
 * a fill, so the orderbook entry may be missing until it lands. `filled`: worked
 * down past the dust floor. `cancelled`: withdrawn by the operator. `expired`:
 * past the operator's own `expiresAt` and swept off the book. `rejected`: the
 * orderbook refused it and `lastError` says why.
 */
export type LimitOrderStatus = "open" | "resizing" | "filled" | "cancelled" | "expired" | "rejected"

/**
 * One of the operator's limit orders.
 *
 * This record is what simplex prices against and draws down; the orderbook entry
 * is a derived copy that expires and is reposted. Amounts and prices are decimal
 * strings at 1e18, the unit the orderbook takes and returns, whatever decimals
 * the tokens use on their own chains.
 */
export interface LimitOrder {
	/** Stable across every repost, unlike `commitment`. */
	id: string
	/** Orderbook book id, with the symbols it resolved to. */
	book: string
	base: string
	quote: string
	side: LimitOrderSide
	/** Where simplex fills, as a state machine id. */
	fillChain: string
	/** Quote per 1 base: the rate simplex signs, before the orderbook's fee haircut. */
	price: string
	/** The output simplex offered to pay when the order was created. */
	size: string
	/** Output not yet delivered. */
	remaining: string
	/** Output promised to bids that have neither filled nor been retracted. */
	reserved: string
	/** Source chains this order accepts swaps from. Never empty. */
	acceptedSources: string[]
	/** TTL written into each posting. */
	ttlSecs: number
	/** Operator expiry for the limit order itself, independent of the posting's. */
	expiresAt: string | null
	status: LimitOrderStatus
	/** The current posting's commitment, absent while nothing is live. */
	commitment: string | null
	/** Bumped on every repost, so each posting hashes differently. */
	orderNonce: string
	/** When the current posting expires, as the orderbook reported it. */
	bookExpiresAt: string | null
	/** `Order.price` from the orderbook, which shades `price` by the protocol fee. */
	bookPrice: string | null
	/** The last rejection, as "CODE: message". */
	lastError: string | null
	/** SQLite-style "YYYY-MM-DD HH:MM:SS" in UTC. Sorts lexicographically. */
	createdAt: string
	updatedAt: string
}

/** What the operator supplies; everything else is derived or defaulted. */
export interface LimitOrderInsert {
	id: string
	book: string
	base: string
	quote: string
	side: LimitOrderSide
	fillChain: string
	price: string
	size: string
	acceptedSources: string[]
	ttlSecs: number
	expiresAt?: string | null
}

export interface LimitOrderFilter {
	status?: LimitOrderStatus
	fillChain?: string
	book?: string
}

/** The fields a posting writes back, applied together so a half-posted row is never visible. */
export interface LimitOrderPosting {
	commitment: string | null
	bookExpiresAt: string | null
	bookPrice: string | null
	orderNonce: string
	status: LimitOrderStatus
	lastError: string | null
}

/**
 * Persistent record of the operator's limit orders.
 *
 * This is inventory, not a cache: `remaining` and `reserved` are what stop two
 * chains from paying out the same liability twice, so a store that loses writes
 * overcommits real money.
 */
export interface LimitOrderStore {
	create(order: LimitOrderInsert): Promise<LimitOrder>
	get(id: string): Promise<LimitOrder | null>
	list(filter?: LimitOrderFilter): Promise<LimitOrder[]>
	/** Every `open` order, which is what the matcher prices against. */
	open(): Promise<LimitOrder[]>
	/**
	 * Records what the orderbook did with the current posting.
	 *
	 * `only` guards the write on the status the row still holds, and answers null
	 * when it has moved on. A posting is a slow round trip, and an operator's
	 * cancel or an expiry sweep that lands first must not be undone by an answer
	 * that was already in flight.
	 */
	setPosting(id: string, posting: LimitOrderPosting, only?: readonly LimitOrderStatus[]): Promise<LimitOrder | null>
	setStatus(
		id: string,
		status: LimitOrderStatus,
		lastError?: string | null,
		only?: readonly LimitOrderStatus[],
	): Promise<LimitOrder | null>
	/**
	 * Adds `amount` to `reserved`, but only while the order is `open` and
	 * `remaining - reserved` still covers it. Resolves false when it does not.
	 *
	 * This is the one method that must not be a read followed by a write. Two
	 * chains bidding against the same limit order at once would both see room in
	 * the gap, and between them promise more output than the order has.
	 */
	reserve(id: string, amount: string): Promise<boolean>
	/** Gives a reservation back, after a bid was retracted, lost or found dead. */
	release(id: string, amount: string): Promise<void>
	/**
	 * Works the order down by output that has actually been delivered, floored at
	 * zero. Returns the order as it now stands, or null when there is none.
	 */
	drawDown(id: string, amount: string): Promise<LimitOrder | null>
	/**
	 * Runs `settle` as one unit where the backend can.
	 *
	 * A fill claims what its bid held, works the orders down by what went out and
	 * gives the rest back, and a crash between those leaves the hold released
	 * against an order that was never drawn down. The bid rows live in the same
	 * database as the limit orders, which is what lets one transaction cover both.
	 *
	 * Only store calls belong inside: they are synchronous underneath, so nothing
	 * else interleaves on the connection, which would not hold for a network call.
	 */
	transaction<T>(settle: () => Promise<T>): Promise<T>
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
	/** Replaces the whole record. Keys absent from `state` are dropped. */
	set(state: RuntimeState): Promise<void>
	/**
	 * Merges `patch` into the stored record atomically, leaving keys it does not
	 * name untouched. Optional: `patchRuntimeState` falls back to a read and a
	 * `set` for stores that cannot do better, which loses a key written
	 * concurrently. The bundled SQLite store implements it.
	 */
	patch?(patch: Partial<RuntimeState>): Promise<RuntimeState>
}
