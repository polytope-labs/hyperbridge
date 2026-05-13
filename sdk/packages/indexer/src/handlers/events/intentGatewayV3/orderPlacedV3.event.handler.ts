import { getBlockTimestamp, getContractCallInput } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderPlacedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV3Abi"
import { DEFAULT_REFERRER, IntentGatewayV3Service, OrderV3 } from "@/services/intentGatewayV3.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"
import { Interface } from "@ethersproject/abi"
import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"
import { INTENT_GATEWAY_V3_ADDRESSES } from "@/constants"
import { bytes32ToBytes20, bytes20ToBytes32 } from "@/utils/transfer.helpers"

const intentGatewayInterface = new Interface(IntentGatewayV3Abi)

export const handleOrderPlacedEventV3 = wrap(async (event: OrderPlacedLog): Promise<void> => {
	logger.info(`[Intent Gateway V3] Order Placed Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, transaction } = event
	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)
	const txMeta = { transactionHash, blockNumber, timestamp }

	let order: OrderV3 = {
		user: args.user as Hex,
		sourceChain: args.source,
		destChain: args.destination,
		deadline: BigInt(args.deadline.toString()),
		nonce: BigInt(args.nonce.toString()),
		fees: BigInt(args.fees.toString()),
		session: args.session as Hex,
		predispatch: {
			assets: args.predispatch.map((token) => ({
				token: token.token as Hex,
				amount: BigInt(token.amount.toString()),
			})),
			call: "0x",
		},
		inputs: args.inputs.map((token) => ({
			token: token.token as Hex,
			amount: BigInt(token.amount.toString()),
		})),
		outputs: {
			beneficiary: "0x",
			assets: args.outputs.map((token) => ({
				token: token.token as Hex,
				amount: BigInt(token.amount.toString()),
			})),
			call: "0x",
		},
	}

	let decoded: { decodedOrder: any; graffitiArg: string } | null = null

	// Try to decode from direct transaction input first (direct call to IntentGateway)
	if (transaction?.input) {
		try {
			decoded = decodePlaceOrder(transaction.input)
		} catch (e: any) {
			logger.info(`Failed to decode direct transaction input, trying nested call: ${e.message}`)
		}
	}

	// If direct decoding failed, try to find IntentGateway call in nested calls
	if (!decoded) {
		const intentGatewayAddress = INTENT_GATEWAY_V3_ADDRESSES[chain]
		if (!intentGatewayAddress) {
			logger.error(`No IntentGatewayV3 address found for chain: ${chain}`)
		} else {
			try {
				const calldata = await getContractCallInput(transactionHash, intentGatewayAddress, chain)
				if (calldata) {
					decoded = decodePlaceOrder(calldata)
				} else {
					logger.warn(`IntentGateway call not found in nested calls for tx: ${transactionHash}`)
				}
			} catch (e: any) {
				logger.error(`Error decoding nested IntentGateway call: ${e.message}`)
			}
		}
	}

	if (!decoded) return

	const { graffiti } = applyDecodedOrder(order, decoded.decodedOrder, decoded.graffitiArg, args.user)
	const commitment = IntentGatewayV3Service.computeOrderCommitment(order)
	order.id = commitment

	logger.info(`[Intent Gateway V3] Order Commitment: ${commitment}`)

	await IntentGatewayV3Service.getOrCreateOrder(
		{ ...order, user: bytes32ToBytes20(order.user) as Hex },
		graffiti,
		txMeta,
	)

	await IntentGatewayV3Service.updateOrderStatus(commitment, OrderStatus.PLACED, txMeta)
})

/**
 * Attempts to decode a placeOrder call from raw calldata.
 * Returns the decoded order and graffiti args on success, or null if the call isn't placeOrder.
 */
function decodePlaceOrder(calldata: string): { decodedOrder: any; graffitiArg: string } | null {
	const { name, args: decodedArgs } = intentGatewayInterface.parseTransaction({ data: calldata })
	if (name !== "placeOrder") return null
	return { decodedOrder: decodedArgs[0], graffitiArg: decodedArgs[1] as string }
}

function applyDecodedOrder(
	order: OrderV3,
	decodedOrder: any,
	graffitiArg: string,
	userAddress: string,
): { order: OrderV3; graffiti: Hex } {
	order.outputs.beneficiary = decodedOrder.output.beneficiary as Hex
	order.outputs.call = decodedOrder.output.call as Hex
	order.predispatch.call = decodedOrder.predispatch.call as Hex

	let graffiti: Hex = DEFAULT_REFERRER as Hex
	if (graffitiArg.toLowerCase() !== userAddress.toLowerCase()) {
		graffiti = bytes20ToBytes32(graffitiArg) as Hex
		logger.info(`Using referrer graffiti: ${graffiti}`)
	}

	return { order, graffiti }
}
