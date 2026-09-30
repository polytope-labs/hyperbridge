import { EthereumBlock } from "@subql/types-ethereum"

import { indexSolverInventoryBlock } from "@/services/solverInventory.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

/**
 * Every EVM block: turns watchlist requests into tracked solvers, runs the genesis read for newly
 * discovered ones, and advances the chain's inventory head with its periodic revaluation. Runs
 * before the block's log handlers, which is what lets a read pinned to this block stand in for
 * every log in it.
 *
 * Best-effort per block: reads that fail leave their solvers for a later block, and nothing here
 * should stop the chain from indexing.
 */
export const handleSolverInventoryBlock = wrap(async (block: EthereumBlock): Promise<void> => {
	const chain = getHostStateMachine(chainId)
	try {
		await indexSolverInventoryBlock(chain, BigInt(block.number), BigInt(block.timestamp))
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.error(`[handleSolverInventoryBlock] chain=${chainId} failed at block #${block.number}: ${message}`)
	}
})
