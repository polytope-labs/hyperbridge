import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { EscrowReleasedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"

export const handleEscrowReleasedEventV3 = wrap(async (event: EscrowReleasedLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Escrow Released Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, logIndex } = event
	if (!args) return
	const { commitment, tokens } = args

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`[Intent Gateway V3] Escrow Released: ${stringify({
			commitment,
		})}, tokens: ${stringify(tokens)}`,
	)

	await IntentGatewayV3Service.recordEscrowRelease(
		commitment,
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

	await IntentGatewayV3Service.updateOrderStatus(commitment, OrderStatus.REDEEMED, {
		transactionHash,
		blockNumber,
		timestamp,
	})

	// The release just paid the filler the order's inputs back on this chain, so its inventory here
	// rose and every pool it backs in those tokens is understating depth. The event names no filler;
	// the gateway recorded the beneficiary in the same call that emitted this. Best-effort: it reads
	// external RPCs, and stale depth is recoverable — the next phantom bid window republishes it.
	try {
		const beneficiary = await IntentGatewayV3Service.filledBeneficiary(commitment, blockNumber)
		if (beneficiary) {
			await IntentGatewayV3Service.refreshLiquidityAfterEscrowRelease({
				provider: beneficiary,
				tokens: tokens.map((token) => ({
					token: token.token as Hex,
					amount: BigInt(token.amount.toString()),
				})),
				timestamp,
				blockNumber,
			})
		}
	} catch (e: any) {
		logger.error(`Failed to refresh pool liquidity for released escrow ${commitment}: ${e.message}`)
	}
})
