import { type HexString, type IGetRequest, type IPostRequest, RequestStatus, TimeoutStatus } from "@/types"
import type { RequestStatusKey, TimeoutStatusKey, RetryConfig } from "@/types"
import { encodePacked, keccak256, toHex } from "viem"
import { createConsola, LogLevels } from "consola"
import { _queryRequestInternal } from "./query-client"

export * from "./utils/mmr"
export * from "./utils/substrate"

export const DEFAULT_POLL_INTERVAL = 5_000

/**
 * Sleeps for the specified number of milliseconds.
 * @param ms The number of milliseconds to sleep.
 */
export function sleep(ms?: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms || DEFAULT_POLL_INTERVAL))
}

/**
 * Checks if the given state machine ID represents an EVM chain.
 * @param stateMachineId The state machine ID to check.
 */
export function isEvmChain(stateMachineId: string): boolean {
	return stateMachineId.startsWith("EVM")
}

/**
 * Checks if the given state machine ID represents a Substrate chain.
 * @param stateMachineId The state machine ID to check.
 */
export function isSubstrateChain(stateMachineId: string): boolean {
	return (
		stateMachineId.startsWith("POLKADOT") ||
		stateMachineId.startsWith("KUSAMA") ||
		stateMachineId.startsWith("SUBSTRATE")
	)
}

/**
 * Checks if the given string is a valid UTF-8 string.
 * @param str The string to check.
 */
export function isValidUTF8(str: string): boolean {
	return Buffer.from(str).toString("utf8") === str
}

/**
 * Calculates the commitment hash for a post request.
 * @param post The post request to calculate the commitment hash for.
 * @returns The commitment hash.
 */
export function postRequestCommitment(post: IPostRequest): HexString {
	return keccak256(
		encodePacked(
			["bytes", "bytes", "uint64", "uint64", "bytes", "bytes", "bytes"],
			[toHex(post.source), toHex(post.dest), post.nonce, post.timeoutTimestamp, post.from, post.to, post.body],
		),
	)
}

export const DEFAULT_LOGGER = createConsola({
	level: LogLevels.silent,
})

export async function retryPromise<T>(operation: () => Promise<T>, retryConfig: RetryConfig): Promise<T> {
	const { logger = DEFAULT_LOGGER, logMessage = "Retry operation failed" } = retryConfig

	let lastError: unknown
	for (let i = 0; i < retryConfig.maxRetries; i++) {
		try {
			return await operation()
		} catch (error) {
			logger.trace(`Retrying(${i}) > ${logMessage}`)
			lastError = error
			await new Promise((resolve) => setTimeout(resolve, retryConfig.backoffMs * 2 ** i))
		}
	}

	throw lastError
}

/**
 * Calculates the commitment hash for a get request.
 * @param get The get request to calculate the commitment hash for.
 * @returns The commitment hash.
 */
export function getRequestCommitment(get: IGetRequest): HexString {
	const keysEncoding = "0x".concat(get.keys.map((key) => key.slice(2)).join(""))
	return keccak256(
		encodePacked(
			["bytes", "bytes", "uint64", "uint64", "uint64", "bytes", "bytes", "bytes"],
			[
				toHex(get.source),
				toHex(get.dest),
				get.nonce,
				get.height,
				get.timeoutTimestamp,
				get.from,
				keysEncoding as HexString,
				get.context,
			],
		),
	)
}

/**
 ** Calculates the weight of a request status.
 * Used to determine the progression of a request through its lifecycle.
 * Higher weights represent more advanced states in the processing pipeline.
 * @returns A record mapping each RequestStatus to its corresponding weight value.
 */
export const REQUEST_STATUS_WEIGHTS: Record<RequestStatusKey, number> = {
	[RequestStatus.SOURCE]: 0,
	[RequestStatus.SOURCE_FINALIZED]: 1,
	[RequestStatus.HYPERBRIDGE_DELIVERED]: 2,
	[RequestStatus.HYPERBRIDGE_FINALIZED]: 3,
	[RequestStatus.DESTINATION]: 4,
	[RequestStatus.HYPERBRIDGE_TIMED_OUT]: 5,
	[RequestStatus.TIMED_OUT]: 6,
}

/**
 * Calculates the weight of a timeout status.
 * Used to determine the progression of a timeout through its lifecycle.
 * Higher weights represent more advanced states in the timeout processing.
 * @returns A record mapping each TimeoutStatus to its corresponding weight value.
 */
export const TIMEOUT_STATUS_WEIGHTS: Record<TimeoutStatusKey, number> = {
	[TimeoutStatus.PENDING_TIMEOUT]: 1,
	[TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT]: 2,
	[TimeoutStatus.HYPERBRIDGE_TIMED_OUT]: 3,
	[TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT]: 4,
	[TimeoutStatus.TIMED_OUT]: 5,
}

/**
 * Combines both request and timeout status weights into a single mapping.
 * This provides a comprehensive view of all possible states a request can be in,
 * with higher weights representing more advanced states in either the normal
 * processing pipeline or the timeout handling process.
 *
 * The weights follow this progression:
 * 0-4: Normal request processing (SOURCE to DESTINATION)
 * 5-9: Timeout handling progression (PENDING_TIMEOUT to TIMED_OUT)
 *
 * @returns A record mapping each RequestStatus and TimeoutStatus to its corresponding weight value.
 */
export const COMBINED_STATUS_WEIGHTS: Record<RequestStatusKey | TimeoutStatusKey, number> = {
	[RequestStatus.SOURCE]: 0,
	[RequestStatus.SOURCE_FINALIZED]: 1,
	[RequestStatus.HYPERBRIDGE_DELIVERED]: 2,
	[RequestStatus.HYPERBRIDGE_FINALIZED]: 3,
	[RequestStatus.DESTINATION]: 4,
	[TimeoutStatus.PENDING_TIMEOUT]: 5,
	[TimeoutStatus.DESTINATION_FINALIZED_TIMEOUT]: 6,
	[TimeoutStatus.HYPERBRIDGE_TIMED_OUT]: 7,
	[TimeoutStatus.HYPERBRIDGE_FINALIZED_TIMEOUT]: 8,
	[TimeoutStatus.TIMED_OUT]: 9,
}
