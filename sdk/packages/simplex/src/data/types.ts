import type { HexString } from "@hyperbridge/sdk"
import type { BridgeTopUpStage, TopUpPayToken } from "@/services/BridgeTopUp"

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
	inventory: InventoryStore
	/** Directory the store keeps its files in; absent for an in-memory store. */
	readonly dataDir?: string
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
	 * The HyperFX orderbook took this bid, so a swapper can execute it whatever Hyperbridge said.
	 * The orderbook serves a bid for its TTL and nothing withdraws it, so this never changes; the
	 * bid is live there until its op's `validUntil`. False for a bid the orderbook refused or was
	 * not offered, and for rows from before bids went to the orderbook.
	 */
	onOrderbook: boolean
	/**
	 * The limit order that priced this bid and the terms it signed, written when the bid is stored
	 * and never changed. Null for a bid no limit order priced.
	 *
	 * Provenance only: nothing is held against the order. The order's on-chain budget refuses a
	 * fill past its cap, and settlement re-reads that budget rather than doing arithmetic on this.
	 */
	pricedBy: BidPricing | null
}

/** The limit order a bid drew on and what the bid signed against it. */
export interface BidPricing {
	limitOrderId: string
	/** The output the bid offered to pay, at 1e18. */
	payout: string
	/**
	 * The take the bid signed beside `payout`, in the input token's raw units. With it, settlement
	 * works out what the gateway charged: the escrow it released at `payout / take`, and which of
	 * several bids on one order a fill executed. Bids recorded before bids carried their own rate
	 * have none.
	 */
	take?: string
	/**
	 * The input token's decimals on the order's source chain, which is where `take` and the escrow
	 * a fill releases are counted. With it, settlement records what a fill took in at 1e18. Bids
	 * recorded before fills kept their input have none.
	 */
	takeDecimals?: number
}

export interface BidInsert {
	commitment: string
	/**
	 * The identifier Hyperbridge files the bid under, `keccak256` of its calldata. Bids on one
	 * incoming order share a commitment, and this is what tells them apart. It is also what
	 * retracting the bid names, so it is recorded here at placement rather than looked up later:
	 * a bid still in Hyperbridge's pool is not in its storage yet, but its retraction, sent after it
	 * from the same account, lands after it and reclaims its deposit all the same.
	 */
	bid?: string
	/** The limit order that priced this bid; see {@link StoredBid.pricedBy}. */
	pricedBy?: BidPricing
	extrinsicHash?: string
	blockHash?: string
	success: boolean
	/** Submission is still in the pool; see {@link StoredBid.pending}. */
	pending?: boolean
	error?: string
	/** The orderbook holds this bid; see {@link StoredBid.onOrderbook}. */
	onOrderbook?: boolean
}

export interface BidStats {
	total: number
	/** Bids a venue took: Hyperbridge, or the orderbook alone. */
	successful: number
	/** Bids no venue took. */
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
	 *
	 * With `rows` (the {@link StoredBid.id}s) only those bids are marked: a retraction
	 * closes the bids it sent retractions for, not one stored on the commitment while it
	 * was in flight. Without, every unretracted bid on the commitment is marked. An empty
	 * `rows` marks nothing.
	 */
	markRetracted(commitment: string, retractExtrinsicHash: string | null, rows?: readonly number[]): Promise<boolean>
	/**
	 * Flags a bid dead, so the sweep retries its retraction next cycle rather than after
	 * the TTL: its order was filled on-chain, or a retraction selected it and could not
	 * close it. With `rows` (the {@link StoredBid.id}s) only those bids are flagged;
	 * without, every unretracted bid on the commitment. An empty `rows` flags nothing.
	 * False when nothing matched.
	 */
	markDead(commitment: string, rows?: readonly number[]): Promise<boolean>
	/**
	 * Every bid a limit order priced, newest first, from each bid's `pricedBy`. What
	 * makes a `remaining` explicable to the operator: which bids drew on the order.
	 */
	byLimitOrder(limitOrderId: string, limit?: number): Promise<StoredBid[]>
	/** Newest first. Implementations should cap `limit` at a few hundred. */
	recent(limit?: number): Promise<StoredBid[]>
	/** Bids no venue took (neither Hyperbridge nor the orderbook), newest first — for debugging. */
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
	/**
	 * The EntryPoint the current posting's op was signed for. Null while nothing is
	 * posted, and on a posting stored before this was recorded, which was signed
	 * for v0.8.
	 */
	entryPoint: string | null
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
	/** Where the order's posted-op nonce starts. Defaults to "0". */
	orderNonce?: string
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
	entryPoint: string | null
	status: LimitOrderStatus
	lastError: string | null
}

/**
 * One fill of a limit order, kept apart from the bid row so an order keeps its history
 * across every resize and repost.
 */
export interface LimitOrderFill {
	id: number
	limitOrderId: string
	/** The swap order the fill settled. */
	commitment: string
	/** The identifier Hyperbridge filed the executed bid under, when known. */
	bid: string | null
	/** What the fill drew the limit order down by, at 1e18 in the token it pays. */
	amount: string
	/**
	 * The escrow the fill released to the solver, at 1e18 in the token the order takes in. Null
	 * for a fill recorded before this was kept, and for one whose event carried no inputs.
	 */
	amountIn: string | null
	/** The fill's transaction on the order's fill chain, when the event carried it. */
	transactionHash: string | null
	/**
	 * Which log of that transaction the fill is. One transaction can carry several of our
	 * fills, so the hash alone does not name one. Null for a fill recorded before this was
	 * kept, and for one whose event carried none.
	 */
	logIndex: number | null
	/** SQLite-style "YYYY-MM-DD HH:MM:SS" in UTC. */
	filledAt: string
}

export interface LimitOrderFillInsert {
	limitOrderId: string
	commitment: string
	bid?: string | null
	amount: string
	amountIn?: string | null
	transactionHash?: string | null
	logIndex?: number | null
}

/**
 * A fill with the terms of the order it drew on: what profitability is worked out from. The
 * order's `price` stands in for the rate of a fill that kept no `amountIn`.
 */
export interface LimitOrderFillRecord {
	id: number
	limitOrderId: string
	book: string
	base: string
	quote: string
	side: LimitOrderSide
	price: string
	amount: string
	amountIn: string | null
	filledAt: string
}

/**
 * Persistent record of the operator's limit orders.
 *
 * `remaining` follows the order's on-chain budget, which is what actually refuses a fill
 * past the order's size: it is lowered to that budget after each fill and by the periodic
 * tally reconciliation, and never raised.
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
	 * Lowers an `open` or `resizing` order's `remaining` to `room`, what the solver's
	 * account on chain will still pay for this order, at 1e18. Never raises it.
	 * Returns the order when it was lowered, null otherwise, including for an order in
	 * any other status.
	 *
	 * `only` narrows the statuses it may act on, like `setStatus` and `setPosting` take;
	 * it never widens them, so a status other than `open` or `resizing` is never clamped.
	 * Both callers pass it, because the two statuses mean different things. A repost
	 * holds a row in `resizing` and lowers it to the tally before it posts, so that call
	 * passes `["resizing"]`: a store that refused `resizing` would have the repost
	 * advertise the pre-fill size. A fill's sync starts a repost of its own after it
	 * lowers the row, so it passes `["open"]`: a row a repost already holds is left to
	 * it, and two reposts of one row would orphan a book entry.
	 *
	 * A custom store must apply the status check and the write atomically: a row that
	 * moved to another status after the caller read it is left as it is.
	 */
	clampRemaining(id: string, room: string, only?: readonly LimitOrderStatus[]): Promise<LimitOrder | null>
	/** Records a fill against its limit order, for its history and profit. */
	recordFill(fill: LimitOrderFillInsert): Promise<void>
	/** Every fill of every limit order, oldest first, each with its order's terms. */
	fillHistory(): Promise<LimitOrderFillRecord[]>
	/** A limit order's fills, newest first. */
	fills(limitOrderId: string, limit?: number): Promise<LimitOrderFill[]>
}

// ===========================================================================
// Operator state
// ===========================================================================

/** Operator state that must survive a restart. */
export interface RuntimeState {
	/** A pause set by the operator stays set across restarts. */
	paused?: boolean
	/** Notification rules, browser subscriptions, and the per-installation VAPID identity. */
	notifications?: NotificationRuntimeState
	/** The latest BRIDGE top-up, kept so an open order is resumed or reclaimed after a restart. */
	bridgeTopUp?: StoredBridgeTopUp
}

export interface StoredBridgeTopUp {
	id: string
	/** Source state machine id the order was placed on. */
	chain: string
	token: TopUpPayToken
	amountIn: string
	bridgeOut: string
	stage: BridgeTopUpStage
	createdAt: number
	updatedAt: number
	/** The order as placed, bigints encoded by `serializeOrder`. */
	order: string
	/** Signs the order's bid selection; it authorizes nothing beyond this order. */
	sessionPrivateKey?: HexString
	commitment?: HexString
	placeTxHash?: HexString
	/**
	 * The wallet nonce a native placement tx was sent with. Only the wallet's nonce moving past it
	 * shows the tx can no longer be mined; a node that forgot the tx shows nothing.
	 */
	placeTxNonce?: number
	/**
	 * The placement UserOp the bundler accepted when its receipt never arrived. The batch may have
	 * been mined, so the record stays pending and the op is looked up again; without it a restart
	 * would declare that nothing was escrowed while the order could still be live.
	 */
	placeUserOpHash?: HexString
	/** The EntryPoint `nonce` the op was signed with, as a decimal string (the record is JSON-persisted). */
	placeUserOpNonce?: string
	/** The EntryPoint `nonceKey` the op was signed with, as a decimal string. */
	placeUserOpNonceKey?: string
	/**
	 * Whether a UserOp placement's paymaster attached a validity deadline (Permit2 mode). A
	 * bootstrap op has none, so it can still land; without this a restart cannot tell them apart.
	 */
	placeUserOpExpires?: boolean
	/**
	 * When the placement batch was submitted, in ms. Dates a UserOp's permit window. Records from
	 * before this was recorded are dated by `stageTimes.placing`, then `createdAt`; `updatedAt` is
	 * rewritten by every save, so it never ages.
	 */
	placeSubmittedAt?: number
	/**
	 * The order's session-key address, matched against `OrderPlaced.session`. A placement receipt
	 * can carry other users' placements (a UserOp is a bundle), so only the log naming this address
	 * is this top-up's.
	 */
	placeSession?: HexString
	/** Block the paying chain was at before the placement was submitted, as a decimal string; the scan starts here. */
	placeFromBlock?: string
	/**
	 * The next block a recovery scan reads, as a decimal string. Saved after each chunk and trailing
	 * the tip by a reorg margin, so the next scan resumes here instead of re-reading the whole range.
	 */
	placeScannedTo?: string
	fillTxHash?: HexString
	reclaimTxHash?: HexString
	/**
	 * A cancel that was submitted without a receipt. It is handed back to the SDK only once a
	 * check shows it succeeded; if it reverted or can no longer be mined, it is cleared and a new
	 * cancel is sent.
	 */
	reclaimSubmission?: StoredSubmission
	/** The HyperFungibleToken request that carries the BRIDGE to Hyperbridge. */
	deliveryCommitment?: HexString
	/** The Hyperbridge extrinsic that delivered it, when this filler relayed it. */
	deliveryTxHash?: HexString
	/** Bids seen for the order. */
	bidCount?: number
	/** When each stage was entered, in ms. */
	stageTimes?: Partial<Record<BridgeTopUpStage, number>>
	/** What the order's executor reported, oldest first: bids, the selection and the fill. */
	events?: TopUpExecutionEvent[]
	/** The solver that filled the order, from the executor or the indexer. */
	filler?: HexString
	/** The BRIDGE request's progress from BSC into Hyperbridge, from the indexer. */
	deliveryStatuses?: DeliveryStatus[]
	/** The unfilled order's recovery, step by step, as the SDK's cancel flow reported it. */
	recoverySteps?: RecoveryStepRecord[]
	/** The GET request a cross-chain cancel dispatched; Hyperbridge answers it with the refund response. */
	getCommitment?: HexString
	/** Hyperbridge's finalized response, which refunds the escrow when submitted on the source. */
	claimCalldata?: HexString
	/** The filler's own refund claim, when no relayer delivered it in time. */
	claimTxHash?: HexString
	error?: string
}

/** A submission as kept in the JSON state: bigints as decimal strings. */
export type StoredSubmission =
	| { kind: "tx"; txHash: HexString; nonce: number; submittedAt: number }
	| {
			kind: "userOp"
			userOpHash: HexString
			nonce: string
			nonceKey: string
			/** Whether the paymaster attached a validity deadline; see {@link StoredBridgeTopUp.placeUserOpExpires}. */
			expires: boolean
			submittedAt: number
	  }

export interface RecoveryStepRecord {
	step:
		| "DESTINATION_FINALIZED"
		| "ORDER_CANCELED"
		| "SOURCE_FINALIZED"
		| "HYPERBRIDGE_DELIVERED"
		| "HYPERBRIDGE_FINALIZED"
		| "FUNDS_CLAIMED"
	/** When the step completed, in ms. */
	timestamp: number
	txHash?: HexString
}

export interface DeliveryStatus {
	/** Dispatched on BSC, BSC final on Hyperbridge, delivered to Hyperbridge. */
	status: "SOURCE" | "SOURCE_FINALIZED" | "DESTINATION"
	/** In ms. */
	timestamp: number
	txHash: HexString
}

export interface TopUpBidPreview {
	solverAddress: HexString
	/** Output budgets as raw token units, the token as bytes32. */
	outputs: { token: HexString; amount: string }[]
}

export interface TopUpExecutionEvent {
	status: "NEW_BID" | "BIDS_RECEIVED" | "BID_SELECTED" | "FILLED"
	/** When the filler saw it, in ms. */
	timestamp: number
	bidCount?: number
	bids?: TopUpBidPreview[]
	selectedSolver?: HexString
	userOpHash?: HexString
	txHash?: HexString
}

export interface NotificationSettings {
	/** Alert once when aggregate spendable USD-stable liquidity crosses below this value. */
	lowLiquidityThresholdUsd: number | null
	/** Alert after this filler completes a swap. */
	swaps: boolean
}

/** Transport-safe alert payload shared by Web Push and the native desktop shell. */
export interface OperatorNotification {
	title: string
	body: string
	tag: string
	/** Relative dashboard destination to open when the operator clicks the alert. */
	url: string
	/** Opaque, short-lived receipt used only to confirm native desktop test delivery. */
	receiptId?: string
}

export interface StoredPushSubscription {
	endpoint: string
	expirationTime?: number | null
	keys: { p256dh: string; auth: string }
}

export interface NotificationRuntimeState {
	settings: NotificationSettings
	vapid: { publicKey: string; privateKey: string }
	subscriptions: StoredPushSubscription[]
	/** Prevents restart/poll spam while liquidity remains below the configured threshold. */
	lowLiquidityActive?: boolean
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

// ===========================================================================
// Inventory
// ===========================================================================

/**
 * What the solver held at one moment: whole tokens per symbol, counting the wallet and the
 * vaults on every chain.
 */
export interface InventorySnapshot {
	id: number
	/** SQLite-style "YYYY-MM-DD HH:MM:SS" in UTC. */
	takenAt: string
	balances: Record<string, number>
}

/**
 * A record of what the solver held, taken about once a day.
 *
 * It is what profit is measured against: the inventory a period began with. The chain can say
 * what an account holds now but not what it held last week, so a snapshot that was never taken
 * cannot be recovered. Like {@link ActivityStore} it is observability rather than correctness:
 * a lost row costs a figure on the analytics page and nothing else.
 */
export interface InventoryStore {
	record(balances: Record<string, number>): Promise<void>
	/** The most recent snapshot, or null when none has been taken. */
	latest(): Promise<InventorySnapshot | null>
	/** Every snapshot taken at or after `from`, oldest first. */
	since(from: Date): Promise<InventorySnapshot[]>
}
