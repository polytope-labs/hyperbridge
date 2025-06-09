import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EscrowReleasedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayAbi"
import { IntentGatewayService } from "@/services/intentGateway.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export async function handleEscrowReleasedEvent(event: EscrowReleasedLog): Promise<void> {
	logger.info(`Order Filled Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, block, blockHash } = event
	const { commitment } = args!

	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`Escrow Released: ${stringify({
			commitment,
		})}`,
	)

	await IntentGatewayService.updateOrderStatus(commitment, OrderStatus.REDEEMED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
}
