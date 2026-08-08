import { Interface } from "@ethersproject/abi"
import { hexToBigInt } from "viem"
import type { Hex } from "viem"
import type { EthereumLog, EthereumTransaction } from "@subql/types-ethereum"

import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"
import { IOrderV3OutputAsset } from "@/configs/src/types/models/IOrderV3OutputAsset"
import { INTENT_GATEWAY_V3_ADDRESSES } from "@/constants"
import { FillEnrichment, IntentGatewayV3Service, OrderV3, TokenInfo } from "@/services/intentGatewayV3.service"
import { getContractCallInputs } from "./rpc.helpers"
import { bytes32ToBytes20, extractAddressFromTopic } from "./transfer.helpers"

// ERC-4337 EntryPoint event, identical across v0.6/v0.7/v0.8:
// UserOperationEvent(bytes32 indexed userOpHash, address indexed sender, address indexed paymaster,
//                    uint256 nonce, bool success, uint256 actualGasCost, uint256 actualGasUsed)
const USER_OPERATION_EVENT_TOPIC = "0x49628fd1471006c1482da88028e9ce4dbb080b815c9b0344d39e5a8e6ec1419f"

// ERC20 Transfer(address indexed from, address indexed to, uint256 value)
const ERC20_TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"

// IntentGatewayV3 fill events, used as boundaries between fills batched in one transaction:
// OrderFilled(bytes32,address,(bytes32,uint256)[],(bytes32,uint256)[])
const ORDER_FILLED_TOPIC = "0xdd5ba16ce7d9636800f5875b2a5572176a96f47947d3218b2a2ac4c5cc11f9bf"
// PartialFill(bytes32,address,(bytes32,uint256)[],(bytes32,uint256)[])
const PARTIAL_FILL_TOPIC = "0xa71fc5b4fbaf5f5f0846475fec0d0c1d6c93100f2326ddb75c117244f45bbe85"

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

const intentGatewayInterface = new Interface(IntentGatewayV3Abi)

type FillLog = Pick<EthereumLog, "address" | "logIndex" | "transactionHash"> & {
	transaction?: EthereumTransaction
}

/**
 * Derives the fill data that only exists in the fill transaction's receipt: the ERC-4337
 * userop hash that executed the fill (if any) and the output amounts actually received
 * by the beneficiary (which exceed the event amounts when the solver overfills).
 * @param fillLog The OrderFilled / PartialFill log being handled
 * @param params The fill's commitment, filler, decoded output assets, and chain
 */
export async function resolveFillEnrichment(
	fillLog: FillLog,
	params: {
		commitment: string
		filler: string
		outputs: TokenInfo[]
		chain: string
	},
): Promise<FillEnrichment> {
	const { commitment, filler, outputs, chain } = params

	if (!fillLog.transaction) {
		logger.warn(`No transaction on fill log ${fillLog.transactionHash}, skipping fill enrichment`)
		return {}
	}

	const receipt = await fillLog.transaction.receipt()
	const logs = receipt.logs ?? []

	const userOpHash = findUserOpHash(logs, filler, fillLog.logIndex)

	const beneficiary = await resolveBeneficiary(commitment, fillLog, chain)
	const amountsReceived = beneficiary
		? matchDeliveryTransfers(logs, fillLog, filler, beneficiary, outputs)
		: undefined

	return { userOpHash, amountsReceived }
}

/**
 * Finds the hash of the ERC-4337 user operation that executed a fill. The EntryPoint emits
 * UserOperationEvent right after each op finishes executing, so the op that performed this
 * fill emits the first event after the fill log whose sender is the filler account.
 * Returns undefined for plain EOA fills.
 */
export function findUserOpHash(logs: EthereumLog[], filler: string, fillLogIndex: number): string | undefined {
	const fillerAddress = filler.toLowerCase()

	const userOpEvent = logs
		.filter(
			(log) =>
				log.topics?.[0] === USER_OPERATION_EVENT_TOPIC &&
				log.topics.length >= 3 &&
				log.logIndex > fillLogIndex &&
				extractAddressFromTopic(log.topics[2]) === fillerAddress,
		)
		.sort((a, b) => a.logIndex - b.logIndex)[0]

	return userOpEvent?.topics[1]
}

/**
 * Resolves the beneficiary (20-byte, lowercase) of an order being filled. Prefers the indexed
 * order's own output assets; when the order hasn't been indexed yet (cross-chain fills index
 * independently of their source chain) recovers the order from the fill transaction's calldata,
 * verifying it against the commitment.
 */
async function resolveBeneficiary(commitment: string, fillLog: FillLog, chain: string): Promise<string | undefined> {
	// Every output of an order shares one beneficiary, so the first asset suffices.
	const orderOutput = await IOrderV3OutputAsset.get(`${commitment}-output-0`)
	if (orderOutput?.beneficiary) {
		return bytes32ToBytes20(orderOutput.beneficiary).toLowerCase()
	}

	try {
		const candidates: string[] = []
		if (fillLog.transaction?.input) {
			candidates.push(fillLog.transaction.input)
		}

		const gatewayAddress = INTENT_GATEWAY_V3_ADDRESSES[chain]
		if (gatewayAddress) {
			candidates.push(...(await getContractCallInputs(fillLog.transactionHash, gatewayAddress, chain)))
		}

		for (const calldata of candidates) {
			const order = tryDecodeFillOrder(calldata)
			if (!order) continue
			// A solver may batch fills for several orders in one transaction; the commitment
			// identifies which fillOrder call belongs to this event.
			if (IntentGatewayV3Service.computeOrderCommitment(order).toLowerCase() !== commitment.toLowerCase()) {
				continue
			}
			return bytes32ToBytes20(order.outputs.beneficiary).toLowerCase()
		}
	} catch (e: any) {
		logger.warn(`Could not recover order ${commitment} from fill calldata: ${e.message}`)
	}

	logger.warn(`Could not resolve beneficiary for order ${commitment}, skipping amountReceived`)
	return undefined
}

/**
 * Attempts to decode a fillOrder call from raw calldata.
 * Returns the order on success, or null if the calldata isn't a fillOrder call.
 */
export function tryDecodeFillOrder(calldata: string): OrderV3 | null {
	try {
		const { name, args } = intentGatewayInterface.parseTransaction({ data: calldata })
		if (name !== "fillOrder") return null

		const decoded = args[0]
		return {
			user: decoded.user as Hex,
			sourceChain: decoded.source,
			destChain: decoded.destination,
			deadline: BigInt(decoded.deadline.toString()),
			nonce: BigInt(decoded.nonce.toString()),
			fees: BigInt(decoded.fees.toString()),
			session: decoded.session as Hex,
			predispatch: {
				assets: decoded.predispatch.assets.map((token: any) => ({
					token: token.token as Hex,
					amount: BigInt(token.amount.toString()),
				})),
				call: decoded.predispatch.call as Hex,
			},
			inputs: decoded.inputs.map((token: any) => ({
				token: token.token as Hex,
				amount: BigInt(token.amount.toString()),
			})),
			outputs: {
				beneficiary: decoded.output.beneficiary as Hex,
				assets: decoded.output.assets.map((token: any) => ({
					token: token.token as Hex,
					amount: BigInt(token.amount.toString()),
				})),
				call: decoded.output.call as Hex,
			},
		}
	} catch {
		return null
	}
}

/**
 * Matches each output asset to the ERC20 Transfer log (filler → beneficiary) that delivered it,
 * returning the actually transferred amounts. The gateway transfers outputs in order, so repeated
 * tokens consume matching transfers in log order. Native token outputs emit no Transfer log and
 * resolve to undefined.
 */
export function matchDeliveryTransfers(
	logs: EthereumLog[],
	fillLog: FillLog,
	filler: string,
	beneficiary: string,
	outputs: TokenInfo[],
): (bigint | undefined)[] {
	const fillerAddress = filler.toLowerCase()
	const gatewayAddress = fillLog.address.toLowerCase()

	// A solver may batch several fills in one transaction. Each fill's delivery transfers sit
	// between the previous fill's terminal event (OrderFilled/PartialFill) and its own.
	const previousFillBoundary = logs
		.filter(
			(log) =>
				log.address.toLowerCase() === gatewayAddress &&
				(log.topics?.[0] === ORDER_FILLED_TOPIC || log.topics?.[0] === PARTIAL_FILL_TOPIC) &&
				log.logIndex < fillLog.logIndex,
		)
		.reduce((max, log) => Math.max(max, log.logIndex), -1)

	const transfers = logs
		.filter(
			(log) =>
				log.topics?.[0] === ERC20_TRANSFER_TOPIC &&
				log.topics.length === 3 &&
				typeof log.data === "string" &&
				log.data.length > 2 &&
				log.logIndex > previousFillBoundary &&
				log.logIndex < fillLog.logIndex &&
				extractAddressFromTopic(log.topics[1]) === fillerAddress &&
				extractAddressFromTopic(log.topics[2]) === beneficiary,
		)
		.sort((a, b) => a.logIndex - b.logIndex)

	const consumed = new Set<number>()
	return outputs.map((output) => {
		const token = bytes32ToBytes20(output.token).toLowerCase()
		if (token === ZERO_ADDRESS) return undefined

		const index = transfers.findIndex((transfer, i) => !consumed.has(i) && transfer.address.toLowerCase() === token)
		if (index === -1) return undefined

		consumed.add(index)
		return hexToBigInt(transfers[index].data as Hex)
	})
}
