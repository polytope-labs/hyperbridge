import { getBlockTimestamp, getContractCallInput } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { OrderPlacedLog } from "@/configs/src/types/abi-interfaces/IntentGatewayV2Abi"
import { DEFAULT_REFERRER, IntentGatewayV2Service, OrderV2 } from "@/services/intentGatewayV2.service"
import { OrderStatus } from "@/configs/src/types"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { Hex } from "viem"
import { wrap } from "@/utils/event.utils"
import { Interface } from "@ethersproject/abi"
import IntentGatewayV2Abi from "@/configs/abis/IntentGatewayV2.abi.json"
import { INTENT_GATEWAY_V2_ADDRESSES } from "@/constants"
import { bytes32ToBytes20, bytes20ToBytes32 } from "@/utils/transfer.helpers"

const intentGatewayInterface = new Interface(IntentGatewayV2Abi)

export const handleOrderPlacedEventV2 = wrap(async (event: OrderPlacedLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] Order Placed Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, transaction } = event
	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)
	const txMeta = { transactionHash, blockNumber, timestamp }

	let order: OrderV2 = {
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
		const intentGatewayAddress = INTENT_GATEWAY_V2_ADDRESSES[chain]
		if (!intentGatewayAddress) {
			logger.error(`No IntentGatewayV2 address found for chain: ${chain}`)
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
	const commitment = IntentGatewayV2Service.computeOrderCommitment(order)
	order.id = commitment

	logger.info(`[Intent Gateway V2] Order Commitment: ${commitment}`)

	await IntentGatewayV2Service.getOrCreateOrder(
		{ ...order, user: bytes32ToBytes20(order.user) as Hex },
		graffiti,
		txMeta,
	)

	await IntentGatewayV2Service.updateOrderStatus(commitment, OrderStatus.PLACED, txMeta)
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
	order: OrderV2,
	decodedOrder: any,
	graffitiArg: string,
	userAddress: string,
): { order: OrderV2; graffiti: Hex } {
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
