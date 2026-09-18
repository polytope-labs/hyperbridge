import { encodeFunctionData, decodeFunctionData, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { isRevert } from "./escrowReads"
import type { FillOptions, HexString, Order } from "@/types"

export type DecodedFillOrder = { order: Order; options: FillOptions }

/** `fillOrder(Order, FillOptions)` selector, pinned by codec tests; avoids import-time hashing in VM2. */
export const FILL_ORDER_SELECTOR = "0x68ddf058" as const
/** The gateway release and SolverAccount code version this SDK speaks. */
export const SUPPORTED_INTENTS_VERSION = 3n
export const CONTRACT_VERSION_ABI = [
	{
		type: "function",
		name: "version",
		stateMutability: "view",
		inputs: [],
		outputs: [{ name: "", type: "uint64" }],
	},
] as const

async function readContractVersion(client: PublicClient, address: HexString): Promise<unknown> {
	try {
		return await client.readContract({ address, abi: CONTRACT_VERSION_ABI, functionName: "version" })
	} catch (error) {
		if (isRevert(error)) return undefined
		throw error
	}
}

/** Whether both the gateway and the account report the supported release. RPC failures propagate. */
export async function supportsRateFills(
	client: PublicClient,
	gateway: HexString,
	solverAccount: HexString,
): Promise<boolean> {
	const versions = await Promise.all([
		readContractVersion(client, gateway),
		readContractVersion(client, solverAccount),
	])
	return versions.every((version) => version === SUPPORTED_INTENTS_VERSION)
}

/** The gateway must report release {@link SUPPORTED_INTENTS_VERSION}; a missing getter or any other release throws. */
export async function assertGatewayRelease(client: PublicClient, gateway: HexString): Promise<void> {
	const version = await readContractVersion(client, gateway)
	if (version === SUPPORTED_INTENTS_VERSION) return
	throw new Error(
		version === undefined
			? `IntentGateway ${gateway} reports no version(); this SDK requires release ${SUPPORTED_INTENTS_VERSION}`
			: `IntentGateway ${gateway} reports release ${String(version)}; this SDK requires release ${SUPPORTED_INTENTS_VERSION}`,
	)
}

const CANONICAL_EVM_TOKEN = /^0x0{24}[0-9a-fA-F]{40}$/

/** A bytes32 token whose upper 12 bytes are zero, the only form the gateway accepts. */
export function isCanonicalEvmToken(token: string): boolean {
	return CANONICAL_EVM_TOKEN.test(token)
}

/** Every leg must be quoted; a zero on both sides skips the leg. */
function validateFillQuotes(order: Order, options: FillOptions): void {
	const count = order.output.assets.length
	if (
		!count ||
		!Array.isArray(options.inputs) ||
		options.inputs.length !== count ||
		options.outputs.length !== count ||
		order.inputs.length !== count
	) {
		throw new Error("Fill inputs and outputs must quote every order leg")
	}
	for (let i = 0; i < count; i++) {
		const input = options.inputs[i]
		const output = options.outputs[i]
		if (
			input.token.toLowerCase() !== order.inputs[i].token.toLowerCase() ||
			output.token.toLowerCase() !== order.output.assets[i].token.toLowerCase() ||
			!isCanonicalEvmToken(input.token) ||
			!isCanonicalEvmToken(output.token)
		) {
			throw new Error("Fill quote tokens must match their order legs")
		}
		if ((input.amount === 0n) !== (output.amount === 0n)) {
			throw new Error("Fill quote input and output must both be zero or positive")
		}
	}
}

/** ABI-encodes a `fillOrder` call. Every leg must carry a validated quote. */
export function encodeFillOrder(order: Order, options: FillOptions): HexString {
	validateFillQuotes(order, options)
	return encodeFunctionData({
		abi: IntentGatewayV2ABI,
		functionName: "fillOrder",
		args: [order as any, options as any],
	}) as HexString
}

/**
 * Decodes a `fillOrder` call.
 *
 * @returns The decoded order and options, or `null` if the calldata is not a `fillOrder` call in
 *   the current shape or its quotes do not cover the order's legs.
 */
export function decodeFillOrder(data: HexString): DecodedFillOrder | null {
	try {
		const decoded = decodeFunctionData({ abi: IntentGatewayV2ABI, data })
		if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) return null
		const order = decoded.args[0] as Order
		const options = decoded.args[1] as FillOptions
		validateFillQuotes(order, options)
		return { order, options }
	} catch {
		return null
	}
}
