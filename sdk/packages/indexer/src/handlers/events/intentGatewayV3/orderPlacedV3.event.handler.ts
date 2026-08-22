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
import { getHostFeeToken } from "@/utils/host.helpers"

const intentGatewayInterface = new Interface(IntentGatewayV3Abi)

/**
 * Handles `OrderPlaced` logs from both event schemas. The project manifest
 * registers this handler twice: against the current schema (which carries the
 * full order and graffiti in the log) and, via a separate legacy datasource,
 * against the pre-#1092 schema whose call payloads must still be recovered
 * from the placeOrder calldata.
 */
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

	let graffiti: Hex
	const eventArgs = args as unknown as { predispatchCall?: string; outputCall?: string; graffiti?: string }
	if (eventArgs.predispatchCall !== undefined && eventArgs.outputCall !== undefined && eventArgs.graffiti !== undefined) {
		// Current schema: the complete order and graffiti are in the log itself.
		order.outputs.beneficiary = args.beneficiary as Hex
		order.outputs.call = eventArgs.outputCall as Hex
		order.predispatch.call = eventArgs.predispatchCall as Hex
		graffiti = resolveGraffiti(eventArgs.graffiti, args.user)
	} else {
		// Pre-#1092 schema: the call payloads and graffiti only exist in the
		// placeOrder calldata, so recover it from the transaction trace.
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

		graffiti = applyDecodedOrder(order, decoded.decodedOrder, decoded.graffitiArg, args.user).graffiti
	}
	const commitment = IntentGatewayV3Service.computeOrderCommitment(order)
	order.id = commitment

	logger.info(`[Intent Gateway V3] Order Commitment: ${commitment}`)

	// Fees are paid in the host's fee token, which differs per chain.
	const feeToken = await getHostFeeToken(chain)

	await IntentGatewayV3Service.getOrCreateOrder(
		{ ...order, user: bytes32ToBytes20(order.user) as Hex },
		graffiti,
		feeToken,
		txMeta,
	)

	await IntentGatewayV3Service.updateOrderStatus(commitment, OrderStatus.PLACED, txMeta)

	// Volume metrics are best-effort: a failure here must not fail the handler and stall indexing.
	try {
		await IntentGatewayV3Service.recordOrderVolume("PLACED", order.inputs, timestamp)
	} catch (e: any) {
		logger.error(`Failed to record PLACED volume for order ${commitment}: ${e.message}`)
	}
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

	return { order, graffiti: resolveGraffiti(graffitiArg, userAddress) }
}

/**
 * Maps a raw graffiti value to the stored referrer tag: a graffiti equal to
 * the placing user's address is treated as unattributed (DEFAULT_REFERRER).
 */
function resolveGraffiti(graffitiArg: string, userAddress: string): Hex {
	if (graffitiArg.toLowerCase() === userAddress.toLowerCase()) return DEFAULT_REFERRER as Hex

	const graffiti = bytes20ToBytes32(graffitiArg) as Hex
	logger.info(`Using referrer graffiti: ${graffiti}`)
	return graffiti
}
