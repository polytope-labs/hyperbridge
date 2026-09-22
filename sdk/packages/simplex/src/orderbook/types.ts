import type { HexString } from "@hyperbridge/sdk"

/**
 * The HyperFX orderbook's wire shapes, narrowed to the fields simplex reads.
 *
 * Every `BigInt` the schema declares arrives as a decimal string and stays one
 * here: these values are U256 and only ever compared or stored, so parsing them
 * eagerly would buy nothing and lose precision on the way back out.
 */

/**
 * The shortest TTL the orderbook documents itself as accepting.
 *
 * A fallback for validating config before anything has been read from the
 * server: the live `serverInfo.minOrderTtlSecs` is authoritative and a posting
 * is checked against that, not this.
 */
export const MIN_ORDER_TTL_SECONDS = 900

/**
 * How long a limit order lives when neither the request nor `[orderbook]
 * defaultTtlSecs` says: 365 days. An order is inventory the operator opens and
 * closes; it should not lapse on its own within a working session.
 */
export const DEFAULT_LIMIT_ORDER_TTL_SECONDS = 365 * 24 * 60 * 60

/**
 * The heartbeat period to fall back on when `serverInfo` cannot be read.
 *
 * Deliberately short. Heartbeating more often than the server asks costs one
 * request; heartbeating less often costs a suspension, and the live
 * `heartbeatIntervalSecs` is used whenever it can be read.
 */
export const FALLBACK_HEARTBEAT_INTERVAL_MS = 60_000

export interface Book {
	id: string
	base: string
	quote: string
}

export interface TokenMinSize {
	symbol: string
	size: string
}

export interface ServerInfo {
	minOrderTtlSecs: number
	heartbeatIntervalSecs: number
	signatureSkewSecs: number
	maxBatchSize: number
	minOrderSizes: TokenMinSize[]
	/**
	 * Every chain the orderbook serves, by state machine id: the chains orders
	 * fill on, and the source chains they may declare.
	 */
	chains: string[]
	eip712DomainName: string
	eip712DomainVersion: string
}

/** One token on one chain, as the orderbook's own registry has it. */
export interface ChainToken {
	symbol: string
	/** Its decimals there: USDC and USDT carry 18 on BNB Chain and 6 on the rest. */
	decimals: number
}

/** One chain the orderbook serves, with the tokens it registers on it. */
export interface ChainInfo {
	/** State machine id, such as `EVM-8453`. */
	id: string
	name: string
	tokens: ChainToken[]
}

/** `serverInfo`, `books` and `chains` together, which is how simplex reads them. */
export interface OrderbookLimits {
	serverInfo: ServerInfo
	books: Book[]
	chains: ChainInfo[]
}

export interface PostedOrder {
	commitment: HexString
	side: "BID" | "ASK"
	status: "ACTIVE" | "SUSPENDED"
	price: string
	quotedAmount: string
	advertisedSize: string
	expiresAt: string
	acceptedSources: string[]
	/** The balance, not the quote, is what binds `advertisedSize`. */
	resized?: boolean
	/**
	 * Whether the solver's balance covers the whole quote. Also false before any
	 * balance has been read, which is why {@link PostedOrder.validatedAt} decides
	 * whether a false here means anything.
	 */
	backed?: boolean
	/**
	 * The last cycle that read this order's balance, absent until one has. An
	 * order surfaces at its quoted size meanwhile.
	 */
	validatedAt?: string | null
}

/**
 * Every code the orderbook's `RejectionCode` enum can send, listed rather than
 * written as a union so a test can hold it against the published schema.
 *
 * Why the orderbook refuses an order. `MIN_ORDER_SIZE` and `TTL_TOO_SHORT` are
 * prevented by validation before posting; `REPLAYED` and `ORDER_EXISTS` are
 * answered by bumping the nonce; the rest are encoding bugs on our side.
 *
 * `UNSUPPORTED_SOURCE_CHAIN` is the exception: a declared source chain has to
 * register the order's input symbol on the server, which is the server's own
 * config and nothing here can check ahead of asking.
 */
export const REJECTION_CODES = [
	"MALFORMED_USER_OP",
	"NO_FILL_ORDER",
	"UNSUPPORTED_CHAIN",
	"UNSUPPORTED_SHAPE",
	"NOT_PHANTOM",
	"TTL_TOO_SHORT",
	"UNSUPPORTED_PAIR",
	"BAD_SIGNATURE",
	"BAD_NONCE_BINDING",
	"REPLAYED",
	"ORDER_EXISTS",
	"TOO_MANY_ORDERS",
	"MIN_ORDER_SIZE",
	"MISSING_DECLARATION",
	"EMPTY_DECLARATION",
	"UNSUPPORTED_SOURCE_CHAIN",
] as const

export type RejectionCode = (typeof REJECTION_CODES)[number]

/**
 * Why the orderbook could not decide an op at all. Ingest reads no chain, so it
 * is the database or the submission running out of time, and both are worth
 * another attempt.
 */
export const FAILURE_CODES = ["DATABASE_UNAVAILABLE", "TIMEOUT"] as const

export type FailureCode = (typeof FAILURE_CODES)[number]

export type SubmitOrderResult =
	| { kind: "accepted"; order: PostedOrder; surfaced: boolean }
	| { kind: "unchanged"; order: PostedOrder }
	| { kind: "rejected"; code: RejectionCode; message: string }
	| { kind: "failed"; code: FailureCode | "REQUEST_FAILED"; message: string; retryable: boolean }

/** Why a signed message was refused, shared by `cancelOrder` and `heartbeat`. */
export const MESSAGE_REJECTION_CODES = [
	"BAD_SIGNATURE",
	"SOLVER_MISMATCH",
	"SIGNATURE_EXPIRED",
	"SIGNATURE_REUSED",
	"UNKNOWN_ORDER",
	"UNKNOWN_SOLVER",
] as const

export type MessageRejectionCode = (typeof MESSAGE_REJECTION_CODES)[number]

export type SolverStatus = "ACTIVE" | "SUSPENDED"

export type HeartbeatResult =
	| { kind: "accepted"; status: SolverStatus; reactivatedOrders: number; heartbeatDueBy: string }
	| { kind: "rejected"; code: MessageRejectionCode; message: string }

/** One page of the solver's own orders, as reconciliation walks them. */
export interface PostedOrderPage {
	orders: PostedOrder[]
	/** Cursor for the next page, absent on the last one. */
	cursor?: string
	status: SolverStatus
}

export type CancelOrderResult =
	| { kind: "cancelled"; commitment: HexString }
	| { kind: "rejected"; code: MessageRejectionCode; message: string }
	/**
	 * The request never got an answer. Distinct from `UNKNOWN_ORDER`, which is the
	 * orderbook saying the entry is already gone: here the entry may well still be
	 * live, and a caller that treats the two alike orphans it.
	 */
	| { kind: "failed"; message: string }
