import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderFilledLog } from "@/configs/src/types/abi-interfaces/IntentGatewayAbi"
import { IntentGatewayService } from "@/services/intentGateway.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export async function handleOrderFilledEvent(event: OrderFilledLog): Promise<void> {
	logger.info(`Order Filled Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, block, blockHash } = event
	const { commitment, filler } = args!

	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`Order Filled: ${stringify({
			commitment,
		})} by ${filler}`,
	)

	await IntentGatewayService.updateOrderStatus(
		commitment,
		OrderStatus.FILLED,
		{
			transactionHash,
			blockNumber,
			timestamp,
		},
		filler,
	)
}
