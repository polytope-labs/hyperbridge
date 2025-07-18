import { Status } from "@/configs/src/types"
import { PostResponseTimeoutHandledLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { HyperBridgeService } from "@/services/hyperbridge.service"
import { ResponseService } from "@/services/response.service"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import stringify from "safe-stable-stringify"

/**
 * Handles the PostResponseTimeoutHandled event
 */
export const handlePostResponseTimeoutHandledEvent = wrap(
	async (event: PostResponseTimeoutHandledLog): Promise<void> => {
		if (!event.args) return
		const { args, block, transaction, transactionHash, transactionIndex, blockHash, blockNumber, data } = event
		const { commitment, dest } = args

		logger.info(
			`Handling PostResponseTimeoutHandled Event: ${stringify({
				blockNumber,
				transactionHash,
			})}`,
		)

		const chain: string = getHostStateMachine(chainId)
		const blockTimestamp = await getBlockTimestamp(blockHash, chain)

		try {
			await HyperBridgeService.incrementNumberOfTimedOutMessagesSent(chain)

			await ResponseService.updateStatus({
				commitment,
				chain,
				blockNumber: blockNumber.toString(),
				blockHash: block.hash,
				blockTimestamp,
				status: Status.TIMED_OUT,
				transactionHash,
			})
		} catch (error) {
			logger.error(`Error updating handling post response timeout: ${stringify(error)}`)
		}
	},
)
