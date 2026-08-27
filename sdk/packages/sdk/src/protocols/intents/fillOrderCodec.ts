import {
	encodeFunctionData,
	decodeFunctionData,
	ContractFunctionRevertedError,
	ContractFunctionZeroDataError,
	type PublicClient,
} from "viem"
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

const FILL_OPTIONS_VERSION_ABI = [
	{
		type: "function",
		name: "fillOptionsVersion",
		inputs: [],
		outputs: [{ name: "", type: "uint256", internalType: "uint256" }],
		stateMutability: "pure",
	},
] as const

/** Keyed by `chainId:gateway`. A deployment's shape only changes on upgrade, so this is safe to hold. */
const versionCache = new Map<string, FillOptionsVersion>()

function cacheKey(chainId: number | undefined, gateway: HexString): string {
	return `${chainId ?? "unknown"}:${gateway.toLowerCase()}`
}

/** Test seam: drop memoised probe results. */
export function resetFillOptionsVersionCache(): void {
	versionCache.clear()
}

/**
 * Probes which `FillOptions` shape a gateway accepts, memoised per deployment.
 *
 * Deployments predating `validUntil` do not implement `fillOptionsVersion`, so the call
 * either reverts or returns no data — both are a definitive "v1" from the contract itself.
 * A transport failure is *not*: it is rethrown uncached, because caching it would silently
 * downgrade every later fill on that chain to unbounded validity for the lifetime of the
 * process. viem wraps every `readContract` failure in `ContractFunctionExecutionError`
 * regardless of cause, so the two are told apart by walking the cause chain rather than by
 * the thrown type.
 */
export async function getFillOptionsVersion(client: PublicClient, gateway: HexString): Promise<FillOptionsVersion> {
	const key = cacheKey(client.chain?.id, gateway)
	const cached = versionCache.get(key)
	if (cached !== undefined) return cached

	try {
		const version = (await client.readContract({
			address: gateway,
			abi: FILL_OPTIONS_VERSION_ABI,
			functionName: "fillOptionsVersion",
		})) as bigint
		const resolved: FillOptionsVersion = version >= 2n ? 2 : 1
		versionCache.set(key, resolved)
		return resolved
	} catch (error: any) {
		const isContractAnswer =
			typeof error?.walk === "function" &&
			!!error.walk(
				(e: unknown) => e instanceof ContractFunctionRevertedError || e instanceof ContractFunctionZeroDataError,
			)
		if (!isContractAnswer) throw error

		versionCache.set(key, 1)
		return 1
	}
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
