import { SubstrateBlock } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { TokenPriceService } from "@/services/token-price.service"
import { CoinGeckoTokenListService } from "@/services/coingeckoTokenList.service"

/**
 * Handle Price Indexing for all registered tokens on a supported chain
 */
export const handlePriceIndexing = wrap(async (event: SubstrateBlock): Promise<void> => {
	try {
		const chain = getHostStateMachine(chainId)

		const {
			block: {
				header: { hash },
			},
		} = event

		const blockHash = hash.toHex()
		const timestamp = await getBlockTimestamp(blockHash, chain)

		logger.info(`[handlePriceIndexing] Updating prices ${timestamp} via ${chain}`)

		await CoinGeckoTokenListService.sync(timestamp)
		await TokenPriceService.initializePriceIndexing(timestamp)

		logger.info(`Price update completed for chain: ${chain}`)
	} catch (error) {
		// @ts-ignore
		logger.error(`[handlePriceIndexing] failed ${error.message}`)
	}
})
