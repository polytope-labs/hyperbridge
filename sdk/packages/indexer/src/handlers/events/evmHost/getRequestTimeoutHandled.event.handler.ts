import { HyperBridgeService } from "@/services/hyperbridge.service"
import { Status } from "@/configs/src/types"
import { GetRequestTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { GetRequestService } from "@/services/getRequest.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"

/**
 * Handles the GetRequestTimeoutHandled event from EVMHost
 */
export async function handleGetRequestTimeoutHandled(event: GetRequestTimeoutHandledLog): Promise<void> {
	if (!event.args) return

	const { args, block, transactionHash, blockNumber, blockHash } = event
	const { commitment } = args

	logger.info(
		`Handling GetRequestTimeoutHandled Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain = getHostStateMachine(chainId)
	const blockTimestamp = await getBlockTimestamp(blockHash, chain)

	await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

	await GetRequestService.updateStatus({
		commitment,
		chain,
		blockNumber: blockNumber.toString(),
		blockHash: block.hash,
		blockTimestamp,
		status: Status.TIMED_OUT,
		transactionHash,
	})
}
