import {
	type HexString,
	type IGetRequest,
	type IPostRequest,
	RequestKind,
	RequestStatus,
	type StateMachineHeight,
	TimeoutStatus,
} from "@/types"
import type { EstimateGasCallData, Order, RequestStatusKey, RetryConfig, TimeoutStatusKey } from "@/types"
import { LogLevels, createConsola } from "consola"
import {
	type CallParameters,
	type PublicClient,
	bytesToHex,
	concatHex,
	encodeAbiParameters,
	encodePacked,
	hexToBytes,
	keccak256,
	toHex,
} from "viem"
import evmHost from "./abis/evmHost"
import handler from "./abis/handler"
import { type IChain, getStateCommitmentFieldSlot } from "./chain"
import { _queryRequestInternal } from "./query-client"
import { generateRootWithProof } from "./utils"
import { ChainConfigService } from "./configs/ChainConfigService"

export * from "./utils/mmr"
export * from "./utils/substrate"

export const DEFAULT_POLL_INTERVAL = 5_000
export const ADDRESS_ZERO = "0x0000000000000000000000000000000000000000" as HexString
export const MOCK_ADDRESS = "0x1234567890123456789012345678901234567890" as HexString
export const DUMMY_PRIVATE_KEY = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString
export const DEFAULT_GRAFFITI = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

/**
 * Returns the maximum of two bigint values
 * @param a - First bigint value
 * @param b - Second bigint value
 * @returns The larger of the two values
 */
export function maxBigInt(a: bigint, b: bigint): bigint {
	return a > b ? a : b
}

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
				sourceChain: order.sourceChain.startsWith("0x")
					? (order.sourceChain as `0x${string}`)
					: toHex(order.sourceChain),
				destChain: order.destChain.startsWith("0x")
					? (order.destChain as `0x${string}`)
					: toHex(order.destChain),
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

/**
 * Retries a promise-returning operation with exponential backoff.
 * This function will attempt to execute the operation up to maxRetries times,
 * with an exponential backoff delay between attempts.
 *
 * @param operation The async operation to retry
 * @param retryConfig Configuration object containing retry parameters
 * @returns Promise that resolves with the operation result or rejects with the last error
 */
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
}): Promise<{ gas_fee: bigint; call_data: EstimateGasCallData }> {
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

	const call_data: EstimateGasCallData = [
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
	]

	const gas_fee = await params.sourceClient.estimateContractGas({
		address: hostParams.handler,
		abi: handler.ABI,
		functionName: "handlePostRequests",
		args: call_data,
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

	return { gas_fee, call_data }
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
 * Maps testnet identifiers to mainnet identifiers for price lookup
 * @param identifier - The original token identifier (symbol or contract address)
 * @returns The mapped mainnet identifier
 */
export function mapToValidCoingeckoId(identifier: string): string {
	identifier = identifier.toLowerCase()

	switch (identifier) {
		case "bnb":
			return "wbnb"
		case "eth":
			return "weth"
		case "tbnb":
			return "wbnb"
		case "0xc043f483373072f7f27420d6e7d7ad269c018e18".toLowerCase():
			return "dai"
		case "0xae13d989dac2f0debff460ac112a837c89baa7cd".toLowerCase():
			return "wbnb"
		case "0x1938165569A5463327fb206bE06d8D9253aa06b7".toLowerCase():
			return "dai"
		case "0xC625ec7D30A4b1AAEfb1304610CdAcD0d606aC92".toLowerCase():
			return "dai"
		case "0x50B1d3c7c073c9caa1Ef207365A2c9C976bD70b9".toLowerCase():
			return "dai"
		case "0xa801da100bf16d07f668f4a49e1f71fc54d05177".toLowerCase():
			return "dai"
		case "pol":
			return "polygon-ecosystem-token"
		default:
			return identifier
	}
}

export async function fetchPrice(identifier: string, chainId = 1, apiKey?: string): Promise<number> {
	const mappedIdentifier = mapToValidCoingeckoId(identifier)

	const network = new ChainConfigService().getCoingeckoId(`EVM-${chainId}`) || "ethereum"

	apiKey = apiKey || (typeof process !== "undefined" ? (process as any)?.env?.COINGECKO : undefined)
	const baseUrl = apiKey ? "https://pro-api.coingecko.com/api/v3" : "https://api.coingecko.com/api/v3"

	const url = mappedIdentifier.startsWith("0x")
		? `${baseUrl}/simple/token_price/${network}?contract_addresses=${mappedIdentifier}&vs_currencies=usd`
		: `${baseUrl}/simple/price?ids=${mappedIdentifier}&vs_currencies=usd`

	const headers = apiKey ? { "x-cg-pro-api-key": apiKey as string } : undefined

	const response = await fetch(url, { headers })

	if (!response.ok) {
		throw new Error(`CoinGecko API error: ${response.status} ${response.statusText}`)
	}

	const data = await response.json()
	const key = mappedIdentifier.toLowerCase()

	if (!data[key]?.usd) {
		throw new Error(`Price not found for token: ${mappedIdentifier} on ${network}`)
	}

	return data[key].usd
}

/**
 * Fetches the current network gas price from an Etherscan-family explorer API.
 * Returns the ProposeGasPrice (in gwei) converted to wei as bigint.
 */
export async function getGasPriceFromEtherscan(chainId: string, apiKey?: string): Promise<bigint> {
	let parsedChainId = Number(chainId.split("-")[1])
	const url = apiKey
		? `https://api.etherscan.io/v2/api?chainid=${parsedChainId}&module=gastracker&action=gasoracle&apikey=${apiKey}`
		: `https://api.etherscan.io/v2/api?chainid=${parsedChainId}&module=gastracker&action=gasoracle`
	const response = await fetch(url)
	const data = await response.json()
	return gweiToWei(data.result.ProposeGasPrice)
}

/**
 * Converts a decimal gwei string to wei bigint without floating point errors.
 */
function gweiToWei(gwei: string): bigint {
	if (!gwei || typeof gwei !== "string") {
		throw new Error(`Invalid gwei value: ${gwei}`)
	}
	const [intPart, fracPartRaw] = gwei.split(".")
	const fracPart = (fracPartRaw || "").slice(0, 9) // up to 9 decimal places for gwei->wei
	const fracPadded = fracPart.padEnd(9, "0")
	const whole = BigInt(intPart || "0") * 1_000_000_000n
	const fractional = BigInt(fracPadded || "0")
	return whole + fractional
}

/**
 * ERC20 method signatures used for storage slot detection
 */
export enum ERC20Method {
	/** ERC20 balanceOf(address) method signature */
	BALANCE_OF = "0x70a08231",
	/** ERC20 allowance(address,address) method signature */
	ALLOWANCE = "0xdd62ed3e",
}

export enum UniversalRouterCommands {
	WRAP_ETH = 0x0b,
	UNWRAP_WETH = 0x0c,
	V2_SWAP_EXACT_IN = 0x08,
	V2_SWAP_EXACT_OUT = 0x09,
	V3_SWAP_EXACT_IN = 0x00,
	V3_SWAP_EXACT_OUT = 0x01,
	V4_SWAP = 0x10,
	V4_SWAP_EXACT_IN_SINGLE = 0x06,
	V4_SWAP_EXACT_OUT_SINGLE = 0x08,
	SETTLE_ALL = 0x0c,
	TAKE_ALL = 0x0f,
}

/**
 * Retrieves the storage slot for a contract call using debug_traceCall
 *
 * This function uses the Ethereum debug API to trace contract execution and identify
 * the storage slot accessed during the call. It's commonly used for ERC20 token state
 * mappings like balanceOf and allowance, but can work with any contract call that
 * performs SLOAD operations.
 *
 * @param client - The viem PublicClient instance connected to an RPC node with debug API enabled
 * @param tokenAddress - The address of the contract to trace
 * @param data - The full encoded function call data (method signature + encoded parameters)
 * @returns The storage slot as a hex string
 * @throws Error if the storage slot cannot be found or if debug API is not available
 *
 * @example
 * ```ts
 * import { ERC20Method, bytes20ToBytes32 } from '@hyperbridge/sdk'
 *
 * // Get balance storage slot for ERC20
 * const balanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(userAddress).slice(2)
 * const balanceSlot = await getStorageSlot(
 *   client,
 *   tokenAddress,
 *   balanceData as HexString
 * )
 *
 * // Get allowance storage slot for ERC20
 * const allowanceData = ERC20Method.ALLOWANCE +
 *   bytes20ToBytes32(ownerAddress).slice(2) +
 *   bytes20ToBytes32(spenderAddress).slice(2)
 * const allowanceSlot = await getStorageSlot(
 *   client,
 *   tokenAddress,
 *   allowanceData as HexString
 * )
 * ```
 */
export async function getStorageSlot(
	client: PublicClient,
	contractAddress: HexString,
	data: HexString,
): Promise<string> {
	// Default tracer (struct logger)
	async function tryDefaultTracer(): Promise<string> {
		const traceCallClient = client.extend((client) => ({
			async traceCall(args: CallParameters) {
				return client.request({
					// @ts-ignore
					method: "debug_traceCall",
					// @ts-ignore
					params: [args, "latest", {}],
				})
			},
		}))

		const response = await traceCallClient.traceCall({
			to: contractAddress,
			data: data,
		})

		const methodSignature = data.slice(0, 10) as HexString
		// @ts-ignore
		const logs = response.structLogs

		if (!logs || logs.length === 0) {
			throw new Error("No struct logs found")
		}

		for (let i = logs.length - 1; i >= 0; i--) {
			const log = logs[i]
			if (log.op === "SLOAD" && log.stack?.length >= 3) {
				const sigHash = log.stack[0]
				const slotHex = log.stack[log.stack.length - 1]
				// Extract method signature from data (first 4 bytes)
				if (sigHash === methodSignature && slotHex.length === 66) {
					return slotHex
				}
			}
		}

		throw new Error(`Storage slot not found for data: ${methodSignature}`)
	}

	// prestateTracer
	async function tryPrestateTracer(): Promise<string> {
		const traceCallClient = client.extend((client) => ({
			async traceCall(args: CallParameters) {
				return client.request({
					// @ts-ignore
					method: "debug_traceCall",
					// @ts-ignore
					params: [
						// @ts-ignore
						args,
						"latest",
						{
							// @ts-ignore
							tracer: "prestateTracer",
							tracerConfig: {
								disableCode: true,
							},
						},
					],
				})
			},
		}))

		const response = await traceCallClient.traceCall({
			to: contractAddress,
			data: data,
		})

		// @ts-ignore
		let contractData = response[contractAddress.toLowerCase()]

		if (!contractData) {
			// @ts-ignore
			const addressKey = Object.keys(response).find(
				(addr) => addr.toLowerCase() === contractAddress.toLowerCase(),
			)
			if (addressKey) {
				// @ts-ignore
				contractData = response[addressKey]
			}
		}

		if (!contractData || !contractData.storage) {
			throw new Error(`No storage access found for contract ${contractAddress} with data: ${data}`)
		}

		let storageSlots = Object.keys(contractData.storage)

		const PROXY_SLOTS = [
			"0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc", // EIP-1967 implementation
			"0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103", // EIP-1967 beacon
			"0xa3f0ad74e5423aebfd80d3ef4346578335a9a72aeaee59ff6cb3582b35133d50", // EIP-1967 admin
		]

		storageSlots = storageSlots.filter((slot) => !PROXY_SLOTS.includes(slot))

		if (storageSlots.length === 0) {
			throw new Error(`No storage slots accessed for contract ${contractAddress} with data: ${data}`)
		}

		if (storageSlots.length === 1) {
			return storageSlots[0] as HexString
		}

		return storageSlots[storageSlots.length - 1] as HexString
	}

	return tryDefaultTracer().catch(() => tryPrestateTracer())
}

/**
 * Adjusts fee amounts between different decimal precisions.
 * Handles scaling up or down based on the decimal difference.
 *
 * @param feeInFeeToken - The fee amount to adjust
 * @param fromDecimals - The current decimal precision
 * @param toDecimals - The target decimal precision
 * @returns The adjusted fee amount with the target decimal precision
 */
export function adjustFeeDecimals(feeInFeeToken: bigint, fromDecimals: number, toDecimals: number): bigint {
	if (fromDecimals === toDecimals) return feeInFeeToken
	if (fromDecimals < toDecimals) {
		const scaleFactor = BigInt(10 ** (toDecimals - fromDecimals))
		return feeInFeeToken * scaleFactor
	} else {
		const scaleFactor = BigInt(10 ** (fromDecimals - toDecimals))
		return (feeInFeeToken + scaleFactor - 1n) / scaleFactor
	}
}

/**
 * Chains that should prefer the Etherscan API for gas price lookup
 */
export const USE_ETHERSCAN_CHAINS = new Set(["EVM-137", "EVM-56", "EVM-1"])

/**
 * Testnet chains
 */
export const TESTNET_CHAINS = new Set(["EVM-10200", "EVM-97"])

/**
 * Replace Websocket with HTTP is a function that replaces a websocket URL with an HTTP URL.
 * @param url The URL to replace
 */
export function replaceWebsocketWithHttp(url: string): string {
	if (url.startsWith("ws://")) {
		return url.replace("ws://", "http://")
	} else if (url.startsWith("wss://")) {
		return url.replace("wss://", "https://")
	}
	return url
}
