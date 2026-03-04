import { OrderV2, ExecutionResult, IntentsCoprocessor } from "@hyperbridge/sdk"
import { Decimal } from "decimal.js"

/** Supported token types for same-token execution */
export type SupportedTokenType = "USDT" | "USDC"

export interface FillerStrategy {
	name: string

	canFill(order: OrderV2): Promise<boolean>

	calculateProfitability(order: OrderV2): Promise<number>

	executeOrder(order: OrderV2, hyperbridge?: IntentsCoprocessor): Promise<ExecutionResult>

	/**
	 * Optional hook for strategies to provide a USD value for the full input basket.
	 * Returns null when the strategy cannot or does not want to price the order.
	 */
	getOrderUsdValue?(order: OrderV2): Promise<{ inputUsd: Decimal; outputUsd: Decimal } | null>
}
