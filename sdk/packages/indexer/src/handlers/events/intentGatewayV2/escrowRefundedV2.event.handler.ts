import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EscrowRefundedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { IntentGatewayV2Service } from "@/services/intentGatewayV2.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handleEscrowRefundedEventV2 = wrap(async (event: EscrowRefundedLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] Escrow Refunded Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, tokens } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V2] Escrow Refunded: ${stringify({
			commitment,
		})}, tokens: ${stringify(tokens)}`,
	)

	await IntentGatewayV2Service.recordEscrowRefund(
		commitment,
		tokens.map((token) => ({
			token: token.token as Hex,
			amount: BigInt(token.amount.toString()),
		})),
		{
			transactionHash,
			blockNumber,
			timestamp,
			logIndex,
		},
	)

	await IntentGatewayV2Service.updateOrderStatus(commitment, OrderStatus.REFUNDED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
})
