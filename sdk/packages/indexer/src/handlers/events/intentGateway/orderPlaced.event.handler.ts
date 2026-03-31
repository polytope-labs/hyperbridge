import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderPlacedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayAbi"
import { DEFAULT_REFERRER, IntentGatewayService, Order } from "@/services/intentGateway.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"
import { Interface } from "@ethersproject/abi"
import IntentGatewayAbi from "@/configs/abis/IntentGateway.abi.json"

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
			const { name, args: decodedArgs } = new Interface(IntentGatewayAbi.abi).parseTransaction({
				data: transaction.input,
			})
			logger.info(`Decoded graffiti: ${stringify({ graffiti: decodedArgs[1] })}`)

			if (name === "placeOrder" && decodedArgs[1].toLowerCase() !== args.user.toLowerCase()) {
				// Either Default Referrer or Actual Referrer
				logger.info(`Using ${stringify(decodedArgs[1])} as graffiti`)
				graffiti = decodedArgs[1] as Hex
			}
		} catch (e: any) {
			logger.error(
				`Error decoding placeOrder args, using default referrer: ${stringify({
					error: e as unknown as Error,
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

	await IntentGatewayService.getOrCreateOrder({ ...order, user: bytes32ToBytes20(order.user) as Hex }, graffiti, {
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
