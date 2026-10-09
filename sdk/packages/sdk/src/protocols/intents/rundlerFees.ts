import { BundlerMethod } from "./types"

/**
 * JSON-RPC codes for a request the server will never answer differently: invalid request,
 * method not found and invalid params. Bundlers that do not serve
 * `rundler_maxPriorityFeePerGas` reply with one of these.
 */
const UNSUPPORTED_REQUEST_CODES = new Set([-32600, -32601, -32602])

/** Bundler URLs that answered `rundler_maxPriorityFeePerGas` as unsupported. */
const bundlersWithoutRundlerFees = new Set<string>()

/**
 * The priority fee a rundler bundler requires of a UserOperation, from
 * `rundler_maxPriorityFeePerGas`. Returns `null` when the bundler does not give one, and the
 * caller keeps its own estimate.
 *
 * A bundler that rejects the method as unsupported is not asked again. Any other failure,
 * such as a network error or an internal error from rundler, is not remembered, so the next
 * call asks again.
 *
 * @param bundlerUrl - The bundler's JSON-RPC endpoint.
 * @returns The required priority fee in wei, or `null`.
 */
export async function fetchRundlerPriorityFee(bundlerUrl: string): Promise<bigint | null> {
	if (bundlersWithoutRundlerFees.has(bundlerUrl)) return null

	try {
		const response = await fetch(bundlerUrl, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({
				jsonrpc: "2.0",
				id: 1,
				method: BundlerMethod.RUNDLER_MAX_PRIORITY_FEE_PER_GAS,
				params: [],
			}),
		})
		const { result, error } = (await response.json()) as { result?: string; error?: { code?: number } }
		if (error) {
			if (error.code !== undefined && UNSUPPORTED_REQUEST_CODES.has(error.code)) {
				bundlersWithoutRundlerFees.add(bundlerUrl)
			}
			return null
		}
		return result == null ? null : BigInt(result)
	} catch {
		return null
	}
}

/**
 * Raises UserOperation fees to what a rundler bundler accepts and bundles. The priority fee
 * becomes at least `rundlerPriorityFee` raised by `priorityFeeBumpPercent`. The max fee
 * becomes at least the base fee raised by `maxFeeBumpPercent` plus that priority fee floor.
 * Fees that already clear both are returned unchanged.
 *
 * The max fee is floored on the rundler figure rather than on the final priority fee: a
 * priority fee estimated from the chain's gas price already includes the base fee, and
 * adding the base fee to it again would make the op pay about twice the base fee.
 */
export function applyRundlerPriorityFee(
	fees: { maxFeePerGas: bigint; maxPriorityFeePerGas: bigint },
	params: {
		rundlerPriorityFee: bigint
		baseFeePerGas: bigint
		priorityFeeBumpPercent: bigint
		maxFeeBumpPercent: bigint
	},
): { maxFeePerGas: bigint; maxPriorityFeePerGas: bigint } {
	const { rundlerPriorityFee, baseFeePerGas, priorityFeeBumpPercent, maxFeeBumpPercent } = params
	const priorityFloor = rundlerPriorityFee + (rundlerPriorityFee * priorityFeeBumpPercent) / 100n
	const maxFeeFloor = baseFeePerGas + (baseFeePerGas * maxFeeBumpPercent) / 100n + priorityFloor
	return {
		maxPriorityFeePerGas: fees.maxPriorityFeePerGas > priorityFloor ? fees.maxPriorityFeePerGas : priorityFloor,
		maxFeePerGas: fees.maxFeePerGas > maxFeeFloor ? fees.maxFeePerGas : maxFeeFloor,
	}
}
