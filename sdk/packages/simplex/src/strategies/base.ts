import { Order, ExecutionResult, IntentsCoprocessor, TokenInfo } from "@hyperbridge/sdk"
import { Decimal } from "decimal.js"

/** Supported token types for same-token execution */
export type SupportedTokenType = "USDT" | "USDC"

export interface FillerStrategy {
	name: string

	canFill(order: Order): Promise<boolean>

	calculateProfitability(order: Order): Promise<number>

	executeOrder(order: Order, hyperbridge?: IntentsCoprocessor): Promise<ExecutionResult>

	/**
	 * Optional hook for strategies to provide a USD value for the full input basket.
	 * Returns null when the strategy cannot or does not want to price the order.
	 */
	getOrderUsdValue?(order: Order): Promise<{ inputUsd: Decimal } | null>

	/**
	 * Optional confirmation policy for this strategy.
	 * If absent, no confirmation waiting is required (e.g. same-chain strategies).
	 */
	confirmationPolicy?: {
		getConfirmationBlocks: (chainId: number, amountUsd: number) => number
	}

	/**
	 * Quote fill outputs for a phantom (expired same-chain) order.
	 * Returns the token amounts the strategy would provide without gas estimation.
	 * Returns null when the strategy cannot handle this token pair.
	 */
	quotePhantomFill?(order: Order): Promise<TokenInfo[] | null>
}
