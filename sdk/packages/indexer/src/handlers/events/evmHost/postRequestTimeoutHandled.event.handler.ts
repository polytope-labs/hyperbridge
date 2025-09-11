import { Status, Transfer, Request } from "@/configs/src/types"
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
import HandlerV1Abi from "@/configs/abis/HandlerV1.abi.json"
import { PostRequestTimeoutMessage } from "@/types/ismp"
import { Interface } from "@ethersproject/abi"

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

		let fromAddresses = [] as string[]

		if (transaction?.input) {
			const { name, args } = new Interface(HandlerV1Abi).parseTransaction({ data: transaction.input })

			if (name === "handlePostRequestTimeouts" && args && args.length > 1) {
				const { timeouts } = args[1] as PostRequestTimeoutMessage
				for (const timeout of timeouts) {
					const { from } = timeout
					fromAddresses.push(from)
				}
			}
		}

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
				await TransferService.storeTransfer({
					transactionHash: transferId,
					chain,
					value,
					from,
					to,
				})

				const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(
					log.address,
					value,
					blockTimestamp,
				)
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
		logger.error(`Error updating handling post request timeout: ${stringify(error)}`)
	}
})
