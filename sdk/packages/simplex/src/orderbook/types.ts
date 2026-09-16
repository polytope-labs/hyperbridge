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

/** `serverInfo` and `books` together, which is how simplex reads them. */
export interface OrderbookLimits {
	serverInfo: ServerInfo
	books: Book[]
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
	/** `advertisedSize < quotedSize`: the balance is binding, not the quote. */
	resized?: boolean
	/** False until the next balance cycle has confirmed the solver can cover the quote. */
	backed?: boolean
}

/**
 * Why the orderbook refused an order. `MIN_ORDER_SIZE` and `TTL_TOO_SHORT` are
 * prevented by validation before posting; `REPLAYED` and `ORDER_EXISTS` are
 * answered by bumping the nonce; the rest are encoding bugs on our side.
 *
 * `UNSUPPORTED_SOURCE_CHAIN` is the exception: a declared source chain has to
 * register the order's input symbol on the server, which is the server's own
 * config and nothing here can check ahead of asking.
 */
export type RejectionCode =
	| "MALFORMED_USER_OP"
	| "NO_FILL_ORDER"
	| "UNSUPPORTED_CHAIN"
	| "UNSUPPORTED_SHAPE"
	| "NOT_PHANTOM"
	| "MISSING_VALID_UNTIL"
	| "TTL_TOO_SHORT"
	| "UNSUPPORTED_PAIR"
	| "BAD_SIGNATURE"
	| "BAD_NONCE_BINDING"
	| "REPLAYED"
	| "ORDER_EXISTS"
	| "TOO_MANY_ORDERS"
	| "MIN_ORDER_SIZE"
	| "MISSING_DECLARATION"
	| "EMPTY_DECLARATION"
	| "UNSUPPORTED_SOURCE_CHAIN"

export type SubmitOrderResult =
	| { kind: "accepted"; order: PostedOrder; surfaced: boolean }
	| { kind: "unchanged"; order: PostedOrder }
	| { kind: "rejected"; code: RejectionCode; message: string }
	| { kind: "failed"; code: string; message: string; retryable: boolean }

/** Why a signed message was refused, shared by `cancelOrder` and `heartbeat`. */
export type MessageRejectionCode =
	| "BAD_SIGNATURE"
	| "SOLVER_MISMATCH"
	| "SIGNATURE_EXPIRED"
	| "SIGNATURE_REUSED"
	| "UNKNOWN_ORDER"
	| "UNKNOWN_SOLVER"

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
