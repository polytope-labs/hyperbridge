import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderFilledLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { discoverSolverFromFill } from "@/services/solverInventory.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"
import { resolveFillEnrichment } from "@/utils/fill.helpers"

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

	const enrichment = await resolveFillEnrichment(event, { commitment, filler, outputs: mappedOutputs, chain })
	await IntentGatewayV3Service.recordFill(
		commitment,
		filler,
		mappedOutputs,
		mappedInputs,
		{ transactionHash, blockNumber, timestamp, logIndex },
		enrichment,
	)

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

	// Filling an order is what makes an address a solver, so the filler starts being tracked. Store-only:
	// the next block's handler reads its balances. Deliberately unguarded, like the store writes above it —
	// only the store can fail it, and a lost discovery means a solver never tracked, so the block should
	// retry rather than swallow it. That is what separates it from the best-effort volume path below.
	await discoverSolverFromFill({
		chain,
		solver: filler,
		blockNumber: BigInt(blockNumber),
		transactionHash,
		timestamp,
	})

	// Volume metrics are best-effort: a failure here must not fail the handler and stall indexing.
	try {
		await IntentGatewayV3Service.recordOrderVolume("FILLED", mappedOutputs, timestamp)
	} catch (e: any) {
		logger.error(`Failed to record FILLED volume for order ${commitment}: ${e.message}`)
	}
})
