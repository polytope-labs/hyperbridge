import type { Order, ExecutionResult, HexString } from "@hyperbridge/sdk"
import type { Decimal } from "decimal.js"
import type { BidPlacer } from "@/core/bid-placer"
import type { SubmitBidResult } from "@/orderbook/types"

/**
 * An execution outcome, and what the HyperFX orderbook said.
 *
 * Under solver selection a bid is placed when the orderbook holds it: `success` is true when the
 * orderbook accepted the bid or already had it, and `error` says why it does not otherwise. A bid
 * is no transaction, so `txHash` is unset.
 */
export interface FillResult extends ExecutionResult {
	/**
	 * The bid's identifier, once its op was built: `keccak256` of its calldata. It tells the bids
	 * on one order apart, and names the bid a fill is attributed to.
	 */
	bid?: HexString
	/** The orderbook's answer; absent when the bid was not offered to it. */
	orderbook?: SubmitBidResult
}

export interface FillerStrategy {
	name: string

	canFill(order: Order): Promise<boolean>

	calculateProfitability(order: Order): Promise<number>

	/**
	 * Places the strategy's bid on `order` through `bids`, which posts it to the HyperFX
	 * orderbook. Without `bids` the destination selects no solver, so there is nothing to bid on.
	 */
	executeOrder(order: Order, bids?: BidPlacer): Promise<FillResult>

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
