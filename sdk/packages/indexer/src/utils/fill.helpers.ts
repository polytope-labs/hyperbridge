import { Interface } from "@ethersproject/abi"
import { hexToBigInt } from "viem"
import type { Hex } from "viem"
import type { EthereumLog, EthereumTransaction } from "@subql/types-ethereum"

import IntentGatewayV3Abi from "@/configs/abis/IntentGatewayV3.abi.json"
import type { OrderStructOutput, TokenInfoStructOutput } from "@/configs/src/types/contracts/IntentGatewayV3Abi"
import { IOrderV3OutputAsset } from "@/configs/src/types/models/IOrderV3OutputAsset"
import { FillEnrichment, IntentGatewayV3Service, OrderV3, TokenInfo } from "@/services/intentGatewayV3.service"
import { getContractCallInputs } from "./rpc.helpers"
import { bytes32ToBytes20, extractAddressFromTopic } from "./transfer.helpers"
import { findUserOpHash } from "./userOp.helpers"

// ERC20 Transfer(address indexed from, address indexed to, uint256 value)
const ERC20_TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"

// IntentGatewayV3 fill events, used as boundaries between fills batched in one transaction:
// OrderFilled(bytes32,address,(bytes32,uint256)[],(bytes32,uint256)[])
const ORDER_FILLED_TOPIC = "0xdd5ba16ce7d9636800f5875b2a5572176a96f47947d3218b2a2ac4c5cc11f9bf"
// PartialFill(bytes32,address,(bytes32,uint256)[],(bytes32,uint256)[])
const PARTIAL_FILL_TOPIC = "0xa71fc5b4fbaf5f5f0846475fec0d0c1d6c93100f2326ddb75c117244f45bbe85"

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

const intentGatewayInterface = new Interface(IntentGatewayV3Abi)
// FillOptions gained validUntil after the original V3 deployment; both selectors remain
// in historical receipts and use the same order tuple / commitment.
const legacyFillAbi = IntentGatewayV3Abi.filter((item) => item.type === "function" && item.name === "fillOrder").map(
	(item) => ({
		...item,
		inputs: item.inputs!.map((input) =>
			input.name === "options"
				? { ...input, components: input.components!.filter((field) => field.name !== "validUntil") }
				: input,
		),
	}),
)
const legacyFillInterface = new Interface(legacyFillAbi)

type FillLog = Pick<EthereumLog, "address" | "logIndex" | "transactionHash"> & {
	transaction?: EthereumTransaction
}

const decodeToken = (token: TokenInfoStructOutput): TokenInfo => ({
	token: token.token as Hex,
	amount: BigInt(token.amount.toString()),
})

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

	const decodeBeneficiary = (calldata: string): string | undefined => {
		const order = tryDecodeFillOrder(calldata)
		if (!order || IntentGatewayV3Service.computeOrderCommitment(order).toLowerCase() !== commitment.toLowerCase()) {
			return undefined
		}
		return bytes32ToBytes20(order.outputs.beneficiary).toLowerCase()
	}

	// Do not require debug tracing when direct calldata already identifies the order.
	if (fillLog.transaction?.input) {
		const beneficiary = decodeBeneficiary(fillLog.transaction.input)
		if (beneficiary) return beneficiary
	}

	try {
		const candidates = await getContractCallInputs(fillLog.transactionHash, fillLog.address, chain)
		for (const calldata of candidates) {
			const beneficiary = decodeBeneficiary(calldata)
			if (beneficiary) return beneficiary
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
		const parser =
			calldata.slice(0, 10).toLowerCase() === legacyFillInterface.getSighash("fillOrder")
				? legacyFillInterface
				: intentGatewayInterface
		const { name, args } = parser.parseTransaction({ data: calldata })
		if (name !== "fillOrder") return null

		const decoded = args[0] as OrderStructOutput
		return {
			user: decoded.user as Hex,
			sourceChain: decoded.source,
			destChain: decoded.destination,
			deadline: BigInt(decoded.deadline.toString()),
			nonce: BigInt(decoded.nonce.toString()),
			fees: BigInt(decoded.fees.toString()),
			session: decoded.session as Hex,
			predispatch: {
				assets: decoded.predispatch.assets.map(decodeToken),
				call: decoded.predispatch.call as Hex,
			},
			inputs: decoded.inputs.map(decodeToken),
			outputs: {
				beneficiary: decoded.output.beneficiary as Hex,
				assets: decoded.output.assets.map(decodeToken),
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
	const beneficiaryAddress = beneficiary.toLowerCase()
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
				/^0x[0-9a-fA-F]{64}$/.test(log.data) &&
				log.logIndex > previousFillBoundary &&
				log.logIndex < fillLog.logIndex &&
				extractAddressFromTopic(log.topics[1]) === fillerAddress &&
				extractAddressFromTopic(log.topics[2]) === beneficiaryAddress,
		)
		.sort((a, b) => a.logIndex - b.logIndex)

	const counts = new Map<string, number>()
	for (const output of outputs) {
		if (output.amount === 0n) continue
		const token = bytes32ToBytes20(output.token).toLowerCase()
		counts.set(token, (counts.get(token) ?? 0) + 1)
	}
	const transfersByToken = new Map<string, { logs: EthereumLog[]; next: number }>()
	for (const transfer of transfers) {
		const token = transfer.address.toLowerCase()
		const group = transfersByToken.get(token)
		if (group) group.logs.push(transfer)
		else transfersByToken.set(token, { logs: [transfer], next: 0 })
	}
	return outputs.map((output) => {
		const token = bytes32ToBytes20(output.token).toLowerCase()
		if (token === ZERO_ADDRESS) return undefined
		// PartialFill emits zero-valued slots for assets that this call did not
		// deliver. ERC-20 has no transfer for that slot, but the event makes zero
		// unambiguous and prevents it from poisoning repeated-token matching.
		if (output.amount === 0n) return 0n

		// Extra or missing transfers make attribution ambiguous (e.g. an unrelated
		// transfer before the call or a fee-on-transfer token emitting multiple logs).
		const group = transfersByToken.get(token)
		if (!group || group.logs.length !== counts.get(token)) {
			return undefined
		}
		return hexToBigInt(group.logs[group.next++].data as Hex)
	})
}
