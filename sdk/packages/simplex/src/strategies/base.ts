import type { Order, ExecutionResult, IntentsCoprocessor, TokenInfo } from "@hyperbridge/sdk"

/**
 * An execution outcome that can also report a still-pooled bid.
 *
 * `submitBid` can time out with the extrinsic still in Hyperbridge's pool. That
 * is neither success nor failure: the extrinsic may yet land and reserve a
 * deposit, so the bid record has to be reclaimable even though `success` is
 * false. Widened here rather than on the SDK's `ExecutionResult` so simplex does
 * not need an unreleased SDK.
 */
export interface FillResult extends ExecutionResult {
	pending?: boolean
}
import type { Decimal } from "decimal.js"

export interface FillerStrategy {
	name: string

	canFill(order: Order): Promise<boolean>

	calculateProfitability(order: Order): Promise<number>

	executeOrder(order: Order, hyperbridge?: IntentsCoprocessor): Promise<FillResult>

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
