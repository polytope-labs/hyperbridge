import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderPlacedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayAbi"
import { IntentGatewayService, Order } from "@/services/intentGateway.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"

export async function handleOrderPlacedEvent(event: OrderPlacedLog): Promise<void> {
	logger.info(`Order Placed Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, block, blockHash } = event

	if (!args) return

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

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	logger.info(
		`Computing Order Commitment: ${stringify({
			order,
		})}`,
	)

	const commitment = IntentGatewayService.computeOrderCommitment(order)

	order.id = commitment

	logger.info(`Order Commitment: ${commitment}`)

	await IntentGatewayService.getOrCreateOrder(order, {
		transactionHash,
		blockNumber,
		timestamp,
	})

	await IntentGatewayService.updateOrderStatus(commitment, OrderStatus.PLACED, {
		transactionHash,
		blockNumber,
		timestamp,
	})
}
