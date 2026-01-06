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
import { INTENT_GATEWAY_ADDRESSES } from "@/constants"
import { bytes32ToBytes20, bytes20ToBytes32 } from "@/utils/transfer.helpers"

export const handleOrderPlacedEventV2 = wrap(async (event: OrderPlacedLog): Promise<void> => {
	logger.info(`[Intent Gateway V2] Order Placed Event: ${stringify(event)}`)

	const { blockNumber, transactionHash, args, blockHash, transaction } = event
	if (!args) return

	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)
	let graffiti = DEFAULT_REFERRER
	let decodedOrder: OrderV2 | null = null

	// Try to decode from direct transaction input first (direct call to IntentGateway)
	if (transaction?.input) {
		logger.info(`Decoding transaction data for referral points: ${stringify(transaction.input)}`)

		try {
			const { name, args: decodedArgs } = new Interface(IntentGatewayV2Abi).parseTransaction({
				data: transaction.input,
			})
			logger.info(`Decoded graffiti: ${stringify({ graffiti: decodedArgs[1] })}`)

			if (name === "placeOrder") {
				// decodedArgs[0] is the order object, decodedArgs[1] is the graffiti
				decodedOrder = decodedArgs[0]

				if (decodedArgs[1].toLowerCase() !== args.user.toLowerCase()) {
					// Either Default Referrer or Actual Referrer
					// Normalize to 32 bytes
					graffiti = bytes20ToBytes32(decodedArgs[1] as string) as Hex
					logger.info(`Using ${stringify(graffiti)} as graffiti`)
				}
			}
		} catch (e: any) {
			logger.info(
				`Failed to decode direct transaction input, trying nested call: ${stringify({
					error: e as unknown as Error,
				})}`,
			)
		}
	}

	// If direct decoding failed or didn't find placeOrder, try to find IntentGateway call in nested calls
	if (!decodedOrder) {
		const intentGatewayAddress = INTENT_GATEWAY_ADDRESSES[chain] // TODO: Update with V2 address
		if (!intentGatewayAddress) {
			logger.error(`No IntentGatewayV2 address found for chain: ${chain}`)
		} else {
			try {
				logger.info(`Attempting to find IntentGateway call in nested calls for transaction: ${transactionHash}`)
				const intentGatewayCalldata = await getContractCallInput(transactionHash, intentGatewayAddress, chain)

				if (intentGatewayCalldata) {
					logger.info(`Found IntentGateway call in nested calls, decoding calldata`)
					const { name, args: decodedArgs } = new Interface(IntentGatewayV2Abi).parseTransaction({
						data: intentGatewayCalldata,
					})

					if (name === "placeOrder") {
						// decodedArgs[0] is the order object, decodedArgs[1] is the graffiti
						decodedOrder = decodedArgs[0]

						if (decodedArgs[1].toLowerCase() !== args.user.toLowerCase()) {
							// Either Default Referrer or Actual Referrer
							// Normalize to 32 bytes
							graffiti = bytes20ToBytes32(decodedArgs[1] as string) as Hex
							logger.info(`Using ${stringify(graffiti)} as graffiti`)
						}
					}
				} else {
					logger.warn(`IntentGateway call not found in nested calls for transaction: ${transactionHash}`)
				}
			} catch (e: any) {
				logger.error(
					`Error finding or decoding nested IntentGateway call: ${stringify({
						error: e as unknown as Error,
					})}`,
				)
			}
		}
	}

	if (decodedOrder) {
		const order: OrderV2 = {
			id: "",
			user: decodedOrder.user as Hex,
			sourceChain: decodedOrder.sourceChain,
			destChain: decodedOrder.destChain,
			deadline: decodedOrder.deadline,
			nonce: decodedOrder.nonce,
			fees: decodedOrder.fees,
			session: decodedOrder.session as Hex,
			predispatch: decodedOrder.predispatch,
			inputs: decodedOrder.inputs,
			outputs: decodedOrder.outputs,
		}

		logger.info(
			`[Intent Gateway V2] Computing Order Commitment: ${stringify({
				order,
			})}`,
		)

		const commitment = IntentGatewayV2Service.computeOrderCommitment(order)

		order.id = commitment

		logger.info(`[Intent Gateway V2] Order Commitment: ${commitment}`)

		await IntentGatewayV2Service.getOrCreateOrder(
			{ ...order, user: bytes32ToBytes20(order.user) as Hex },
			graffiti,
			{
				transactionHash,
				blockNumber,
				timestamp,
			},
		)

		await IntentGatewayV2Service.updateOrderStatus(commitment, OrderStatus.PLACED, {
			transactionHash,
			blockNumber,
			timestamp,
		})
	}
})
