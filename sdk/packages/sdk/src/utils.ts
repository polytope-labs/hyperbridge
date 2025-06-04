import {
	type HexString,
	type IGetRequest,
	type IPostRequest,
	RequestStatus,
	TimeoutStatus,
	type StateMachineHeight,
	RequestKind,
	EvmLanguage,
} from "@/types"
import type { RequestStatusKey, TimeoutStatusKey, RetryConfig, Order } from "@/types"
import {
	encodePacked,
	keccak256,
	toHex,
	encodeAbiParameters,
	hexToBytes,
	bytesToHex,
	type PublicClient,
	concatHex,
} from "viem"
import { createConsola, LogLevels } from "consola"
import { _queryRequestInternal } from "./query-client"
import { getStateCommitmentFieldSlot, type IChain } from "./chain"
import { generateRootWithProof } from "./utils"
import handler from "./abis/handler"
import evmHost from "./abis/evmHost"

export * from "./utils/mmr"
export * from "./utils/substrate"

export const DEFAULT_POLL_INTERVAL = 5_000
export const ADDRESS_ZERO = "0x0000000000000000000000000000000000000000" as HexString
export const DUMMY_PRIVATE_KEY = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

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
		case "EVM": {
			const evmChainId = Number.parseInt(value, 10)
			if (Number.isNaN(evmChainId)) {
				throw new Error(`Invalid EVM chain ID: ${value}. Expected a number.`)
			}
			stateId.Evm = evmChainId
			break
		}

		case "SUBSTRATE": {
			// Convert the string to hex-encoded UTF-8 bytes
			const bytes = Buffer.from(value, "utf8")
			stateId.Substrate = `0x${bytes.toString("hex")}` as HexString
			break
		}

		case "POLKADOT": {
			const polkadotChainId = Number.parseInt(value, 10)
			if (Number.isNaN(polkadotChainId)) {
				throw new Error(`Invalid Polkadot chain ID: ${value}. Expected a number.`)
			}
			stateId.Polkadot = polkadotChainId
			break
		}

		case "KUSAMA": {
			const kusamaChainId = Number.parseInt(value, 10)
			if (Number.isNaN(kusamaChainId)) {
				throw new Error(`Invalid Kusama chain ID: ${value}. Expected a number.`)
			}
			stateId.Kusama = kusamaChainId
			break
		}

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
 * @returns The commitment hash and the encode packed data.
 */
export function postRequestCommitment(post: IPostRequest): { commitment: HexString; encodePacked: HexString } {
	const data = encodePacked(
		["bytes", "bytes", "uint64", "uint64", "bytes", "bytes", "bytes"],
		[toHex(post.source), toHex(post.dest), post.nonce, post.timeoutTimestamp, post.from, post.to, post.body],
	)

	return {
		commitment: keccak256(data),
		encodePacked: data,
	}
}

export function orderCommitment(order: Order): HexString {
	const encodedOrder = encodeAbiParameters(
		[
			{
				name: "order",
				type: "tuple",
				components: [
					{ name: "user", type: "bytes32" },
					{ name: "sourceChain", type: "bytes" },
					{ name: "destChain", type: "bytes" },
					{ name: "deadline", type: "uint256" },
					{ name: "nonce", type: "uint256" },
					{ name: "fees", type: "uint256" },
					{
						name: "outputs",
						type: "tuple[]",
						components: [
							{ name: "token", type: "bytes32" },
							{ name: "amount", type: "uint256" },
							{ name: "beneficiary", type: "bytes32" },
						],
					},
					{
						name: "inputs",
						type: "tuple[]",
						components: [
							{ name: "token", type: "bytes32" },
							{ name: "amount", type: "uint256" },
						],
					},
					{ name: "callData", type: "bytes" },
				],
			},
		],
		[
			{
				user: order.user,
				sourceChain: toHex(order.sourceChain),
				destChain: toHex(order.destChain),
				deadline: order.deadline,
				nonce: order.nonce,
				fees: order.fees,
				outputs: order.outputs,
				inputs: order.inputs,
				callData: order.callData,
			},
		],
	)

	return keccak256(encodedOrder)
}

/**
 * Converts a bytes32 token address to bytes20 format
 * This removes the extra padded zeros from the address
 */
export function bytes32ToBytes20(bytes32Address: string): HexString {
	if (bytes32Address === ADDRESS_ZERO) {
		return ADDRESS_ZERO
	}

	const bytes = hexToBytes(bytes32Address as HexString)
	const addressBytes = bytes.slice(12)
	return bytesToHex(addressBytes) as HexString
}

export function bytes20ToBytes32(bytes20Address: string): HexString {
	return `0x${bytes20Address.slice(2).padStart(64, "0")}` as HexString
}

export function hexToString(hex: string): string {
	const hexWithoutPrefix = hex.startsWith("0x") ? hex.slice(2) : hex

	const bytes = new Uint8Array(hexWithoutPrefix.length / 2)
	for (let i = 0; i < hexWithoutPrefix.length; i += 2) {
		bytes[i / 2] = Number.parseInt(hexWithoutPrefix.slice(i, i + 2), 16)
	}

	return new TextDecoder().decode(bytes)
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

/**
 * Estimates the gas required for a post request transaction on the source chain.
 * This function constructs a post request, generates mock proofs, and estimates
 * the gas cost for executing the transaction on the source chain.
 */
export async function estimateGasForPost(params: {
	postRequest: IPostRequest
	sourceClient: PublicClient
	hostLatestStateMachineHeight: bigint
	hostAddress: HexString
}): Promise<bigint> {
	const hostParams = await params.sourceClient.readContract({
		address: params.hostAddress,
		abi: evmHost.ABI,
		functionName: "hostParams",
	})

	const { root, proof, index, kIndex, treeSize } = await generateRootWithProof(params.postRequest, 2n ** 10n)
	const latestStateMachineHeight = params.hostLatestStateMachineHeight
	const overlayRootSlot = getStateCommitmentFieldSlot(
		BigInt(4009n), // Hyperbridge chain id
		latestStateMachineHeight, // Hyperbridge chain height
		1, // For overlayRoot
	)
	const postParams = {
		height: {
			stateMachineId: BigInt(4009n),
			height: latestStateMachineHeight,
		},
		multiproof: proof,
		leafCount: treeSize,
	}

	const gas = await params.sourceClient.estimateContractGas({
		address: hostParams.handler,
		abi: handler.ABI,
		functionName: "handlePostRequests",
		args: [
			params.hostAddress,
			{
				proof: postParams,
				requests: [
					{
						request: {
							...params.postRequest,
							source: toHex(params.postRequest.source),
							dest: toHex(params.postRequest.dest),
						},
						index,
						kIndex,
					},
				],
			},
		],
		stateOverride: [
			{
				address: params.hostAddress,
				stateDiff: [
					{
						slot: overlayRootSlot,
						value: root,
					},
				],
			},
		],
	})

	return gas
}

/**
 * Constructs the request body for a redeem escrow operation.
 * This function encodes the order commitment, beneficiary address, and token inputs
 * to match the format expected by the IntentGateway contract.
 */
export function constructRedeemEscrowRequestBody(order: Order, beneficiary: HexString): HexString {
	const commitment = order.id as HexString
	const inputs = order.inputs

	// RequestKind.RedeemEscrow is 0 as defined in the contract
	const requestKind = encodePacked(["uint8"], [RequestKind.RedeemEscrow])

	const requestBody = {
		commitment: commitment as HexString,
		beneficiary: bytes20ToBytes32(beneficiary),
		tokens: inputs,
	}

	const encodedRequestBody = encodeAbiParameters(
		[
			{
				name: "requestBody",
				type: "tuple",
				components: [
					{ name: "commitment", type: "bytes32" },
					{ name: "beneficiary", type: "bytes32" },
					{
						name: "tokens",
						type: "tuple[]",
						components: [
							{ name: "token", type: "bytes32" },
							{ name: "amount", type: "uint256" },
						],
					},
				],
			},
		],
		[requestBody],
	)

	return concatHex([requestKind, encodedRequestBody]) as HexString
}

export const normalizeTimestamp = (timestamp: bigint): bigint => {
	if (timestamp.toString().length <= 11) {
		return timestamp * 1000n
	}
	return timestamp
}

/// Convert ensure a date string is in iso format before getting it's timestamp
export const dateStringtoTimestamp = (date: string): number => {
	if (!date.endsWith("Z")) {
		date = `${date}Z`
	}
	return new Date(date).getTime()
}

/**
 * Calculates the balance mapping location for a given slot and holder address.
 * This function handles the different encoding formats used by Solidity and Vyper.
 *
 * @param slot - The slot number to calculate the mapping location for.
 * @param holder - The address of the holder to calculate the mapping location for.
 * @param language - The language of the contract.
 * @returns The balance mapping location as a HexString.
 */
export function calculateBalanceMappingLocation(slot: bigint, holder: string, language: EvmLanguage): HexString {
	const holderBytes = bytes20ToBytes32(holder)
	const slotBytes = `0x${slot.toString(16).padStart(64, "0")}` as HexString

	if (language === EvmLanguage.Solidity) {
		return keccak256(
			encodeAbiParameters([{ type: "bytes32" }, { type: "bytes32" }], [holderBytes, slotBytes]) as HexString,
		)
	} else {
		// Vyper uses reverse order
		return keccak256(
			encodeAbiParameters([{ type: "bytes32" }, { type: "bytes32" }], [slotBytes, holderBytes]) as HexString,
		)
	}
}
