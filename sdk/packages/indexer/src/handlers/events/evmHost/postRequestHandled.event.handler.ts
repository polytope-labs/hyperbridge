import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status, Transfer } from "@/configs/src/types"
import { PostRequestHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { RequestService } from "@/services/request.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { VolumeService } from "@/services/volume.service"
import { getPriceDataFromEthereumLog, isERC20TransferEvent } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { safeArray } from "@/utils/data.helper"

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
export const handlePostRequestHandledEvent = wrap(async (event: PostRequestHandledLog): Promise<void> => {
	if (!event.args) return

	const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
	const { relayer: relayer_id, commitment } = args

	logger.info(
		`Handling PostRequestHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	try {
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain, blockTimestamp)

		await RequestService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockHash: block.hash,
			blockTimestamp,
			status: Status.DESTINATION,
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
		console.error(`Error handling PostRequestHandled event: ${stringify(error)}`)
	}
})
