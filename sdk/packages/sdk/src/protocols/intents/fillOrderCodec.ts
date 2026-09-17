import { encodeFunctionData, decodeFunctionData, toFunctionSelector, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { FillOptions, HexString, Order, TokenInfo } from "@/types"

/** Historical v1/v2 and current v3 calldata use distinct selectors. */
export type FillOptionsVersion = 1 | 2 | 3
export type NormalizedFillOptions = FillOptions & { inputs: TokenInfo[] }
export type DecodedFillOrder = { order: Order; options: NormalizedFillOptions; version: FillOptionsVersion }

/** Pinned four-field options ABI; independent of future generated-ABI changes. */
export const FILL_ORDER_V2_ABI = [
	{
		type: "function",
		name: "fillOrder",
		inputs: [
			{
				name: "order",
				type: "tuple",
				internalType: "struct Order",
				components: [
					{
						name: "user",
						type: "bytes32",
						internalType: "bytes32",
					},
					{
						name: "source",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "destination",
						type: "bytes",
						internalType: "bytes",
					},
					{
						name: "deadline",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "nonce",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "fees",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "session",
						type: "address",
						internalType: "address",
					},
					{
						name: "predispatch",
						type: "tuple",
						internalType: "struct DispatchInfo",
						components: [
							{
								name: "assets",
								type: "tuple[]",
								internalType: "struct TokenInfo[]",
								components: [
									{
										name: "token",
										type: "bytes32",
										internalType: "bytes32",
									},
									{
										name: "amount",
										type: "uint256",
										internalType: "uint256",
									},
								],
							},
							{
								name: "call",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
					{
						name: "inputs",
						type: "tuple[]",
						internalType: "struct TokenInfo[]",
						components: [
							{
								name: "token",
								type: "bytes32",
								internalType: "bytes32",
							},
							{
								name: "amount",
								type: "uint256",
								internalType: "uint256",
							},
						],
					},
					{
						name: "output",
						type: "tuple",
						internalType: "struct PaymentInfo",
						components: [
							{
								name: "beneficiary",
								type: "bytes32",
								internalType: "bytes32",
							},
							{
								name: "assets",
								type: "tuple[]",
								internalType: "struct TokenInfo[]",
								components: [
									{
										name: "token",
										type: "bytes32",
										internalType: "bytes32",
									},
									{
										name: "amount",
										type: "uint256",
										internalType: "uint256",
									},
								],
							},
							{
								name: "call",
								type: "bytes",
								internalType: "bytes",
							},
						],
					},
				],
			},
			{
				name: "options",
				type: "tuple",
				internalType: "struct FillOptions",
				components: [
					{
						name: "relayerFee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "nativeDispatchFee",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "validUntil",
						type: "uint256",
						internalType: "uint256",
					},
					{
						name: "outputs",
						type: "tuple[]",
						internalType: "struct TokenInfo[]",
						components: [
							{
								name: "token",
								type: "bytes32",
								internalType: "bytes32",
							},
							{
								name: "amount",
								type: "uint256",
								internalType: "uint256",
							},
						],
					},
				],
			},
		],
		outputs: [],
		stateMutability: "payable",
	},
] as const

export const FILL_ORDER_V1_ABI = [
	{
		type: "function",
		name: "fillOrder",
		stateMutability: "payable",
		outputs: [],
		inputs: [
			FILL_ORDER_V2_ABI[0].inputs[0],
			{
				name: "options",
				type: "tuple",
				internalType: "struct FillOptions",
				components: [
					{ name: "relayerFee", type: "uint256", internalType: "uint256" },
					{ name: "nativeDispatchFee", type: "uint256", internalType: "uint256" },
					{
						name: "outputs",
						type: "tuple[]",
						internalType: "struct TokenInfo[]",
						components: [
							{ name: "token", type: "bytes32", internalType: "bytes32" },
							{ name: "amount", type: "uint256", internalType: "uint256" },
						],
					},
				],
			},
		],
	},
] as const

/** Derived from the generated Solidity ABI, not a manually maintained version flag. */
export const FILL_ORDER_V3_SELECTOR = toFunctionSelector(
	IntentGatewayV2ABI.find((entry) => entry.type === "function" && entry.name === "fillOrder")!,
)
export const FILL_ORDER_SELECTOR_ABI = [
	{
		type: "function",
		name: "fillOrderSelector",
		stateMutability: "pure",
		inputs: [],
		outputs: [{ name: "", type: "bytes4" }],
	},
] as const

/** Retained for callers that inspect the older informational capability flag. */
export const SUPPORTS_RATE_FILLS_ABI = [
	{
		type: "function",
		name: "supportsRateFills",
		stateMutability: "pure",
		inputs: [],
		outputs: [{ name: "", type: "bool" }],
	},
] as const

function isMissingSelector(error: unknown): boolean {
	let current = error
	while (current && typeof current === "object") {
		const item = current as { name?: string; cause?: unknown; code?: number; message?: string }
		if (item.name === "ContractFunctionZeroDataError") return true
		// viem also labels JSON-RPC -32603 provider failures ContractFunctionRevertedError.
		// Require an actual EVM error code or the original RPC message, never that wrapper.
		if (item.code === 3) return true
		if (!item.cause || typeof item.cause !== "object") {
			return /^(?:execution reverted\b|VM Exception while processing transaction:\s*revert\b|function selector was not recognized\b)/i.test(
				item.message ?? "",
			)
		}
		current = item.cause
	}
	return false
}

async function readFillOrderSelector(client: PublicClient, address: HexString): Promise<unknown> {
	try {
		return await client.readContract({ address, abi: FILL_ORDER_SELECTOR_ABI, functionName: "fillOrderSelector" })
	} catch (error) {
		if (isMissingSelector(error)) return undefined
		throw error
	}
}

/** Exact, uncached gateway/account compatibility; transport errors remain actionable failures. */
export async function supportsRateFills(
	client: PublicClient,
	gateway: HexString,
	solverAccount: HexString,
): Promise<boolean> {
	const markers = await Promise.all([
		readFillOrderSelector(client, gateway),
		readFillOrderSelector(client, solverAccount),
	])
	return markers.every((marker) => marker === FILL_ORDER_V3_SELECTOR)
}

const ERC1967_IMPLEMENTATION_SLOT = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc" as HexString

export const LEGACY_FILL_OPTIONS_IMPLEMENTATIONS = new Set<string>([
	// The pre-validUntil IntentGatewayV2 implementation. One entry covers every chain: the
	// protocol contracts are CREATE2-deployed, so this is the implementation address on all
	// of them (confirmed with the maintainers).
	"0x976b268b06f545c4a2bf44866aa2465bd8b3c67d",
])

/**
 * Chains whose IntentGateway has not been redeployed with `FillOptions.validUntil` yet.
 *
 * A blunter instrument than {@link LEGACY_FILL_OPTIONS_IMPLEMENTATIONS} and used for the same
 * reason: those chains run a pre-`validUntil` implementation whose address is not tracked here,
 * so the address check would wrongly read them as current and every fill would revert on a
 * selector that does not exist.
 *
 * Delete a chain from this set when its gateway is redeployed. Once the set is empty the
 * implementation-address check covers everything on its own.
 */
export const CHAINS_WITHOUT_VALID_UNTIL = new Set<number>([
	97, // BNB testnet
	10200, // Gnosis Chiado
	80002, // Polygon Amoy
	84532, // Base Sepolia
	421614, // Arbitrum Sepolia
	688689, // Pharos testnet
	11155111, // Sepolia
	11155420, // Optimism Sepolia
	420420417, // Polkadot Hub Paseo
])

/** @deprecated Version resolution is now always fresh; retained for source compatibility. */
export function resetFillOptionsVersionCache(): void {}

async function resolveImplementation(client: PublicClient, gateway: HexString): Promise<HexString> {
	const slot = await client.getStorageAt({ address: gateway, slot: ERC1967_IMPLEMENTATION_SLOT })
	if (!slot || slot.length < 66) return gateway
	const address = `0x${slot.slice(-40)}` as HexString
	return /^0x0{40}$/.test(address) ? gateway : address
}

/** Probe before legacy overrides, so proxy upgrades and chain identity are always observed. */
export async function getFillOptionsVersion(client: PublicClient, gateway: HexString): Promise<FillOptionsVersion> {
	const selector = await readFillOrderSelector(client, gateway)
	if (selector === FILL_ORDER_V3_SELECTOR) return 3
	if (selector !== undefined && selector !== "0x" && selector !== "0x00000000") {
		throw new Error(`Unsupported fillOrder selector: ${String(selector)}`)
	}
	const chainId = client.chain?.id
	if (chainId !== undefined && CHAINS_WITHOUT_VALID_UNTIL.has(chainId)) return 1
	const implementation = await resolveImplementation(client, gateway)
	return LEGACY_FILL_OPTIONS_IMPLEMENTATIONS.has(implementation.toLowerCase()) ? 1 : 2
}

/** v1 drops validUntil; neither historical ABI can carry nonempty input takes. */
export function encodeFillOrder(order: Order, options: FillOptions, version: FillOptionsVersion): HexString {
	const normalized = { ...options, inputs: options.inputs ?? [] }
	if (version !== 3 && normalized.inputs.length)
		throw new Error("Historical fillOrder ABI cannot encode input takes (options.inputs)")
	const abi = version === 3 ? IntentGatewayV2ABI : version === 2 ? FILL_ORDER_V2_ABI : FILL_ORDER_V1_ABI
	return encodeFunctionData({ abi, functionName: "fillOrder", args: [order as any, normalized as any] }) as HexString
}

/** Normalizes historical options with validUntil=0 (v1) and inputs=[] (v1/v2). */
export function decodeFillOrder(data: HexString): DecodedFillOrder | null {
	for (const [version, abi] of [
		[3, IntentGatewayV2ABI],
		[2, FILL_ORDER_V2_ABI],
		[1, FILL_ORDER_V1_ABI],
	] as const) {
		try {
			const decoded = decodeFunctionData({ abi, data })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue
			const options = decoded.args[1] as FillOptions
			return {
				order: decoded.args[0] as Order,
				options: {
					...options,
					validUntil: version === 1 ? 0n : options.validUntil,
					inputs: options.inputs ?? [],
				},
				version,
			}
		} catch {
			/* Another selector, or malformed calldata. */
		}
	}
	return null
}
