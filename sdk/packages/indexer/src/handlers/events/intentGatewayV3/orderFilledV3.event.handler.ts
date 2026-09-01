import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderFilledLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handleOrderFilledEventV3 = wrap(async (event: OrderFilledLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Order Filled Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, filler, outputs, inputs } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Order Filled: ${stringify({
			commitment,
		})} by ${stringify({ filler })}, outputs: ${stringify(outputs)}, inputs: ${stringify(inputs)}`,
	)

	const mappedOutputs = outputs.map((token) => ({
		token: token.token as Hex,
		amount: BigInt(token.amount.toString()),
	}))
	const mappedInputs = inputs.map((token) => ({
		token: token.token as Hex,
		amount: BigInt(token.amount.toString()),
	}))

	await IntentGatewayV3Service.recordFill(commitment, filler, mappedOutputs, mappedInputs, {
		transactionHash,
		blockNumber,
		timestamp,
		logIndex,
	})

	await IntentGatewayV3Service.updateOrderStatus(
		commitment,
		OrderStatus.FILLED,
		{
			transactionHash,
			blockNumber,
			timestamp,
		},
		filler,
	)

	// Volume metrics are best-effort: a failure here must not fail the handler and stall indexing.
	try {
		await IntentGatewayV3Service.recordOrderVolume("FILLED", mappedOutputs, timestamp)
	} catch (e: any) {
		logger.error(`Failed to record FILLED volume for order ${commitment}: ${e.message}`)
	}

	// The fill just spent the filler's output-token inventory, which is what the pool's published
	// depth is a sum of, so re-read the LPs backing those pools. Best-effort for the same reason as
	// above, and doubly so here: it reads external RPCs, and stale depth is recoverable — the next
	// phantom bid window republishes it from scratch.
	try {
		await IntentGatewayV3Service.refreshPoolLiquidityAfterFill({
			commitment,
			inputs: mappedInputs,
			outputs: mappedOutputs,
			timestamp,
			blockNumber,
		})
	} catch (e: any) {
		logger.error(`Failed to refresh pool liquidity for order ${commitment}: ${e.message}`)
	}
})
