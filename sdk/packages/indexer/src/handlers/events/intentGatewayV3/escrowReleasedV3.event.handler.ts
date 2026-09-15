import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EscrowReleasedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handleEscrowReleasedEventV3 = wrap(async (event: EscrowReleasedLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Escrow Released Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, solver, tokens } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Escrow Released: ${stringify({
			commitment,
			solver,
		})}, tokens: ${stringify(tokens)}`,
	)

	// recordEscrowRelease decides whether this release completes the order (REDEEMED)
	// or is a non-finalizing partial redeem that leaves the escrow open.
	await IntentGatewayV3Service.recordEscrowRelease(
		commitment,
		solver,
		tokens.map((token) => ({
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

	// The release just paid the solver the order's inputs back on this chain, so its inventory here
	// rose and every pool it backs in those tokens is understating depth. Best-effort: it reads
	// external RPCs, and stale depth is recoverable — the next phantom bid window republishes it.
	try {
		await IntentGatewayV3Service.publishInventoryAfterEscrowRelease({
			provider: solver,
			tokens: tokens.map((token) => ({
				token: token.token as Hex,
				amount: BigInt(token.amount.toString()),
			})),
			timestamp,
			blockNumber,
		})
	} catch (e: any) {
		logger.error(`Failed to publish pool inventory for released escrow ${commitment}: ${e.message}`)
	}
})
