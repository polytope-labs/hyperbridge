import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { PartialFillLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { IntentGatewayV2Service } from "@/services/intentGatewayV2.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handlePartialFilledEventV2 = wrap(async (event: PartialFillLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] Partial Fill Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return

	const { commitment, filler, outputs, inputs } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V2] Partial Fill: ${stringify({
			commitment,
		})} by ${stringify({ filler })}`,
	)

	await IntentGatewayV2Service.recordPartialFill(
		commitment,
		filler as Hex,
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
})
