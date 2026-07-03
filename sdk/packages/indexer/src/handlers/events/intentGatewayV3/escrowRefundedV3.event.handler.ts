import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EscrowRefundedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handleEscrowRefundedEventV3 = wrap(async (event: EscrowRefundedLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Escrow Refunded Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, tokens } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Escrow Refunded: ${stringify({
			commitment,
		})}, tokens: ${stringify(tokens)}`,
	)

	const refundTokens = tokens.map((token) => ({
		token: token.token as Hex,
		amount: BigInt(token.amount.toString()),
	}))

	await IntentGatewayV3Service.recordEscrowRefund(commitment, refundTokens, {
		transactionHash,
		blockNumber,
		timestamp,
		logIndex,
	})

	// A cancel of an already fully-filled order refunds nothing (the escrow went to
	// solvers) but still emits EscrowRefunded with all-zero amounts — the order must
	// not be marked REFUNDED then.
	if (refundTokens.some((token) => token.amount > 0n)) {
		await IntentGatewayV3Service.updateOrderStatus(commitment, OrderStatus.REFUNDED, {
			transactionHash,
			blockNumber,
			timestamp,
		})
	} else {
		logger.info(
			`[Intent Gateway V3] Escrow Refunded with zero amounts for ${stringify({ commitment })}, leaving order status unchanged`,
		)
	}
})
