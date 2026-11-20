import type { BundlerRetryEvent } from "@/types"
import { sleep } from "@/utils"

/**
 * A failed call to an ERC-4337 bundler, thrown by {@link CryptoUtils.sendBundler}. The
 * message reads `Bundler error: <reason>`; the fields say what kind of failure it was.
 */
export class BundlerRpcError extends Error {
	/** The JSON-RPC error code, when the bundler answered with an error object. */
	readonly code?: number
	/** The JSON-RPC error's `data`, when it has one. */
	readonly data?: unknown
	/** The HTTP status of the response, when one arrived. */
	readonly httpStatus?: number
	/** True when no complete response arrived: the request or its body failed in transit. */
	readonly transport: boolean

	constructor(
		message: string,
		details: { code?: number; data?: unknown; httpStatus?: number; transport?: boolean; cause?: unknown } = {},
	) {
		super(message, details.cause === undefined ? undefined : { cause: details.cause })
		this.name = "BundlerRpcError"
		this.code = details.code
		this.data = details.data
		this.httpStatus = details.httpStatus
		this.transport = details.transport ?? false
	}
}

/** JSON-RPC internal error. Rundler answers with it when its upstream node fails ("rpc provider error"). */
const JSON_RPC_INTERNAL_ERROR = -32603

/**
 * JSON-RPC codes a retry cannot change: parse error, invalid request, method not found,
 * invalid params, and the ERC-4337 range (-32500 to -32599) for operations the EntryPoint,
 * paymaster or bundler rules reject.
 */
function isDeterministicRpcCode(code: number): boolean {
	return code === -32700 || (code >= -32602 && code <= -32600) || (code >= -32599 && code <= -32500)
}

/** An EntryPoint revert code such as `AA23`, which simulation reproduces on every attempt. */
const ENTRY_POINT_REVERT = /\bAA\d\d\b/

/**
 * Whether a bundler call failed for a reason that may not recur: a network failure, a
 * JSON-RPC internal error (-32603), or an HTTP 408, 429 or 5xx without a JSON-RPC error
 * that says otherwise. An EntryPoint revert (`AA..`), an ERC-4337 rejection code or
 * invalid params is the bundler's answer for that operation, so it is never transient.
 */
export function isTransientBundlerError(error: unknown): boolean {
	if (!(error instanceof BundlerRpcError)) return false
	if (error.transport) return true
	if (ENTRY_POINT_REVERT.test(error.message)) return false
	if (error.code !== undefined && isDeterministicRpcCode(error.code)) return false
	if (error.code === JSON_RPC_INTERNAL_ERROR) return true
	return (
		error.httpStatus === 408 ||
		error.httpStatus === 429 ||
		(error.httpStatus !== undefined && error.httpStatus >= 500)
	)
}

/**
 * Waits before the second and third attempts of a bundler gas estimate. Three attempts
 * add at most one second, because a fill bid is only worth sending while the order is open.
 */
export const BUNDLER_ESTIMATE_RETRY_DELAYS_MS: readonly number[] = [250, 750]

/**
 * How long one bundler request may take before it is aborted (see `CryptoUtils.sendBundler`).
 * Bounds each attempt so a stalled connection cannot hold up a fill bid: with the retries,
 * three attempts take at most three of these plus the waits above. Matches viem's default
 * HTTP transport timeout.
 */
export const BUNDLER_REQUEST_TIMEOUT_MS = 10_000

/** Reports a retry when the caller has no logger of its own for it. */
function warnBundlerRetry({ attempt, maxAttempts, delayMs, error }: BundlerRetryEvent): void {
	console.warn(
		`Bundler gas estimate attempt ${attempt}/${maxAttempts} failed, retrying in ${delayMs}ms: ${error.message}`,
	)
}

/**
 * Runs a bundler call, and runs it again after each delay in
 * {@link BUNDLER_ESTIMATE_RETRY_DELAYS_MS} while it fails transiently (see
 * {@link isTransientBundlerError}). Any other failure, and the failure of the last
 * attempt, is thrown as it is.
 *
 * Only for calls that change nothing, such as `eth_estimateUserOperationGas`. Each
 * attempt is bounded by {@link BUNDLER_REQUEST_TIMEOUT_MS}, so a stalled connection
 * cannot hold up the caller past the retries' own budget.
 *
 * @param operation - The bundler call.
 * @param onRetry - Called before each retry. Defaults to a `console.warn`.
 */
export async function retryTransientBundlerErrors<T>(
	operation: () => Promise<T>,
	onRetry: (event: BundlerRetryEvent) => void = warnBundlerRetry,
): Promise<T> {
	const maxAttempts = BUNDLER_ESTIMATE_RETRY_DELAYS_MS.length + 1
	for (let attempt = 1; ; attempt++) {
		try {
			return await operation()
		} catch (error) {
			if (attempt === maxAttempts || !isTransientBundlerError(error)) throw error
			const delayMs = BUNDLER_ESTIMATE_RETRY_DELAYS_MS[attempt - 1]
			onRetry({ attempt, maxAttempts, delayMs, error: error as BundlerRpcError })
			await sleep(delayMs)
		}
	}
}
