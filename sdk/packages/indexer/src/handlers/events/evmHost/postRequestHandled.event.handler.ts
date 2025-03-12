import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status } from "@/configs/src/types"
import { PostRequestHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { RequestService } from "@/services/request.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
export async function handlePostRequestHandledEvent(event: PostRequestHandledLog): Promise<void> {
	if (!event.args) return

	const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
	const { relayer: relayer_id, commitment } = args

	logger.info(
		`Handling PostRequestHandled Event: ${JSON.stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)

	try {
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain)

		await RequestService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockHash: block.hash,
			blockTimestamp: block.timestamp,
			status: Status.DESTINATION,
			transactionHash,
		})
	} catch (error) {
		;`Error handling PostRequestHandled event: ${JSON.stringify(error)}`
	}
}
