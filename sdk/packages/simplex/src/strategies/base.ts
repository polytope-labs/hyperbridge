import { Order, ExecutionResult, IntentsCoprocessor } from "@hyperbridge/sdk"
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
}
