import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status } from "@/configs/src/types"
import { GetRequestTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { GetRequestService } from "@/services/getRequest.service"

/**
 * Handles the GetRequestTimeoutHandled event from EVMHost
 */
export async function handleGetRequestTimeoutHandled(event: GetRequestTimeoutHandledLog): Promise<void> {
	if (!event.args) return

	const { args, block, transactionHash, blockNumber } = event
	const { commitment } = args

	logger.info(
		`Handling GetRequestTimeoutHandled Event: ${JSON.stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)

	await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

	await GetRequestService.updateStatus({
		commitment,
		chain,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		blockTimestamp: block.timestamp,
		status: Status.TIMED_OUT,
		transactionHash,
	})
}
