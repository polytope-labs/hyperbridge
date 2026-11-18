import type { HexString } from "@hyperbridge/sdk"

export type RebalanceMethod = "cctp" | "usdt0" | "cex"

/**
 * Options for rebalancing transfers (CCTP, USDT0, or CEX).
 * - For CCTP/USDT0: coin is implicit (CCTP=USDC, USDT0=USDT), recipientAddress is optional
 * - For unified router/CEX: coin is required for routing, recipientAddress is ignored
 */
export interface RebalanceOptions {
	/** Amount in human-readable format (e.g., "100.00") */
	amount: string
	/** Source chain in state machine format (e.g., "EVM-137") */
	source: string
	/** Destination chain in state machine format (e.g., "EVM-42161") */
	destination: string
	/** Optional recipient address (defaults to sender's address) - only used for CCTP/USDT0, ignored for CEX */
	recipientAddress?: HexString
}

/**
 * UnifiedRebalanceOptions is kept for broader compatibility.
 */
export type UnifiedRebalanceOptions = RebalanceOptions & {
	coin: "USDC" | "USDT"
}

/** One transfer a route is asked to make. `amount` is in human units. */
export interface RouteTransfer {
	asset: string
	amount: string
	source: string
	destination: string
}

/**
 * A way of moving an asset between two chains. Each route says for itself where it runs, so
 * `RebalancingService` keeps no chain list of its own that could drift from the route's.
 */
export interface RebalanceRoute {
	readonly method: RebalanceMethod
	/** The asset symbols the route carries. */
	readonly assets: readonly string[]
	/**
	 * Whether the route also sends a transaction from the solver wallet on the destination
	 * chain. Transfers that send on the same chain must not run at once.
	 */
	readonly sendsOnDestination: boolean
	/**
	 * The token this route moves for `asset` on `chain`, or undefined where the route does
	 * not run. A transfer is possible when both of its chains answer.
	 */
	tokenOn(asset: string, chain: string): HexString | undefined
	/** Makes the transfer. Resolves once the funds are delivered, and rejects when they are not. */
	send(transfer: RouteTransfer): Promise<{ txHash?: string }>
}

export * from "./binance"
export * from "./cctp"
export * from "./usdt0"
