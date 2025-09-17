import { SubstrateBlock } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { BridgeTokenSupplyService } from "@/services/bridgeTokenSupply.service"

/**
 * Handle Bridge Token Supply Indexing for Hyperbridge token
 */
export const handleBridgeTokenSupplyIndexing = wrap(async (event: SubstrateBlock): Promise<void> => {
	try {
		const chain = getHostStateMachine(chainId)

		const {
			block: {
				header: { hash },
			},
		} = event

		const blockHash = hash.toHex()
		const timestamp = await getBlockTimestamp(blockHash, chain)

		logger.info(`[handleBridgeTokenSupplyIndexing] Updating bridge token supply ${timestamp} via ${chain}`)

		await BridgeTokenSupplyService.updateTokenSupply(timestamp)

		logger.info(`Bridge token supply update completed for chain: ${chain}`)
	} catch (error) {
		// @ts-ignore
		logger.error(`[handleBridgeTokenSupplyIndexing] failed ${error.message}`)
	}
})