import { encodeFunctionData, decodeFunctionData, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { FillOptions, HexString, Order } from "@/types"

/**
 * `FillOptions` gained a `validUntil` field. Adding a field to a struct changes the
 * enclosing function's selector, so `fillOrder` has two incompatible shapes in the wild:
 *
 *   v1  fillOrder(Order, (uint256 relayerFee, uint256 nativeDispatchFee, TokenInfo[] outputs))
 *   v2  fillOrder(Order, (uint256 relayerFee, uint256 nativeDispatchFee, uint256 validUntil, TokenInfo[] outputs))
 *
 * The selectors differ (`0x5cfb1ea5` vs `0xa5470064`), so a v2 payload sent to a v1
 * deployment finds no matching function and reverts rather than mis-decoding — which is the
 * safe failure, but it does mean callers have to know which shape a gateway speaks.
 */
export type FillOptionsVersion = 1 | 2

/** The v1 `fillOrder`, kept only so we can still talk to deployments that predate `validUntil`. */
const FILL_ORDER_V1_ABI = [
	{
		type: "function",
		name: "fillOrder",
		stateMutability: "payable",
		outputs: [],
		inputs: [
			IntentGatewayV2ABI.find((e) => e.type === "function" && e.name === "fillOrder")!.inputs![0],
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

/**
 * ERC-1967 implementation slot: `keccak256("eip1967.proxy.implementation") - 1`.
 * The gateway is deployed behind this proxy, so `eth_getCode` on the gateway returns the
 * proxy stub — the implementation's code, where the selectors actually live, is one hop away.
 */
const ERC1967_IMPLEMENTATION_SLOT = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc" as HexString

/**
 * IntentGateway implementations deployed before `FillOptions.validUntil` existed.
 *
 * The list is of *legacy* implementations rather than current ones, so the default is v2 and
 * nothing has to be added here when a new implementation ships — only when an old one is
 * discovered. Once every deployment is upgraded this set is vestigial and still correct.
 *
 * The alternative, listing known-good implementations, would be the version constant this
 * replaced wearing a different hat: a value someone must remember to update on every upgrade,
 * where forgetting breaks every fill on the chain.
 */
export const LEGACY_FILL_OPTIONS_IMPLEMENTATIONS = new Set<string>([
	// Pre-validUntil IntentGatewayV2 implementation.
	"0x976b268b06f545c4a2bf44866aa2465bd8b3c67d",
])

/**
 * Gateways already resolved to v2, keyed by proxy address.
 *
 * Only v2 answers are cached, and that asymmetry is deliberate. A deployment can move from
 * legacy to current but never back, so a v2 result is true forever, while caching a v1 result
 * would pin the old encoding across the very upgrade that changes it — the proxy address does
 * not move, so nothing would ever invalidate it. A still-legacy gateway therefore costs one
 * storage read per fill, and an upgraded one costs none.
 */
const knownV2Gateways = new Set<string>()

/** Test seam: drop memoised detection results. */
export function resetFillOptionsVersionCache(): void {
	knownV2Gateways.clear()
}

/** The address the ERC-1967 proxy delegates to, or the gateway itself if it is not a proxy. */
async function resolveImplementation(client: PublicClient, gateway: HexString): Promise<HexString> {
	const slot = await client.getStorageAt({ address: gateway, slot: ERC1967_IMPLEMENTATION_SLOT })
	if (!slot || slot.length < 66) return gateway
	const addr = `0x${slot.slice(-40)}` as HexString
	return /^0x0{40}$/.test(addr) ? gateway : addr
}

/**
 * Works out which `FillOptions` shape a gateway accepts from the implementation it delegates to.
 *
 * EIP-1967 standardises three slots, all holding addresses — there is no version field to read,
 * and the contract deliberately does not carry one either: a hand-maintained version constant is
 * a second source of truth that has to be bumped on the right upgrade. The implementation address
 * is the value the proxy already updates, so it is what identifies the deployed code.
 */
export async function getFillOptionsVersion(client: PublicClient, gateway: HexString): Promise<FillOptionsVersion> {
	const key = gateway.toLowerCase()
	if (knownV2Gateways.has(key)) return 2

	const implementation = await resolveImplementation(client, gateway)
	if (LEGACY_FILL_OPTIONS_IMPLEMENTATIONS.has(implementation.toLowerCase())) return 1

	knownV2Gateways.add(key)
	return 2
}

/**
 * ABI-encodes a `fillOrder` call in the shape the target gateway understands.
 *
 * On a v1 gateway `validUntil` is dropped — there is nowhere to put it and no check on the
 * other side. That is a real loss of protection, so callers that rely on the bound should
 * surface it rather than assume it took effect.
 */
export function encodeFillOrder(order: Order, options: FillOptions, version: FillOptionsVersion): HexString {
	if (version === 2) {
		return encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [order as any, options as any],
		}) as HexString
	}

	const { relayerFee, nativeDispatchFee, outputs } = options
	return encodeFunctionData({
		abi: FILL_ORDER_V1_ABI,
		functionName: "fillOrder",
		args: [order as any, { relayerFee, nativeDispatchFee, outputs } as any],
	}) as HexString
}

/**
 * Decodes a `fillOrder` call of either shape.
 *
 * v2 is tried first and v1 is the fallback; the selectors differ, so there is no shape a
 * decode could silently get wrong. `validUntil` reads as `0n` for a v1 payload, which is the
 * same value that means "no bound" in v2 — accurate, since a v1 fill genuinely has none.
 *
 * @returns The decoded order and options, or `null` if the calldata is not a `fillOrder`.
 */
export function decodeFillOrder(data: HexString): { order: Order; options: FillOptions } | null {
	try {
		const decoded = decodeFunctionData({ abi: IntentGatewayV2ABI, data })
		if (decoded.functionName === "fillOrder" && decoded.args && decoded.args.length >= 2) {
			return { order: decoded.args[0] as Order, options: decoded.args[1] as FillOptions }
		}
	} catch {
		// Falls through to the v1 attempt below.
	}

	try {
		const decoded = decodeFunctionData({ abi: FILL_ORDER_V1_ABI, data })
		if (decoded.functionName === "fillOrder" && decoded.args && decoded.args.length >= 2) {
			const legacy = decoded.args[1] as Omit<FillOptions, "validUntil">
			return { order: decoded.args[0] as Order, options: { ...legacy, validUntil: 0n } }
		}
	} catch {
		// Not a fillOrder call in either shape.
	}

	return null
}
