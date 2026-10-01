import { Status, Transfer, RequestV2 } from "@/configs/src/types"
import { PostRequestTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { RequestService } from "@/services/request.service"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import stringify from "safe-stable-stringify"
import { VolumeService } from "@/services/volume.service"
import { getPriceDataFromEthereumLog, isERC20TransferEvent, extractAddressFromTopic } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { safeArray } from "@/utils/data.helper"
import { getHandlerMessageModules } from "@/utils/handler.helpers"

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

	await RequestService.updateStatus({
		commitment,
		chain,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		blockTimestamp,
		status: Status.TIMED_OUT,
		transactionHash,
  })

	try {
		await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

		const fromAddresses = getHandlerMessageModules(transaction?.input, "handlePostRequestTimeouts")

		for (const [index, log] of safeArray(transaction.logs).entries()) {
			if (!isERC20TransferEvent(log)) {
				continue
			}

			const value = BigInt(log.data)
			const transferId = `${log.transactionHash}-index-${index}`
			const transfer = await Transfer.get(transferId)

			if (!transfer) {
				const [_, fromTopic, toTopic] = log.topics
				const from = extractAddressFromTopic(fromTopic)
				const to = extractAddressFromTopic(toTopic)

				// Compute USD value first; skip zero-USD transfers
				const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(
					log.address,
					value,
					blockTimestamp,
				)
				if (amountValueInUSD === "0") {
					continue
				}

				await TransferService.storeTransfer({
					transactionHash: transferId,
					chain,
					value,
					from,
					to,
				})

				await VolumeService.updateVolume(`Transfer.${symbol}`, amountValueInUSD, blockTimestamp)

				const matchingContract = fromAddresses.find(
					(addr) => addr.toLowerCase() === from.toLowerCase() || addr.toLowerCase() === to.toLowerCase(),
				)

				if (matchingContract) {
					await VolumeService.updateVolume(`Contract.${matchingContract}`, amountValueInUSD, blockTimestamp)
				}
			}
		}
	} catch (error) {
		logger.error(`Error indexing volume in post request timeout: ${stringify(error)}`)
	}
})
