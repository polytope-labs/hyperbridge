import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status, Transfer } from "@/configs/src/types"
import { GetRequestHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { GetRequestService } from "@/services/getRequest.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { getPriceDataFromEthereumLog, isERC20TransferEvent } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { VolumeService } from "@/services/volume.service"
import { safeArray } from "@/utils/data.helper"

/**
 * Handles the GetRequestHandled event from EVMHost
 */
export const handleGetRequestHandledEvent = wrap(async (event: GetRequestHandledLog): Promise<void> => {
	if (!event.args) return

	const { args, block, transaction, transactionHash, blockNumber, blockHash } = event
	const { relayer: relayer_id, commitment } = args

	logger.info(
		`Handling GetRequestHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	try {
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain, blockTimestamp)

		await GetRequestService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockHash: block.hash,
			blockTimestamp,
			status: Status.DESTINATION,
			transactionHash,
		})

		for (const log of safeArray(transaction?.logs)) {
			if (!isERC20TransferEvent(log)) {
				continue
			}

			const transfer = await Transfer.get(log.transactionHash)

			if (!transfer) {
				const [_, from, to] = log.topics
				await TransferService.storeTransfer({
					transactionHash: log.transactionHash,
					chain,
					value: BigInt(log.data),
					from,
					to,
				})

				const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(log.address, BigInt(log.data))
				await VolumeService.updateVolume(`Transfer.${symbol}`, amountValueInUSD, blockTimestamp)
			}
		}
	} catch (error) {
		logger.error(`Error handling GetRequestHandled Event: ${error}`)
	}
})
