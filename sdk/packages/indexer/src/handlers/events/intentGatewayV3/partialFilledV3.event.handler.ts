import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { PartialFillLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handlePartialFilledEventV3 = wrap(async (event: PartialFillLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Partial Fill Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return

	const { commitment, filler, outputs, inputs } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Partial Fill: ${stringify({
			commitment,
		})} by ${stringify({ filler })}`,
	)

	const mappedOutputs = outputs.map((token) => ({
		token: token.token as Hex,
		amount: BigInt(token.amount.toString()),
	}))
	const mappedInputs = inputs.map((token) => ({
		token: token.token as Hex,
		amount: BigInt(token.amount.toString()),
	}))

	await IntentGatewayV3Service.recordPartialFill(commitment, filler as Hex, mappedOutputs, mappedInputs, {
		transactionHash,
		blockNumber,
		timestamp,
		logIndex,
	})

	// A partial fill spends the filler's output-token inventory exactly as a full one does, so the
	// pools it drew on are re-read the same way. Best-effort: it reads external RPCs, and stale
	// depth is recoverable — the next phantom bid window republishes it from scratch.
	try {
		await IntentGatewayV3Service.refreshPoolLiquidityAfterFill({
			commitment,
			inputs: mappedInputs,
			outputs: mappedOutputs,
			timestamp,
			blockNumber,
		})
	} catch (e: any) {
		logger.error(`Failed to refresh pool liquidity for partially filled order ${commitment}: ${e.message}`)
	}
})
