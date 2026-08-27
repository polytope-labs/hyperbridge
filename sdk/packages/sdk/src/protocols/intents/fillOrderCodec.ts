import { encodeFunctionData, decodeFunctionData, toFunctionSelector, type PublicClient } from "viem"
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

/** Derived from the ABIs rather than written down, so a struct edit cannot leave them stale. */
const FILL_ORDER_V2_SELECTOR = toFunctionSelector(
	IntentGatewayV2ABI.find((e: any) => e.type === "function" && e.name === "fillOrder") as any,
)

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

const FILL_ORDER_V1_SELECTOR = toFunctionSelector(FILL_ORDER_V1_ABI[0] as any)

/**
 * ERC-1967 implementation slot: `keccak256("eip1967.proxy.implementation") - 1`.
 * The gateway is deployed behind this proxy, so `eth_getCode` on the gateway returns the
 * proxy stub — the implementation's code, where the selectors actually live, is one hop away.
 */
const ERC1967_IMPLEMENTATION_SLOT = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc" as HexString

/**
 * Memoised per *implementation* address, not per gateway.
 *
 * That is the load-bearing detail: the proxy address never changes, so keying on it would
 * pin the first answer for the life of the process and keep encoding the old shape after an
 * upgrade. The implementation address is exactly what an upgrade changes, so a cache keyed
 * on it invalidates itself.
 */
const versionByImplementation = new Map<string, FillOptionsVersion>()

/** Test seam: drop memoised detection results. */
export function resetFillOptionsVersionCache(): void {
	versionByImplementation.clear()
}

/** The address the ERC-1967 proxy delegates to, or the gateway itself if it is not a proxy. */
async function resolveImplementation(client: PublicClient, gateway: HexString): Promise<HexString> {
	const slot = await client.getStorageAt({ address: gateway, slot: ERC1967_IMPLEMENTATION_SLOT })
	if (!slot || slot.length < 66) return gateway
	const addr = `0x${slot.slice(-40)}` as HexString
	return /^0x0{40}$/.test(addr) ? gateway : addr
}

/**
 * Works out which `FillOptions` shape a gateway accepts by looking at its deployed code.
 *
 * There is deliberately no version getter on the contract to ask. A hand-maintained version
 * constant is a second source of truth that has to be remembered on every upgrade, and it
 * answers the wrong question — what the deployment *says* it is, rather than what it can
 * actually decode. The selector is the capability: Solidity emits it into the dispatcher, so
 * its presence in the implementation's runtime code is the ground truth, and it updates
 * itself whenever the proxy is repointed.
 *
 * Finding neither selector is an error rather than a guess. Both shapes cannot be decoded by
 * the other — a v1 payload sent to a v2 gateway is as dead as the reverse — so silently
 * assuming one would break every fill on that chain with a confusing revert instead of a
 * clear message here.
 */
export async function getFillOptionsVersion(client: PublicClient, gateway: HexString): Promise<FillOptionsVersion> {
	const implementation = await resolveImplementation(client, gateway)
	const key = implementation.toLowerCase()
	const cached = versionByImplementation.get(key)
	if (cached !== undefined) return cached

	const code = (await client.getCode({ address: implementation }))?.toLowerCase() ?? "0x"
	const hasV2 = code.includes(FILL_ORDER_V2_SELECTOR.slice(2))
	const hasV1 = code.includes(FILL_ORDER_V1_SELECTOR.slice(2))

	if (!hasV2 && !hasV1) {
		throw new Error(
			`No fillOrder selector found in the IntentGateway implementation at ${implementation} ` +
				`(proxy ${gateway}). Neither ${FILL_ORDER_V2_SELECTOR} nor ${FILL_ORDER_V1_SELECTOR} is present, ` +
				`so the FillOptions shape cannot be determined.`,
		)
	}

	const resolved: FillOptionsVersion = hasV2 ? 2 : 1
	versionByImplementation.set(key, resolved)
	return resolved
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
