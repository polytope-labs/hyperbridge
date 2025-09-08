import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderPlacedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayAbi"
import { DEFAULT_REFERRER, IntentGatewayService, Order } from "@/services/intentGateway.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex, decodeFunctionData } from "viem"
import { wrap } from "@/utils/event.utils"
import IntentGatewayAbi from "@/configs/abis/IntentGateway.abi.json"
import { PointsService } from "@/services/points.service"

export const handleOrderPlacedEvent = wrap(async (event: OrderPlacedLog): Promise<void> => {
	logger.info(`Order Placed Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, block, blockHash, transaction } = event
	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)
	let graffiti = DEFAULT_REFERRER

	if (transaction?.input) {
		logger.info(`Decoding transaction data for referral points: ${stringify(transaction.input)}`)

		try {
			const inputData = transaction.input as string

			// Graffiti is the second parameter, located at position 74 (10 + 64)
			const graffitiStart = 74
			const graffitiEnd = graffitiStart + 64

			if (inputData.length >= graffitiEnd) {
				const graffitiHex = inputData.slice(graffitiStart, graffitiEnd)
				const decodedGraffiti = "0x" + graffitiHex

				logger.info(
					`Extracted graffiti from transaction: ${stringify({
						graffiti: decodedGraffiti,
						graffitiValue: BigInt("0x" + graffitiHex).toString(),
						isZero: decodedGraffiti === DEFAULT_REFERRER,
					})}`,
				)

				if (decodedGraffiti !== DEFAULT_REFERRER) {
					graffiti = decodedGraffiti as Hex
					logger.info(`Updated graffiti from transaction: ${stringify({ graffiti })}`)
				} else {
					logger.info("No referral code provided, using default referrer", {
						graffiti: DEFAULT_REFERRER,
					})
				}
			} else {
				logger.warn(
					`Transaction input too short to contain graffiti: ${stringify({
						inputLength: inputData.length,
						requiredLength: graffitiEnd,
					})}`,
				)
			}
		} catch (error) {
			logger.error(
				`Failed to extract graffiti from transaction: ${stringify({
					transactionHash,
					errorMessage: error?.toString() || "Unknown error",
					inputType: typeof transaction.input,
					inputLength: transaction.input?.length || 0,
				})}`,
			)
		}
	}

	const order: Order = {
		id: "",
		user: args.user as Hex,
		sourceChain: args.sourceChain,
		destChain: args.destChain,
		deadline: args.deadline.toBigInt(),
		nonce: args.nonce.toBigInt(),
		fees: args.fees.toBigInt(),
		inputs: args.inputs.map((input) => ({
			token: input.token as Hex,
			amount: input.amount.toBigInt(),
		})),
		outputs: args.outputs.map((output) => ({
			token: output.token as Hex,
			amount: output.amount.toBigInt(),
			beneficiary: output.beneficiary as Hex,
		})),
		callData: args.callData as Hex,
	}

	logger.info(
		`Computing Order Commitment: ${stringify({
			order,
		})}`,
	)

	const commitment = IntentGatewayService.computeOrderCommitment(order)

	order.id = commitment

	logger.info(`Order Commitment: ${commitment}`)

	await IntentGatewayService.getOrCreateOrder(order, graffiti, {
		transactionHash,
		blockNumber,
		timestamp,
	})

	await IntentGatewayService.updateOrderStatus(commitment, OrderStatus.PLACED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
})
