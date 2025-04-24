import { HyperBridgeService } from "@/services/hyperbridge.service"
import { RequestService } from "@/services/request.service"
import { RequestStatusMetadata, Status } from "@/configs/src/types"
import { PostRequestEventLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { normalizeTimestamp } from "@/utils/date.helpers"

/**
 * Handles the PostRequest event from Evm Hosts
 */
export async function handlePostRequestEvent(event: PostRequestEventLog): Promise<void> {
	logger.info(
		`Handling PostRequest Event: ${JSON.stringify({
			event,
		})}`,
	)
	if (!event.args) return

	const { transaction, blockNumber, transactionHash, args, block } = event
	let { dest, fee, from, nonce, source, timeoutTimestamp, to, body } = args

	const chain: string = getHostStateMachine(chainId)

	await HyperBridgeService.handlePostRequestOrResponseEvent(chain, event)

	logger.info(
		`Computing Request Commitment Event: ${JSON.stringify({
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
		`Request Commitment: ${JSON.stringify({
			commitment: request_commitment,
		})}`,
	)

	const normalizedTimestamp = normalizeTimestamp(block.timestamp)
	const blockTimestamp = block.timestamp

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
		createdAt: new Date(Number(normalizedTimestamp)),
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
		createdAt: new Date(Number(normalizedTimestamp)),
	})

	await requestStatusMetadata.save()
}
