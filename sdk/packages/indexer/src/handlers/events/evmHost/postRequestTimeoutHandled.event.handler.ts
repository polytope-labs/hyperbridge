import { Status } from "../../../../configs/src/types"
import { PostRequestTimeoutHandledLog } from "../../../../configs/src/types/abi-interfaces/EthereumHostAbi"
import { HyperBridgeService } from "../../../services/hyperbridge.service"
import { RequestService } from "../../../services/request.service"
import { getHostStateMachine } from "../../../utils/substrate.helpers"

/**
 * Handles the PostRequestTimeoutHandled event
 */
export async function handlePostRequestTimeoutHandledEvent(event: PostRequestTimeoutHandledLog): Promise<void> {
	if (!event.args) return

	const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
	const { commitment, dest } = args

	logger.info(
		`Handling PostRequestTimeoutHandled Event: ${JSON.stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain: string = getHostStateMachine(chainId)

	try {
		await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

		await RequestService.updateStatus({
			commitment,
			chain,
			blockNumber: blockNumber.toString(),
			blockHash: block.hash,
			blockTimestamp: block.timestamp,
			status: Status.TIMED_OUT,
			transactionHash,
		})
	} catch (error) {
		logger.error(`Error updating handling post request timeout: ${JSON.stringify(error)}`)
	}
}
