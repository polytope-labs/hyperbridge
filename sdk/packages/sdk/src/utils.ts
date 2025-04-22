import {
	type HexString,
	type IGetRequest,
	type IPostRequest,
	RequestStatus,
	TimeoutStatus,
	type StateMachineHeight,
} from "@/types"
import type { RequestStatusKey, TimeoutStatusKey, RetryConfig } from "@/types"
import { encodePacked, keccak256, toHex } from "viem"
import { createConsola, LogLevels } from "consola"
import { _queryRequestInternal } from "./query-client"
import type { IChain } from "./chain"

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
 * Waits for the challenge period to elapse on a chain.
 * This function will sleep until the challenge period has elapsed.
 *
 * @param chain The chain object implementing IChain interface
 * @param stateMachineHeight The state machine height to wait for
 * @returns Promise that resolves when the challenge period has elapsed
 */
export async function waitForChallengePeriod(chain: IChain, stateMachineHeight: StateMachineHeight): Promise<void> {
	// Get the challenge period for this state machine
	const challengePeriod = await chain.challengePeriod(stateMachineHeight.id)

	if (challengePeriod === BigInt(0)) return

	// Get the state machine update time
	const updateTime = await chain.stateMachineUpdateTime(stateMachineHeight)
	// Check current timestamp
	let currentTimestamp = await chain.timestamp()
	// Calculate time passed since update
	let timeElapsed = currentTimestamp - updateTime

	if (timeElapsed > challengePeriod) return

	// First sleep for the whole challenge period
	await sleep(Number(challengePeriod) * 1000)

	// Keep sleeping until challenge period has fully elapsed
	while (timeElapsed <= challengePeriod) {
		// Sleep for remaining time
		const remainingTime = challengePeriod - timeElapsed
		await sleep(Number(remainingTime) * 1000)

		// Check timestamp again
		currentTimestamp = await chain.timestamp()
		timeElapsed = currentTimestamp - updateTime
	}
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
 * Converts a state machine ID string to a stateId object.
 * Handles formats like:
 * - "EVM-97" → { Evm: 97 }
 * - "SUBSTRATE-cere" → { Substrate: "0x63657265" } (hex encoded UTF-8 bytes)
 * - "POLKADOT-3367" → { Polkadot: 3367 }
 * - "KUSAMA-123" → { Kusama: 123 }
 *
 * @param stateMachineId The state machine ID string
 * @returns A stateId object conforming to the StateMachineIdParams interface
 */
export function parseStateMachineId(stateMachineId: string): {
	stateId: { Evm?: number; Substrate?: HexString; Polkadot?: number; Kusama?: number }
} {
	const [type, value] = stateMachineId.split("-")

	if (!type || !value) {
		throw new Error(
			`Invalid state machine ID format: ${stateMachineId}. Expected format like "EVM-97" or "SUBSTRATE-cere"`,
		)
	}

	const stateId: { Evm?: number; Substrate?: HexString; Polkadot?: number; Kusama?: number } = {}

	switch (type.toUpperCase()) {
		case "EVM":
			const evmChainId = Number.parseInt(value, 10)
			if (isNaN(evmChainId)) {
				throw new Error(`Invalid EVM chain ID: ${value}. Expected a number.`)
			}
			stateId.Evm = evmChainId
			break

		case "SUBSTRATE":
			// Convert the string to hex-encoded UTF-8 bytes
			const bytes = Buffer.from(value, "utf8")
			stateId.Substrate = `0x${bytes.toString("hex")}` as HexString
			break

		case "POLKADOT":
			const polkadotChainId = Number.parseInt(value, 10)
			if (isNaN(polkadotChainId)) {
				throw new Error(`Invalid Polkadot chain ID: ${value}. Expected a number.`)
			}
			stateId.Polkadot = polkadotChainId
			break

		case "KUSAMA":
			const kusamaChainId = Number.parseInt(value, 10)
			if (isNaN(kusamaChainId)) {
				throw new Error(`Invalid Kusama chain ID: ${value}. Expected a number.`)
			}
			stateId.Kusama = kusamaChainId
			break

		default:
			throw new Error(`Unsupported chain type: ${type}. Expected one of: EVM, SUBSTRATE, POLKADOT, KUSAMA.`)
	}

	return { stateId }
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
