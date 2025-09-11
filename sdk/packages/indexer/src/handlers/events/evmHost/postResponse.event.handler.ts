import { Status, Request, Transfer } from "@/configs/src/types"
import { PostResponseEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { ResponseService } from "@/services/response.service"
import { RequestService } from "@/services/request.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"
import { safeArray } from "@/utils/data.helper"
import { extractAddressFromTopic, getPriceDataFromEthereumLog, isERC20TransferEvent } from "@/utils/transfer.helpers"
import { TransferService } from "@/services/transfer.service"
import { VolumeService } from "@/services/volume.service"

/**
 * Handles the PostResponse event from Evm Hosts
 */
export const handlePostResponseEvent = wrap(async (event: PostResponseEventLog): Promise<void> => {
	logger.info(
		`Handling PostRequest Event: ${stringify({
			event,
		})}`,
	)
	if (!event.args) return

	const { transaction, blockNumber, transactionHash, args, block, blockHash } = event
	let {
		body,
		dest,
		fee,
		from: eventFrom,
		nonce,
		source,
		timeoutTimestamp,
		to: eventTo,
		response,
		responseTimeoutTimestamp,
	} = args

	const chain: string = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event)

	logger.info(
		`Computing Response Commitment Event: ${stringify({
			dest,
			fee,
			eventFrom,
			nonce,
			source,
			timeoutTimestamp,
			eventTo,
			body,
		})}`,
	)

	// Compute the response commitment
	let response_commitment = ResponseService.computeResponseCommitment(
		source,
		dest,
		BigInt(nonce.toString()),
		BigInt(timeoutTimestamp.toString()),
		eventFrom,
		eventTo,
		body,
		response,
		BigInt(responseTimeoutTimestamp.toString()),
	)

	logger.info(
		`Response Commitment: ${stringify({
			commitment: response_commitment,
		})}`,
	)

	// Compute the request commitment
	let request_commitment = RequestService.computeRequestCommitment(
		source,
		dest,
		BigInt(nonce.toString()),
		BigInt(timeoutTimestamp.toString()),
		eventFrom,
		eventTo,
		body,
	)

	let request = await Request.get(request_commitment)

	if (typeof request === "undefined") {
		logger.error(
			`Error handling PostResponseEvent because request with commitment: ${request_commitment} was not found`,
		)
		return
	}

	// Create the response entity
	await ResponseService.findOrCreate({
		chain,
		commitment: response_commitment,
		responseTimeoutTimestamp: BigInt(responseTimeoutTimestamp.toString()),
		response_message: response,
		status: Status.SOURCE,
		request,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		transactionHash,
		blockTimestamp,
	})

	for (const [index, log] of safeArray(transaction.logs).entries()) {
		if (!isERC20TransferEvent(log)) {
			continue
		}

		const value = BigInt(log.data)
		const transferId = `${log.transactionHash}-index-${index}`
		const transfer = await Transfer.get(transferId)

		if (!transfer) {
			const [_, fromTopic, toTopic] = log.topics
			const logFrom = extractAddressFromTopic(fromTopic)
			const logTo = extractAddressFromTopic(toTopic)

			// Store all transfers for volume tracking
			await TransferService.storeTransfer({
				transactionHash: transferId,
				chain,
				value,
				from: logFrom,
				to: logTo,
			})

			const { symbol, amountValueInUSD } = await getPriceDataFromEthereumLog(log.address, value, blockTimestamp)
			await VolumeService.updateVolume(`Transfer.${symbol}`, amountValueInUSD, blockTimestamp)

			if (logFrom.toLowerCase() === eventTo.toLowerCase() || logTo.toLowerCase() === eventTo.toLowerCase()) {
				await VolumeService.updateVolume(`Contract.${eventTo}`, amountValueInUSD, blockTimestamp)
			}
		}
	}
})
