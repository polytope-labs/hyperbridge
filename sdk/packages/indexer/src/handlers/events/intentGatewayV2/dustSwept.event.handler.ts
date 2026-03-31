import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { DustSweptLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { ProtocolRevenueService } from "@/services/protocol-revenue.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

export const handleDustSweptEvent = wrap(async (event: DustSweptLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] DustSwept Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash } = event
	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	const { token, amount } = args

	logger.info(
		`DustSwept: ${stringify({
			token,
			amount: amount.toString(),
			beneficiary: args.beneficiary,
			transactionHash,
		})}`,
	)

	await ProtocolRevenueService.recordDustSwept(token, BigInt(amount.toString()), timestamp)
})
