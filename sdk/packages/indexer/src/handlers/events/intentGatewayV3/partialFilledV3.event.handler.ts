import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { PartialFillLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { discoverSolverFromFill } from "@/services/solverInventory.service"
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

	// A partial fill makes the filler a solver just as a full one does.
	await discoverSolverFromFill({
		chain,
		solver: filler,
		blockNumber: BigInt(blockNumber),
		transactionHash,
		timestamp,
	})
})
