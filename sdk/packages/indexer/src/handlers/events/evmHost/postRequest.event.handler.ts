import { HyperBridgeService } from "@/services/hyperbridge.service"
import { RequestService } from "@/services/request.service"
import { RequestStatusMetadata, Status } from "@/configs/src/types"
import { PostRequestEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"

/**
 * Handles the PostRequest event from Evm Hosts
 */
export async function handlePostRequestEvent(event: PostRequestEventLog): Promise<void> {
	logger.info(
		`Handling PostRequest Event: ${stringify({
			event,
		})}`,
	)
	if (!event.args) return

	const { transaction, blockNumber, transactionHash, args, block } = event
	let { dest, fee, from, nonce, source, timeoutTimestamp, to, body } = args

	const chain: string = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(block.hash, chain)

	await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event)

	logger.info(
		`Computing Request Commitment Event: ${stringify({
			dest,
			fee,
			from,
			nonce,
			source,
			timeoutTimestamp,
			to,
			body,
		})}`,
	)

	// Compute the request commitment
	let request_commitment = RequestService.computeRequestCommitment(
		source,
		dest,
		BigInt(nonce.toString()),
		BigInt(timeoutTimestamp.toString()),
		from,
		to,
		body,
	)

	logger.info(
		`Request Commitment: ${stringify({
			commitment: request_commitment,
		})}`,
	)

	const blockTimestamp = timestamp

	// Create the request entity
	await RequestService.createOrUpdate({
		chain,
		commitment: request_commitment,
		body,
		dest,
		fee: BigInt(fee.toString()),
		from,
		nonce: BigInt(nonce.toString()),
		source,
		status: Status.SOURCE,
		timeoutTimestamp: BigInt(timeoutTimestamp.toString()),
		to,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		transactionHash,
		blockTimestamp,
		createdAt: timestampToDate(timestamp),
	})

	// Always create a new status metadata entry
	let requestStatusMetadata = RequestStatusMetadata.create({
		id: `${request_commitment}.${Status.SOURCE}`,
		requestId: request_commitment,
		status: Status.SOURCE,
		chain,
		timestamp: blockTimestamp,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		transactionHash,
		createdAt: timestampToDate(timestamp),
	})

	await requestStatusMetadata.save()
}
