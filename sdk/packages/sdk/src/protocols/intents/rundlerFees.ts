import { BundlerMethod } from "./types"

/**
 * JSON-RPC codes for a request the server will never answer differently: invalid request,
 * method not found and invalid params. Bundlers that do not serve
 * `rundler_maxPriorityFeePerGas` reply with one of these.
 */
const UNSUPPORTED_REQUEST_CODES = new Set([-32600, -32601, -32602])

type GasFees = { maxFeePerGas: bigint; maxPriorityFeePerGas: bigint }

/** Bundler URLs that answered `rundler_maxPriorityFeePerGas` as unsupported. */
const bundlersWithoutRundlerFees = new Set<string>()

/** Bundler URLs that answered `rundler_getUserOperationGasPrice` as unsupported. */
const bundlersWithoutRundlerGasPrice = new Set<string>()

/**
 * Calls a parameterless rundler method. Returns `null` when the bundler does not answer it.
 *
 * A bundler that rejects the method as unsupported is added to `unsupported` and not asked
 * again. Any other failure, such as a network error or an internal error from rundler, is
 * not remembered, so the next call asks again.
 */
async function rundlerRequest(bundlerUrl: string, method: string, unsupported: Set<string>): Promise<unknown> {
	if (unsupported.has(bundlerUrl)) return null

	try {
		const response = await fetch(bundlerUrl, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params: [] }),
		})
		const { result, error } = (await response.json()) as { result?: unknown; error?: { code?: number } }
		if (error) {
			if (error.code !== undefined && UNSUPPORTED_REQUEST_CODES.has(error.code)) {
				unsupported.add(bundlerUrl)
			}
			return null
		}
		return result ?? null
	} catch {
		return null
	}
}

/**
 * The priority fee a rundler bundler requires of a UserOperation, from
 * `rundler_maxPriorityFeePerGas`. Returns `null` when the bundler does not give one, and the
 * caller keeps its own estimate.
 *
 * @param bundlerUrl - The bundler's JSON-RPC endpoint.
 * @returns The required priority fee in wei, or `null`.
 */
export async function fetchRundlerPriorityFee(bundlerUrl: string): Promise<bigint | null> {
	const result = await rundlerRequest(
		bundlerUrl,
		BundlerMethod.RUNDLER_MAX_PRIORITY_FEE_PER_GAS,
		bundlersWithoutRundlerFees,
	)
	try {
		return result == null ? null : BigInt(result as string)
	} catch {
		return null
	}
}

/**
 * The fees a rundler bundler suggests for a UserOperation, from
 * `rundler_getUserOperationGasPrice`. Rundler only bundles an op whose max fee covers its
 * bundle base fee, the pending base fee raised by its overhead (27% by default), plus its
 * priority fee. The suggested max fee adds a buffer on top of that. Returns `null` when the
 * bundler does not give them.
 *
 * @param bundlerUrl - The bundler's JSON-RPC endpoint.
 * @returns The suggested fees in wei, or `null`.
 */
export async function fetchRundlerSuggestedFees(bundlerUrl: string): Promise<GasFees | null> {
	const result = (await rundlerRequest(
		bundlerUrl,
		BundlerMethod.RUNDLER_GET_USER_OPERATION_GAS_PRICE,
		bundlersWithoutRundlerGasPrice,
	)) as { suggested?: { maxFeePerGas?: string; maxPriorityFeePerGas?: string } } | null
	const suggested = result?.suggested
	if (suggested?.maxFeePerGas == null || suggested.maxPriorityFeePerGas == null) return null
	try {
		return {
			maxFeePerGas: BigInt(suggested.maxFeePerGas),
			maxPriorityFeePerGas: BigInt(suggested.maxPriorityFeePerGas),
		}
	} catch {
		return null
	}
}

/**
 * UserOperation fees a rundler bundler will bundle. Where the bundler suggests fees, those
 * are used, each raised by its bump. Otherwise `fees` are raised to the priority fee it
 * requires (see {@link applyRundlerPriorityFee}), or returned unchanged when it gives none.
 */
export async function rundlerUserOperationFees(
	bundlerUrl: string,
	fees: GasFees,
	params: { baseFeePerGas: bigint; priorityFeeBumpPercent: bigint; maxFeeBumpPercent: bigint },
): Promise<GasFees> {
	const { baseFeePerGas, priorityFeeBumpPercent, maxFeeBumpPercent } = params
	const suggested = await fetchRundlerSuggestedFees(bundlerUrl)
	if (suggested) {
		return {
			maxFeePerGas: suggested.maxFeePerGas + (suggested.maxFeePerGas * maxFeeBumpPercent) / 100n,
			maxPriorityFeePerGas:
				suggested.maxPriorityFeePerGas + (suggested.maxPriorityFeePerGas * priorityFeeBumpPercent) / 100n,
		}
	}
	const rundlerPriorityFee = await fetchRundlerPriorityFee(bundlerUrl)
	if (rundlerPriorityFee === null) return fees
	return applyRundlerPriorityFee(fees, {
		rundlerPriorityFee,
		baseFeePerGas,
		priorityFeeBumpPercent,
		maxFeeBumpPercent,
	})
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
	fees: GasFees,
	params: {
		rundlerPriorityFee: bigint
		baseFeePerGas: bigint
		priorityFeeBumpPercent: bigint
		maxFeeBumpPercent: bigint
	},
): GasFees {
	const { rundlerPriorityFee, baseFeePerGas, priorityFeeBumpPercent, maxFeeBumpPercent } = params
	const priorityFloor = rundlerPriorityFee + (rundlerPriorityFee * priorityFeeBumpPercent) / 100n
	const maxFeeFloor = baseFeePerGas + (baseFeePerGas * maxFeeBumpPercent) / 100n + priorityFloor
	return {
		maxPriorityFeePerGas: fees.maxPriorityFeePerGas > priorityFloor ? fees.maxPriorityFeePerGas : priorityFloor,
		maxFeePerGas: fees.maxFeePerGas > maxFeeFloor ? fees.maxFeePerGas : maxFeeFloor,
	}
}
