import { SubstrateBlock } from "@subql/types"

import { pollSolverWatchlist } from "@/services/solverWatchlist.service"
import { wrap } from "@/utils/event.utils"

/**
 * Every Hyperbridge block while the node is near chain head: polls the HyperFX orderbook's watchlist
 * and queues the solvers it has not asked about before. One request covers every chain. Runs on the
 * Hyperbridge node only, so one node polls rather than every EVM node.
 */
export const handleSolverWatchlistPoll = wrap(async (block: SubstrateBlock): Promise<void> => {
	const blockNumber = block.block.header.number.toBigInt()
	try {
		await pollSolverWatchlist({ blockNumber, blockTime: block.timestamp })
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.error(`[handleSolverWatchlistPoll] chain=${chainId} failed at block #${blockNumber}: ${message}`)
	}
})
