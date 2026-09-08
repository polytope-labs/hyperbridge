import { SubstrateBlock } from "@subql/types"
import { foldInventoryReadings } from "@/services/liquidityPool.service"
import { wrap } from "@/utils/event.utils"

/**
 * Every Hyperbridge block, folds the inventory readings the EVM nodes have published since the
 * last pass into the pool rows. This runs on the Hyperbridge node only, because that node is the
 * single writer of the pool family: the phantom snapshot writes them here, and so does this.
 *
 * Best-effort per block: a failure costs one block of depth freshness and the next block retries
 * the same readings, since applying one is idempotent.
 */
export const handleInventoryFold = wrap(async (block: SubstrateBlock): Promise<void> => {
	const blockNumber = block.block.header.number.toBigInt()
	try {
		await foldInventoryReadings({ blockNumber })
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.error(`[handleInventoryFold] chain=${chainId} failed at block #${blockNumber}: ${message}`)
	}
})
