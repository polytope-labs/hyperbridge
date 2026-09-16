import { ProtocolFeeRefundedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import stringify from "safe-stable-stringify"

export const handleProtocolFeeRefundedEventV3 = wrap(async (event: ProtocolFeeRefundedLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] ProtocolFeeRefunded Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return

	const { commitment, token, amount } = args
	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	await IntentGatewayV3Service.recordProtocolFeeRefund(commitment, token, BigInt(amount.toString()), {
		transactionHash,
		blockNumber,
		timestamp,
		logIndex,
	})
})
