import { EthereumBlock } from "@subql/types-ethereum"
import { PendingStatusService } from "@/services/pendingStatus.service"
import { wrap } from "@/utils/event.utils"

const FLUSH_LIMIT = 10

/**
 * EVM block handler that drains up to FLUSH_LIMIT pending status rows
 * on each chain's own instance. The historical-by-timestamp filter
 * hides rows written by other chains from the Hyperbridge instance, so
 * each chain has to clean up the rows it wrote.
 */
export const handlePendingStatusFlushEvm = wrap(async (event: EthereumBlock): Promise<void> => {
	const blockNumber = event.number.toString()
	logger.info(
		`[handlePendingStatusFlushEvm] chain=${chainId} entered at block #${blockNumber}, limit=${FLUSH_LIMIT}`,
	)
	try {
		await PendingStatusService.flushBatch(FLUSH_LIMIT)
		logger.info(
			`[handlePendingStatusFlushEvm] chain=${chainId} completed for block #${blockNumber}`,
		)
	} catch (error) {
		// @ts-ignore
		logger.error(
			`[handlePendingStatusFlushEvm] chain=${chainId} failed at block #${blockNumber}: ${error.message}`,
		)
	}
})
