import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { DustCollectedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { ProtocolRevenueService } from "@/services/protocol-revenue.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

export const handleDustCollectedEvent = wrap(async (event: DustCollectedLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] DustCollected Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash } = event
	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	const { token, amount } = args

	logger.info(
		`DustCollected: ${stringify({
			token,
			amount: amount.toString(),
			transactionHash,
		})}`,
	)

	await ProtocolRevenueService.recordDustCollected(token, BigInt(amount.toString()), timestamp)
})
