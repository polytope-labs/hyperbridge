import { SubstrateBlock } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { CoinGeckoTokenListService } from "@/services/coingeckoTokenList.service"

/**
 * Handle Token Indexing for all tokens on supported chains
 */
export const handleTokenIndexing = wrap(async (event: SubstrateBlock): Promise<void> => {
	try {
		const chain = getHostStateMachine(chainId)

		const {
			block: {
				header: { hash },
			},
		} = event

		const blockHash = hash.toHex()
		const timestamp = await getBlockTimestamp(blockHash, chain)

		logger.info(`[handleTokenIndexing] Syncing token list ${timestamp} via ${chain}`)

		await CoinGeckoTokenListService.sync(timestamp)

		logger.info(`Token list sync completed for chain: ${chain}`)
	} catch (error) {
		// @ts-ignore
		logger.error(`[handleTokenIndexing] failed ${error.message}`)
	}
})