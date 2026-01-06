import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EscrowReleasedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { IntentGatewayV2Service } from "@/services/intentGatewayV2.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

export const handleEscrowReleasedEventV2 = wrap(async (event: EscrowReleasedLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] Escrow Released Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash } = event
	const { commitment } = args!

	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V2] Escrow Released: ${stringify({
			commitment,
		})}`,
	)

	await IntentGatewayV2Service.updateOrderStatus(commitment, OrderStatus.REDEEMED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
})
