import type { Order, ExecutionResult, HexString } from "@hyperbridge/sdk"
import type { Decimal } from "decimal.js"
import type { BidPlacer } from "@/core/bid-venues"
import type { SubmitBidResult } from "@/orderbook/types"

/**
 * An execution outcome that can also report a still-pooled bid, and what the orderbook said.
 *
 * `success`, `txHash`, `pending` and `error` are Hyperbridge's answer. `submitBid` can time out
 * with the extrinsic still in Hyperbridge's pool. That is neither success nor failure: the
 * extrinsic may yet land and reserve a deposit, so the bid record has to be reclaimable even
 * though `success` is false. Widened here rather than on the SDK's `ExecutionResult` so simplex
 * does not need an unreleased SDK.
 *
 * A bid goes to the HyperFX orderbook as well, and to it alone when the filler holds no BRIDGE
 * for the Hyperbridge deposit; `orderbook` is that venue's answer. A bid the orderbook holds is
 * live whatever Hyperbridge said.
 */
export interface FillResult extends ExecutionResult {
	pending?: boolean
	/**
	 * The identifier Hyperbridge files the bid under, once its op was built:
	 * `keccak256` of its calldata. It tells the bids on one order apart, and retracting the bid names it.
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
	 * Places the strategy's bid on `order` through `bids`, the venues the filler bids on
	 * (Hyperbridge and the HyperFX orderbook). Without `bids` the destination selects no
	 * solver, so there is nothing to bid on.
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
