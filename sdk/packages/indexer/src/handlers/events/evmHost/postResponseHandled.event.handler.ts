import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status } from "@/configs/src/types"
import { PostResponseHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { ResponseService } from "@/services/response.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { Transfer, Response, Request } from "@/configs/src/types"
import { VolumeService } from "@/services/volume.service"
import { getPriceDataFromEthereumLog, isERC20TransferEvent, extractAddressFromTopic } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { safeArray } from "@/utils/data.helper"
import HandlerV1Abi from "@/configs/abis/HandlerV1.abi.json"
import { PostResponseMessage } from "@/types/ismp"
import { Interface } from "@ethersproject/abi"

/**
 * Handles the PostResponseHandled event from Hyperbridge
 */
export const handlePostResponseHandledEvent = wrap(async (event: PostResponseHandledLog): Promise<void> => {
	if (!event.args) return

	const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
	const { relayer: relayer_id, commitment } = args

	logger.info(
		`Handling PostResponseHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain: string = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	try {
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain, blockTimestamp)

		await ResponseService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockTimestamp,
			blockHash: block.hash,
			status: Status.DESTINATION,
			transactionHash,
		})

		let fromAddresses = [] as string[]

		if (transaction?.input) {
			const { name, args } = new Interface(HandlerV1Abi).parseTransaction({ data: transaction.input })
			if (name === "handlePostResponses" && args && args.length > 1) {
				const postResponses = args[1] as PostResponseMessage
				for (const postResponse of postResponses.responses) {
					const { post } = postResponse.response
					const { from: postRequestFrom } = post
					fromAddresses.push(postRequestFrom)
				}
			}
		}

		for (const [index, log] of safeArray(transaction?.logs).entries()) {
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
		logger.error(`Error updating handling post response: ${stringify(error)}`)
	}
})
