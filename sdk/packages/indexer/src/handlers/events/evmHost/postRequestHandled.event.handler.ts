import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status, Transfer, RequestV2 } from "@/configs/src/types"
import { PostRequestHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { RequestService } from "@/services/request.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { VolumeService } from "@/services/volume.service"
import { getPriceDataFromEthereumLog, isERC20TransferEvent, extractAddressFromTopic } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { safeArray } from "@/utils/data.helper"
import HandlerV1Abi from "@/configs/abis/HandlerV1.abi.json"
import { PostRequestMessage } from "@/types/ismp"
import { Interface } from "@ethersproject/abi"

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

	// We want the indexer to crash and restart if updating the request status failed so it can be retried
	await RequestService.updateStatus({
		commitment,
		chain,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		blockTimestamp,
		status: Status.DESTINATION,
		transactionHash,
  })

	// Non-critical operations: stats, parsing, transfers, and volumes
	try {
		// Update hyperbridge stats
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain, blockTimestamp, transaction)

		// Parse transaction to extract addresses
		let toAddresses = [] as string[]
		if (transaction?.input) {
			const { name, args } = new Interface(HandlerV1Abi).parseTransaction({ data: transaction.input })

			if (name === "handlePostRequests" && args && args.length > 1) {
				const postRequests = args[1] as PostRequestMessage
				for (const postRequest of postRequests.requests) {
					const { to: postRequestTo } = postRequest.request
					toAddresses.push(postRequestTo)
				}
			}
		}

		// Process transfers and update volumes
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

				const matchingContract = toAddresses.find(
					(addr) => addr.toLowerCase() === from.toLowerCase() || addr.toLowerCase() === to.toLowerCase(),
				)

				if (matchingContract) {
					await VolumeService.updateVolume(`Contract.${matchingContract}`, amountValueInUSD, blockTimestamp)
				}
			}
		}
	} catch (error) {
		logger.error(`Error in non-critical operations for PostRequestHandled: ${stringify(error)}`)
	}
})
