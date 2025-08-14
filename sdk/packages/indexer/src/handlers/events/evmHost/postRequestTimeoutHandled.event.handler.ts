import { Status, Transfer } from "@/configs/src/types"
import { PostRequestTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { RequestService } from "@/services/request.service"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import stringify from "safe-stable-stringify"
import { VolumeService } from "@/services/volume.service"
import { getPriceDataFromEthereumLog, isERC20TransferEvent } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { safeArray } from "@/utils/data.helper"

/**
 * Handles the PostRequestTimeoutHandled event
 */
export const handlePostRequestTimeoutHandledEvent = wrap(async (event: PostRequestTimeoutHandledLog): Promise<void> => {
	if (!event.args) return

	const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
	const { commitment, dest } = args

	logger.info(
		`Handling PostRequestTimeoutHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain: string = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	try {
		await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

		await RequestService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockHash: block.hash,
			blockTimestamp,
			status: Status.TIMED_OUT,
			transactionHash,
		})

		for (const log of safeArray(transaction.logs)) {
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
		logger.error(`Error updating handling post request timeout: ${stringify(error)}`)
	}
})
