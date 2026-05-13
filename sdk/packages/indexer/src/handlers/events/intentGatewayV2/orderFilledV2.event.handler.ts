import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderFilledLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { IntentGatewayV2Service } from "@/services/intentGatewayV2.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handleOrderFilledEventV2 = wrap(async (event: OrderFilledLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] Order Filled Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, filler, outputs, inputs } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V2] Order Filled: ${stringify({
			commitment,
		})} by ${stringify({ filler })}, outputs: ${stringify(outputs)}, inputs: ${stringify(inputs)}`,
	)

	await IntentGatewayV2Service.recordFill(
		commitment,
		filler,
		outputs.map((token) => ({
			token: token.token as Hex,
			amount: BigInt(token.amount.toString()),
		})),
		inputs.map((token) => ({
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

	await IntentGatewayV2Service.updateOrderStatus(
		commitment,
		OrderStatus.FILLED,
		{
			transactionHash,
			blockNumber,
			timestamp,
		},
		filler,
	)
})
