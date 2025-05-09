import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status } from "@/configs/src/types"
import { PostResponseHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { ResponseService } from "@/services/response.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"

/**
 * Handles the PostResponseHandled event from Hyperbridge
 */
export async function handlePostResponseHandledEvent(event: PostResponseHandledLog): Promise<void> {
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
		await HyperBridgeService.handlePostRequestOrResponseHandledEvent(relayer_id, chain)

		await ResponseService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockTimestamp,
			blockHash: block.hash,
			status: Status.DESTINATION,
			transactionHash,
		})
	} catch (error) {
		logger.error(`Error updating handling post response: ${stringify(error)}`)
	}
}
