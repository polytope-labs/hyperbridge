import { SubstrateBlock } from "@subql/types"
import { PendingStatusService } from "@/services/pendingStatus.service"
import { wrap } from "@/utils/event.utils"

const FLUSH_LIMIT = 10

/**
 * On each newly indexed Hyperbridge block, attempt to flush up to
 * FLUSH_LIMIT pending status rows whose parent entity now exists. Picks up
 * any rows the per-request `flushPendingStatuses(commitment)` calls missed
 * (e.g. when the status event was indexed after the request was created on
 * another chain, or when a chain reorg dropped the original flush).
 */
export const handlePendingStatusFlush = wrap(async (event: SubstrateBlock): Promise<void> => {
	const blockNumber = event.block.header.number.toString()
	const blockHash = event.block.header.hash.toHex()
	logger.info(
		`[handlePendingStatusFlush] entered at block #${blockNumber} (${blockHash}), limit=${FLUSH_LIMIT}`,
	)
	try {
		await PendingStatusService.flushBatch(FLUSH_LIMIT)
		logger.info(`[handlePendingStatusFlush] completed for block #${blockNumber}`)
	} catch (error) {
		// @ts-ignore
		logger.error(`[handlePendingStatusFlush] failed at block #${blockNumber}: ${error.message}`)
	}
})
