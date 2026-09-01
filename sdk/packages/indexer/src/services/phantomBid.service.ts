// Reads back what a solver declared in its phantom bids. The bids themselves are already indexed
// raw (`FillerBid.bidData`, the SCALE-encoded PackedUserOperation), so anything a bid asserts is
// recoverable from the store without persisting a decoded copy of it.
import { FillerBid, PhantomOrderV2 } from "@/configs/src/types"
import { readAllPages } from "@/utils/store.helpers"
import { decodePhantomBidDeclaration, decodeUserOpScale, type HexString } from "@hyperbridge/sdk/intents-helpers"

// How far back to look for a solver's last bid on a chain. A bidder row only survives a bid window
// the solver bid in — `updateLiquidityPools` removes the rows of solvers that stop bidding — so the
// bid is normally in the newest order, and the extra windows only cover the case where a window's
// aggregation failed and left the rows standing on an older one.
const RECENT_PHANTOM_ORDERS = 5

/**
 * The Uniswap V4 positions `solver` declared in its most recent phantom bid on `chain`, newest bid
 * first; empty when it declared none, has not bid recently, or its bid payload was never resolved.
 *
 * A bid is the only place a position is ever named, and this is how a fill re-values one between
 * bid windows. What comes back is a pointer, not a claim: the caller reads each position on-chain
 * and checks the owner, which is the same guarantee the aggregation applies before weighting a leg
 * by one. That check is what makes reading the bid raw here safe — an unsigned bid naming someone
 * else's position values nothing.
 */
export async function declaredV4Positions(chain: string, solver: string): Promise<bigint[]> {
	const target = solver.toLowerCase()

	// Newest first: one order per chain per generation interval, so this is a handful of rows.
	const orders = await PhantomOrderV2.getByFields([["chain", "=", chain]], {
		limit: RECENT_PHANTOM_ORDERS,
		orderBy: "createdAtBlock",
		orderDirection: "DESC",
	})

	for (const order of orders) {
		// Bids per order are bounded by the number of fillers, which nothing declares.
		const bids = await readAllPages((limit, offset) =>
			FillerBid.getByFields([["commitment", "=", order.id]], {
				limit,
				offset,
				// A filler may re-bid within one window; the last one it placed is the live declaration.
				orderBy: "blockNumber",
				orderDirection: "DESC",
			}),
		)
		for (const bid of bids) {
			if (!bid.bidData) continue
			let sender: string
			let paymasterAndData: string | undefined
			try {
				const decoded = decodeUserOpScale(bid.bidData as HexString)
				sender = decoded.sender
				paymasterAndData = decoded.paymasterAndData
			} catch (err) {
				// A payload that does not decode is this bid's problem; the rest still speak for the
				// solvers that placed them.
				logger.debug({ err, bid: bid.id }, "Skipping undecodable bid payload while reading declarations")
				continue
			}
			// The bid is keyed by the substrate filler, but a declaration belongs to the EVM solver
			// that signed it, which only the payload names.
			if (sender.toLowerCase() !== target) continue
			return decodePhantomBidDeclaration(paymasterAndData).uniswapV4Positions
		}
	}
	return []
}
