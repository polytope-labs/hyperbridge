import { HexString } from "@hyperbridge/sdk"

export type RebalanceMethod = "cctp" | "usdt0" | "cex"

export interface RouteDecision {
	method: RebalanceMethod
	reason: string
}

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

export * from "./binance"
export * from "./cctp"
export * from "./usdt0"
