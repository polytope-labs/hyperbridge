import {
	decodeUserOpScale,
	fetchBidsForOrder,
	getTotalSolverBalance,
	sweepSolverLiquidity,
	weightedMedian,
	type AggregationLogger,
	type HexString,
	type LpBalance,
	type PhantomAggregation,
	type YieldVaultMap,
} from "@hyperbridge/sdk/intents-helpers"
import { extractFillDataVm2 } from "./phantom-decode"

// Strips a bytes32 token field to a 20-byte lowercase address (or normalises an address as-is).
function toAddress(token: string): HexString {
	const hex = token.toLowerCase().replace(/^0x/, "")
	const addr = hex.length > 40 ? hex.slice(-40) : hex.padStart(40, "0")
	return `0x${addr}` as HexString
}

/**
 * Aggregates every bid for a phantom order into a single price/liquidity snapshot. Lives in the
 * indexer (not the SDK) because phantom aggregation is indexer-only and its bid decoding must be
 * VM2-safe — it composes the SDK's RPC/math helpers with the indexer's ethers-based
 * {@link extractFillDataVm2}.
 *
 * Fetches the live bids via `intents_getBidsForOrder` and reads each filler's quoted output amount.
 * The liquidity-weighted median weights every quote by the solver's balance of the output token on
 * the destination chain, so a solver that can't actually deliver size carries little or no weight —
 * which is why no fill simulation is needed to filter unfillable quotes. For each bidding solver it
 * also records a full liquidity sweep — every configured yield-vault token on every supported chain
 * (raw ERC-20 + vault positions). Returns `null` when there are no decodable bids.
 */
export async function aggregatePhantomBids(params: {
	nodeUrl: string
	/** RPC URL per supported EVM chain (stateMachineId -> url); must include the destination chain. */
	evmRpcUrls: Record<string, string>
	chain: string
	gatewayAddress: string
	commitment: string
	yieldVaults: YieldVaultMap
	logger?: AggregationLogger
}): Promise<PhantomAggregation | null> {
	const { nodeUrl, evmRpcUrls, chain, gatewayAddress, commitment, yieldVaults, logger } = params

	const destUrl = evmRpcUrls[chain]
	if (!destUrl) return null

	const bids = await fetchBidsForOrder(nodeUrl, commitment)
	if (bids.length === 0) return null

	const quotes: { price: bigint; weight: bigint }[] = []
	const lpBalances: LpBalance[] = []

	for (const bid of bids) {
		if (!bid.user_op) continue
		try {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const solver = decoded.sender

			const fillData = extractFillDataVm2(decoded.callData as HexString, gatewayAddress)
			if (!fillData) continue

			// Price influence: the solver's liquidity in the output token on the destination chain.
			const outputTokenAddress = toAddress(fillData.outputToken)
			const weight = await getTotalSolverBalance(destUrl, chain, outputTokenAddress, solver, yieldVaults)
			quotes.push({ price: fillData.solverAmount, weight })

			// Full liquidity picture: every configured token on every supported chain.
			lpBalances.push(...(await sweepSolverLiquidity(evmRpcUrls, yieldVaults, solver)))
		} catch (err) {
			logger?.warn({ err, filler: bid.filler }, "Failed to process bid for price snapshot")
		}
	}

	if (quotes.length === 0) return null

	const sortedPrices = quotes.map((q) => q.price).sort((a, b) => (a < b ? -1 : a > b ? 1 : 0))

	return {
		lowestPrice: sortedPrices[0],
		highestPrice: sortedPrices[sortedPrices.length - 1],
		medianPrice: weightedMedian(quotes),
		bidCount: quotes.length,
		lpBalances,
	}
}
