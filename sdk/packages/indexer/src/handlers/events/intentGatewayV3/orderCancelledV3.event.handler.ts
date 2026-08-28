import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderCancelledLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

/**
 * Handles OrderCancelled, emitted when a cancellation is *initiated* on the chain it is
 * initiated from. EscrowRefunded remains the terminal event and is what moves the order to
 * REFUNDED — it follows in the same transaction for a same-chain cancel, and on the source
 * chain once the cancellation has travelled through Hyperbridge for a cross-chain one.
 *
 * The status write is guarded inside the service so this cannot regress an order that has
 * already been refunded; see `recordOrderCancellation`.
 */
export const handleOrderCancelledEventV3 = wrap(async (event: OrderCancelledLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Order Cancelled Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, canceller } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Order Cancelled: ${stringify({
			commitment,
			canceller,
		})}`,
	)

	await IntentGatewayV3Service.recordOrderCancellation(commitment, canceller, {
		transactionHash,
		blockNumber,
		timestamp,
		logIndex,
	})
})
