import { encodeFunctionData, decodeFunctionData, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { isRevert } from "./escrowReads"
import { TESTNET_CHAINS } from "@/utils"
import type { FillOptions, HexString, Order } from "@/types"

export type DecodedFillOrder = { order: Order; options: FillOptions }

/** `fillOrder(Order, FillOptions)` selector, pinned by codec tests; avoids import-time hashing in VM2. */
export const FILL_ORDER_SELECTOR = "0x68ddf058" as const
/** The gateway release this SDK speaks. SolverAccount carries no version, so only the gateway is read. */
export const SUPPORTED_INTENTS_VERSION = 4n
/**
 * Testnet gateways already run the userOpHash solver selection that release 4 brings to mainnet, but
 * report 3, so a testnet chain accepts both.
 */
const TESTNET_INTENTS_VERSIONS = [3n, SUPPORTED_INTENTS_VERSION]

/** The releases a gateway on `stateMachineId` may report. */
function supportedReleases(stateMachineId: string): bigint[] {
	return TESTNET_CHAINS.has(stateMachineId) ? TESTNET_INTENTS_VERSIONS : [SUPPORTED_INTENTS_VERSION]
}
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

/** Whether the gateway on `stateMachineId` reports a supported release. RPC failures propagate. */
export async function supportsRateFills(
	client: PublicClient,
	gateway: HexString,
	stateMachineId: string,
): Promise<boolean> {
	const version = await readContractVersion(client, gateway)
	return supportedReleases(stateMachineId).some((release) => release === version)
}

/**
 * The gateway on `stateMachineId` must report a supported release: {@link SUPPORTED_INTENTS_VERSION}, or
 * 3 on a testnet. A missing getter or any other release throws.
 */
export async function assertGatewayRelease(
	client: PublicClient,
	gateway: HexString,
	stateMachineId: string,
): Promise<void> {
	const supported = supportedReleases(stateMachineId)
	const version = await readContractVersion(client, gateway)
	if (supported.some((release) => release === version)) return
	const required = supported.join(" or ")
	throw new Error(
		version === undefined
			? `IntentGateway ${gateway} reports no version(); this SDK requires release ${required}`
			: `IntentGateway ${gateway} reports release ${String(version)}; this SDK requires release ${required}`,
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
