import type { HexString } from "@hyperbridge/sdk"
import type { OrderbookClient } from "@/orderbook/client"
import type { SubmitBidResult } from "@/orderbook/types"
import type { Logger } from "@/services/Logger"

/**
 * Where a bid goes. A strategy builds the signed fill op and hands it here. The filler passes one
 * of these to `FillerStrategy.executeOrder`.
 */
export interface BidPlacer {
	/**
	 * @param commitment - The swap order the bid is on.
	 * @param userOp - The signed fill UserOp, SCALE-encoded.
	 * @returns The orderbook's answer. A request that got no answer comes back as a `failed`
	 *   result with code `REQUEST_FAILED`; this never rejects.
	 */
	place(commitment: HexString, userOp: HexString): Promise<SubmitBidResult>
}

/**
 * Posts bids to the HyperFX orderbook, the only venue simplex bids on.
 *
 * The orderbook accepts a bid only from a solver holding an active limit order on the leg's book
 * filling on the order's destination (`NO_LIMIT_ORDER`), serves it for `serverInfo.bidTtlSecs`, and
 * cannot withdraw it. The op's own `validUntil` bounds when it can execute.
 */
export class OrderbookBidPlacer implements BidPlacer {
	constructor(
		private readonly orderbook: Pick<OrderbookClient, "submitBid">,
		private readonly logger: Logger,
	) {}

	async place(commitment: HexString, userOp: HexString): Promise<SubmitBidResult> {
		try {
			const result = await this.orderbook.submitBid(userOp)
			if (result.kind === "rejected") {
				this.logger.info({ commitment, code: result.code, message: result.message }, "The orderbook refused the bid")
			} else if (result.kind === "failed") {
				this.logger.warn({ commitment, code: result.code, message: result.message }, "The orderbook could not take the bid")
			}
			return result
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err)
			this.logger.warn({ commitment, err }, "Could not post the bid to the orderbook")
			return { kind: "failed", code: "REQUEST_FAILED", message, retryable: true }
		}
	}
}

/** Whether the orderbook holds the bid after this answer: it took it now, or already had it. */
export function orderbookHoldsBid(result: SubmitBidResult | undefined): boolean {
	return result?.kind === "accepted" || result?.kind === "unchanged"
}

/** Why the orderbook does not hold the bid, as one line for the bid record; undefined when it does. */
export function bidRefusal(result: SubmitBidResult): string | undefined {
	if (result.kind === "rejected" || result.kind === "failed") return `${result.code}: ${result.message}`
	return undefined
}
