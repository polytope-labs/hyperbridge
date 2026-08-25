/** Default upper bound for an intent-execution RPC request. */
export const DEFAULT_INTENT_RPC_TIMEOUT_MS = 15_000

/** A request did not settle before its execution deadline. */
export class RpcTimeoutError extends Error {
	constructor(
		readonly operation: string,
		readonly timeoutMs = DEFAULT_INTENT_RPC_TIMEOUT_MS,
	) {
		super(`${operation} timed out after ${timeoutMs}ms`)
		this.name = "RpcTimeoutError"
	}
}

/**
 * Runs an RPC operation with a finite deadline.  Operations that honour AbortSignal
 * (notably fetch) are cancelled. The timeout also protects injected/custom clients
 * which do not expose cancellation.
 */
export async function withRpcTimeout<T>(
	operation: string,
	request: (signal: AbortSignal) => Promise<T>,
	timeoutMs = DEFAULT_INTENT_RPC_TIMEOUT_MS,
): Promise<T> {
	const controller = new AbortController()
	let timer: ReturnType<typeof setTimeout> | undefined
	try {
		return await Promise.race([
			request(controller.signal),
			new Promise<T>((_, reject) => {
				timer = setTimeout(() => {
					controller.abort()
					reject(new RpcTimeoutError(operation, timeoutMs))
				}, timeoutMs)
			}),
		])
	} finally {
		if (timer) clearTimeout(timer)
	}
}

/** A send timed out after bytes may have reached the bundler. Never retry it as a new bid. */
export class SubmissionOutcomeUnknownError extends Error {
	constructor(
		readonly userOpHash: `0x${string}`,
		readonly cause: unknown,
		readonly pendingResult: SelectBidResult,
	) {
		super(`UserOperation submission outcome is unknown for ${userOpHash}`)
		this.name = "SubmissionOutcomeUnknownError"
	}
}
import type { SelectBidResult } from "@/types"
