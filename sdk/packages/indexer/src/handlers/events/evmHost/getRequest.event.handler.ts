import { GetRequestEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { GetRequestService } from "@/services/getRequest.service"
import { GetRequestStatusMetadata, Status } from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"

/**
 * Handles the GetRequest event from Evm Hosts
 */
export const handleGetRequestEvent = wrap(async (event: GetRequestEventLog): Promise<void> => {
	logger.info(
		`Handling GetRequest Event: ${stringify({
			event,
		})}`,
	)
	if (!event.args) return

	const { blockNumber, transactionHash, args, block, blockHash } = event
	let { source, dest, from, nonce, height, context, timeoutTimestamp, fee } = args
	let keys = args[3]

	const chain: string = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	// Update HyperBridge stats
	await HyperBridgeService.incrementNumberOfSentMessages(chain)

	logger.info(
		`Processing GetRequest Event: ${stringify({
			source,
			dest,
			from,
			keys,
			nonce,
			height,
			context,
			timeoutTimestamp,
			fee,
		})}`,
	)

	let get_request_commitment = GetRequestService.computeRequestCommitment(
		source,
		dest,
		BigInt(nonce.toString()),
		BigInt(height.toString()),
		BigInt(timeoutTimestamp.toString()),
		from,
		keys,
		context,
	)

	logger.info(
		`Get Request Commitment: ${stringify({
			commitment: get_request_commitment,
		})}`,
	)

	const blockTimestamp = block.timestamp

	await GetRequestService.createOrUpdate({
		id: get_request_commitment,
		source,
		dest,
		from,
		keys,
		nonce: BigInt(nonce.toString()),
		height: BigInt(height.toString()),
		context,
		timeoutTimestamp: BigInt(timeoutTimestamp.toString()),
		fee: BigInt(fee.toString()),
		transactionHash,
		blockNumber: blockNumber.toString(),
		blockHash,
		blockTimestamp,
		status: Status.SOURCE,
		chain,
	})

	const getRequestStatusMetadata = GetRequestStatusMetadata.create({
		id: `${get_request_commitment}.${Status.SOURCE}`,
		requestId: get_request_commitment,
		status: Status.SOURCE,
		chain,
		timestamp: blockTimestamp,
		blockNumber: blockNumber.toString(),
		blockHash,
		transactionHash,
		createdAt: timestampToDate(timestamp),
	})

	await getRequestStatusMetadata.save()
})
