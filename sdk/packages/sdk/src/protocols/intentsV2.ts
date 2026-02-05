import {
	encodeFunctionData,
	decodeFunctionData,
	keccak256,
	toHex,
	encodeAbiParameters,
	decodeAbiParameters,
	concat,
	concatHex,
	pad,
	maxUint256,
	type Hex,
	formatUnits,
	parseUnits,
	parseAbiParameters,
	encodePacked,
	parseEventLogs,
} from "viem"
import { generatePrivateKey, privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import EVM_HOST from "@/abis/evmHost"
import { createSessionKeyStorage, createCancellationStorage, STORAGE_KEYS, type SessionKeyData } from "@/storage"
import {
	type HexString,
	type OrderV2,
	type PackedUserOperation,
	type SubmitBidOptions,
	type EstimateFillOrderV2Params,
	type FillOrderEstimateV2,
	type IPostRequest,
	type IGetRequest,
	type DispatchPost,
	type FillOptionsV2,
	type SelectOptions,
	type FillerBid,
	type IntentOrderStatusUpdate,
	type SelectBidResult,
	type ExecuteIntentOrderOptions,
	type DecodedOrderV2PlacedLog,
	RequestStatus,
	type RequestStatusWithMetadata,
} from "@/types"
import {
	ADDRESS_ZERO,
	bytes32ToBytes20,
	bytes20ToBytes32,
	ERC20Method,
	retryPromise,
	fetchPrice,
	adjustDecimals,
	constructRedeemEscrowRequestBody,
	MOCK_ADDRESS,
	getRecordedStorageSlot,
	sleep,
	DEFAULT_POLL_INTERVAL,
	hexToString,
	getRequestCommitment,
	parseStateMachineId,
	waitForChallengePeriod,
	calculateBalanceMappingLocation,
	EvmLanguage,
} from "@/utils"
import { orderV2Commitment } from "@/utils"
import { Swap } from "@/utils/swap"
import { EvmChain, requestCommitmentKey } from "@/chains/evm"
import { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import { type IGetRequestMessage, type IProof, type SubstrateChain } from "@/chain"
import type { IndexerClient } from "@/client"
import Decimal from "decimal.js"
import IntentGateway from "@/abis/IntentGateway"
import ERC7821ABI from "@/abis/erc7281"
import { type ERC7821Call } from "@/types"

// =============================================================================
// Constants
// =============================================================================

/** Default graffiti value (bytes32 zero) */
export const DEFAULT_GRAFFITI = "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString

/** ERC-7821 single batch execution mode */
export const ERC7821_BATCH_MODE = "0x0100000000000000000000000000000000000000000000000000000000000000" as HexString

/** Bundler RPC method names for ERC-4337 operations */
export const BundlerMethod = {
	/** Submit a user operation to the bundler */
	ETH_SEND_USER_OPERATION: "eth_sendUserOperation",
	/** Get the receipt of a user operation */
	ETH_GET_USER_OPERATION_RECEIPT: "eth_getUserOperationReceipt",
	/** Estimate gas for a user operation */
	ETH_ESTIMATE_USER_OPERATION_GAS: "eth_estimateUserOperationGas",
} as const

/** Response from bundler's eth_estimateUserOperationGas */
export interface BundlerGasEstimate {
	preVerificationGas: HexString
	verificationGasLimit: HexString
	callGasLimit: HexString
	paymasterVerificationGasLimit?: HexString
	paymasterPostOpGasLimit?: HexString
}

export type BundlerMethod = (typeof BundlerMethod)[keyof typeof BundlerMethod]

// =============================================================================
// Types and Interfaces
// =============================================================================

/** Event map for cancellation status updates */
export interface CancelEventMap {
	DESTINATION_FINALIZED: { proof: IProof }
	AWAITING_GET_REQUEST: undefined
	SOURCE_FINALIZED: { metadata: { blockNumber: number } }
	HYPERBRIDGE_DELIVERED: RequestStatusWithMetadata
	HYPERBRIDGE_FINALIZED: RequestStatusWithMetadata
	SOURCE_PROOF_RECEIVED: IProof
}

export type CancelEvent = {
	[K in keyof CancelEventMap]: { status: K; data: CancelEventMap[K] }
}[keyof CancelEventMap]

/** IntentGatewayV2 contract initialization parameters */
export interface IntentGatewayV2Params {
	host: HexString
	dispatcher: HexString
	solverSelection: boolean
	surplusShareBps: bigint
	protocolFeeBps: bigint
	priceOracle: HexString
}

// =============================================================================
// IntentGatewayV2 Class
// =============================================================================

/**
 * IntentGatewayV2 utilities for placing orders, submitting bids, and managing the intent lifecycle.
 *
 * This class provides a complete SDK for interacting with the IntentGatewayV2 protocol:
 * - **Order Placement**: Generate session keys and prepare order transactions
 * - **Bid Management**: Validate, sort, and select optimal bids from solvers
 * - **Execution Flow**: Full lifecycle management from order to completion
 * - **Cancellation**: Handle order cancellation with cross-chain proofs
 *
 * Session keys are automatically managed with environment-appropriate storage
 * (Node.js filesystem, browser localStorage/IndexedDB, or in-memory fallback).
 *
 * @example
 * ```typescript
 * const gateway = new IntentGatewayV2(sourceChain, destChain, coprocessor, bundlerUrl)
 *
 * // Place an order
 * const gen = gateway.preparePlaceOrder(order)
 * const { value: { calldata } } = await gen.next()
 * const txHash = await wallet.sendTransaction({ to: gatewayAddr, data: calldata })
 * const { value: finalOrder } = await gen.next(txHash)
 *
 * // Execute and track
 * for await (const status of gateway.executeIntentOrder({ order: finalOrder })) {
 *   console.log(status.status)
 * }
 * ```
 */
export class IntentGatewayV2 {
	// =========================================================================
	// Static Constants (EIP-712 Type Hashes)
	// =========================================================================

	/** EIP-712 type hash for SelectSolver message */
	static readonly SELECT_SOLVER_TYPEHASH = keccak256(toHex("SelectSolver(bytes32 commitment,address solver)"))

	/** EIP-712 type hash for PackedUserOperation */
	static readonly PACKED_USEROP_TYPEHASH = keccak256(
		toHex(
			"PackedUserOperation(address sender,uint256 nonce,bytes initCode,bytes callData,bytes32 accountGasLimits,uint256 preVerificationGas,bytes32 gasFees,bytes paymasterAndData)",
		),
	)

	/** EIP-712 type hash for EIP712Domain */
	static readonly DOMAIN_TYPEHASH = keccak256(
		toHex("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"),
	)

	// =========================================================================
	// Private Instance Fields
	// =========================================================================

	private readonly sessionKeyStorage = createSessionKeyStorage()
	private readonly cancellationStorage = createCancellationStorage()
	private readonly swap: Swap = new Swap()
	private readonly feeTokenCache: Map<string, { address: HexString; decimals: number }> = new Map()
	private initPromise: Promise<void> | null = null

	// =========================================================================
	// Constructor
	// =========================================================================

	/**
	 * Creates a new IntentGatewayV2 instance.
	 *
	 * @param source - Source chain for order placement
	 * @param dest - Destination chain for order fulfillment
	 * @param intentsCoprocessor - Optional coprocessor for bid fetching and order execution
	 * @param bundlerUrl - Optional ERC-4337 bundler URL for gas estimation and UserOp submission.
	 *
	 */
	constructor(
		public readonly source: EvmChain,
		public readonly dest: EvmChain,
		public readonly intentsCoprocessor?: IntentsCoprocessor,
		public readonly bundlerUrl?: string,
	) {
		this.initPromise = this.initFeeTokenCache()
	}

	// =========================================================================
	// Initialization
	// =========================================================================

	/**
	 * Ensures the fee token cache is initialized before use.
	 * Called automatically by methods that need the cache.
	 */
	async ensureInitialized(): Promise<void> {
		if (this.initPromise) {
			await this.initPromise
		}
	}

	// =========================================================================
	// Order Lifecycle - Placement
	// =========================================================================

	/**
	 * Generator function that prepares and places an order.
	 *
	 * Flow:
	 * 1. Generates a session key and sets `order.session`
	 * 2. Encodes the placeOrder calldata and yields `{ calldata, sessionPrivateKey }`
	 * 3. Waits for the user/frontend to submit the transaction and provide the txHash via `next(txHash)`
	 * 4. Fetches the transaction receipt and extracts the OrderPlaced event
	 * 5. Updates `order.nonce` and `order.inputs` from the actual event data
	 * 6. Computes the commitment and sets `order.id`
	 * 7. Stores the session key and returns the finalized order
	 *
	 * @param order - The order to prepare and place
	 * @yields `{ calldata, sessionPrivateKey }` - Encoded placeOrder calldata and session private key
	 * @returns The finalized order with correct nonce, inputs, and commitment from on-chain event
	 *
	 * @example
	 * ```typescript
	 * const generator = gateway.preparePlaceOrder(order)
	 *
	 * // Step 1: Get calldata and private key
	 * const { value: { calldata, sessionPrivateKey } } = await generator.next()
	 *
	 * // Step 2: Submit transaction using the calldata
	 * const txHash = await walletClient.sendTransaction({
	 *   to: intentGatewayV2Address,
	 *   data: calldata,
	 * })
	 *
	 * // Step 3: Pass txHash back to generator and get finalized order
	 * const { value: finalizedOrder } = await generator.next(txHash)
	 * ```
	 */
	async *preparePlaceOrder(
		order: OrderV2,
	): AsyncGenerator<{ calldata: HexString; sessionPrivateKey: HexString }, OrderV2, HexString> {
		await this.ensureInitialized()

		const privateKey = generatePrivateKey()
		const account = privateKeyToAccount(privateKey)
		const sessionKeyAddress = account.address as HexString

		order.session = sessionKeyAddress

		const calldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "placeOrder",
			args: [transformOrderForContract(order), DEFAULT_GRAFFITI],
		}) as HexString

		const txHash: HexString = yield { calldata, sessionPrivateKey: privateKey as HexString }

		const receipt = await this.source.client.getTransactionReceipt({
			hash: txHash,
		})

		const events = parseEventLogs({
			abi: IntentGatewayV2ABI,
			logs: receipt.logs,
			eventName: "OrderPlaced",
		})

		const orderPlacedEvent = events[0] as DecodedOrderV2PlacedLog | undefined
		if (!orderPlacedEvent) {
			throw new Error("OrderPlaced event not found in transaction receipt")
		}

		order.nonce = orderPlacedEvent.args.nonce
		order.inputs = orderPlacedEvent.args.inputs.map((input) => ({
			token: input.token,
			amount: input.amount,
		}))

		order.id = orderV2Commitment(order)

		const sessionKeyData: SessionKeyData = {
			privateKey: privateKey as HexString,
			address: sessionKeyAddress,
			commitment: order.id as HexString,
			createdAt: Date.now(),
		}

		await this.sessionKeyStorage.setSessionKey(order.id as HexString, sessionKeyData)

		return order
	}

	// =========================================================================
	// Order Lifecycle - Execution
	// =========================================================================

	/**
	 * Generator function that orchestrates the full intent order execution flow.
	 *
	 * Flow: AWAITING_BIDS → BIDS_RECEIVED → BID_SELECTED → USEROP_SUBMITTED
	 *
	 * Requires `intentsCoprocessor` and `bundlerUrl` to be set in the constructor.
	 *
	 * @param options - Execution options including the order and optional parameters
	 * @yields Status updates throughout the execution flow
	 *
	 * @example
	 * ```typescript
	 * for await (const status of gateway.executeIntentOrder({ order, orderTxHash: txHash })) {
	 *   switch (status.status) {
	 *     case 'AWAITING_BIDS':
	 *       console.log('Waiting for solver bids...')
	 *       break
	 *     case 'BIDS_RECEIVED':
	 *       console.log(`Received ${status.metadata.bidCount} bids`)
	 *       break
	 *     case 'BID_SELECTED':
	 *       console.log(`Selected solver: ${status.metadata.selectedSolver}`)
	 *       break
	 *     case 'USEROP_SUBMITTED':
	 *       console.log(`UserOp submitted: ${status.metadata.userOpHash}`)
	 *       break
	 *   }
	 * }
	 * ```
	 */
	async *executeIntentOrder(options: ExecuteIntentOrderOptions): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const {
			order,
			sessionPrivateKey,
			minBids = 1,
			bidTimeoutMs = 60_000,
			pollIntervalMs = DEFAULT_POLL_INTERVAL,
		} = options

		const commitment = order.id as HexString

		if (!this.intentsCoprocessor) {
			yield {
				status: "FAILED",
				metadata: { error: "IntentsCoprocessor required for order execution" },
			}
			return
		}

		if (!this.bundlerUrl) {
			yield {
				status: "FAILED",
				metadata: { error: "Bundler URL not configured" },
			}
			return
		}

		try {
			yield {
				status: "AWAITING_BIDS",
				metadata: { commitment },
			}

			const startTime = Date.now()
			let bids: FillerBid[] = []

			while (Date.now() - startTime < bidTimeoutMs) {
				try {
					bids = await this.intentsCoprocessor.getBidsForOrder(commitment)

					if (bids.length >= minBids) {
						break
					}
				} catch {
					// Continue polling on errors
				}

				await sleep(pollIntervalMs)
			}

			if (bids.length === 0) {
				yield {
					status: "FAILED",
					metadata: {
						commitment,
						error: `No bids received within ${bidTimeoutMs}ms timeout`,
					},
				}
				return
			}

			yield {
				status: "BIDS_RECEIVED",
				metadata: {
					commitment,
					bidCount: bids.length,
					bids,
				},
			}

			try {
				const result = await this.selectBid(order, bids, sessionPrivateKey)

				yield {
					status: "BID_SELECTED",
					metadata: {
						commitment,
						selectedSolver: result.solverAddress,
						userOpHash: result.userOpHash,
						userOp: result.userOp,
					},
				}

				yield {
					status: "USEROP_SUBMITTED",
					metadata: {
						commitment,
						userOpHash: result.userOpHash,
						selectedSolver: result.solverAddress,
						transactionHash: result.txnHash,
					},
				}
			} catch (err) {
				yield {
					status: "FAILED",
					metadata: {
						commitment,
						error: `Failed to select bid and submit: ${err instanceof Error ? err.message : String(err)}`,
					},
				}
				return
			}
		} catch (err) {
			yield {
				status: "FAILED",
				metadata: {
					commitment,
					error: `Unexpected error: ${err instanceof Error ? err.message : String(err)}`,
				},
			}
		}
	}

	// =========================================================================
	// Order Lifecycle - Cancellation
	// =========================================================================

	/**
	 * Returns the native token amount required to dispatch a cancellation GET request.
	 *
	 * @param order - The order to get the cancellation quote for
	 * @returns Native token amount required for the cancellation GET request
	 */
	async quoteCancelNative(order: OrderV2): Promise<bigint> {
		const height = order.deadline + 1n

		const destIntentGateway = this.dest.configService.getIntentGatewayV2Address(
			hexToString(order.destination as HexString),
		)
		const slotHash = await this.dest.client.readContract({
			abi: IntentGatewayV2ABI,
			address: destIntentGateway,
			functionName: "calculateCommitmentSlotHash",
			args: [order.id as HexString],
		})
		const key = concatHex([destIntentGateway as HexString, slotHash as HexString]) as HexString

		const context = encodeAbiParameters(
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
			[
				{
					commitment: order.id as HexString,
					beneficiary: order.user as HexString,
					tokens: order.inputs,
				},
			],
		) as HexString

		const getRequest: IGetRequest = {
			source: order.source.startsWith("0x") ? hexToString(order.source as HexString) : order.source,
			dest: order.destination.startsWith("0x") ? hexToString(order.destination as HexString) : order.destination,
			from: this.source.configService.getIntentGatewayV2Address(hexToString(order.destination as HexString)),
			nonce: await this.source.getHostNonce(),
			height,
			keys: [key],
			timeoutTimestamp: 0n,
			context,
		}

		return await this.source.quoteNative(getRequest, 0n)
	}

	/**
	 * Generator function that handles the full order cancellation flow.
	 *
	 * This allows users to cancel orders that haven't been filled by the deadline.
	 *
	 * Flow:
	 * 1. Fetch proof that the order wasn't filled on the destination chain
	 * 2. Submit a GET request to read the unfilled order state
	 * 3. Wait for the GET request to be processed through Hyperbridge
	 * 4. Finalize the cancellation on Hyperbridge
	 *
	 * @param order - The order to cancel
	 * @param indexerClient - Client for querying the indexer
	 * @yields Status updates throughout the cancellation process
	 *
	 * @example
	 * ```typescript
	 * const cancelStream = gateway.cancelOrder(order, indexerClient)
	 *
	 * for await (const event of cancelStream) {
	 *   switch (event.status) {
	 *     case 'DESTINATION_FINALIZED':
	 *       console.log('Got destination proof')
	 *       break
	 *     case 'AWAITING_GET_REQUEST':
	 *       const txHash = await submitCancelTx()
	 *       cancelStream.next(txHash)
	 *       break
	 *     case 'SOURCE_FINALIZED':
	 *       console.log('Source finalized')
	 *       break
	 *     case 'HYPERBRIDGE_DELIVERED':
	 *       console.log('Delivered to Hyperbridge')
	 *       break
	 *     case 'HYPERBRIDGE_FINALIZED':
	 *       console.log('Cancellation complete')
	 *       break
	 *   }
	 * }
	 * ```
	 */
	async *cancelOrder(order: OrderV2, indexerClient: IndexerClient): AsyncGenerator<CancelEvent> {
		const orderId = order.id!

		const hyperbridge = indexerClient.hyperbridge as SubstrateChain
		const sourceStateMachine = hexToString(order.source as HexString)
		const sourceConsensusStateId = this.source.configService.getConsensusStateId(sourceStateMachine)

		let destIProof: IProof | null = await this.cancellationStorage.getItem(STORAGE_KEYS.destProof(orderId))
		if (!destIProof) {
			destIProof = yield* this.fetchDestinationProof(order, indexerClient)
			await this.cancellationStorage.setItem(STORAGE_KEYS.destProof(orderId), destIProof)
		} else {
			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		}

		let getRequest: IGetRequest | null = await this.cancellationStorage.getItem(STORAGE_KEYS.getRequest(orderId))
		if (!getRequest) {
			const transactionHash = yield {
				status: "AWAITING_GET_REQUEST",
				data: undefined,
			}
			const receipt = await this.source.client.getTransactionReceipt({
				hash: transactionHash,
			})

			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const request = events.find((e) => e.eventName === "GetRequestEvent")
			if (!request) throw new Error("GetRequest missing")
			getRequest = request.args as unknown as IGetRequest

			await this.cancellationStorage.setItem(STORAGE_KEYS.getRequest(orderId), getRequest)
		}

		const commitment = getRequestCommitment({
			...getRequest,
			keys: [...getRequest.keys],
		})
		const sourceStatusStream = indexerClient.getRequestStatusStream(commitment)

		for await (const statusUpdate of sourceStatusStream) {
			if (statusUpdate.status === RequestStatus.SOURCE_FINALIZED) {
				yield {
					status: "SOURCE_FINALIZED",
					data: { metadata: statusUpdate.metadata },
				}

				const sourceHeight = BigInt(statusUpdate.metadata.blockNumber)
				let sourceIProof: IProof | null = await this.cancellationStorage.getItem(
					STORAGE_KEYS.sourceProof(orderId),
				)
				if (!sourceIProof) {
					sourceIProof = await fetchSourceProof(
						commitment,
						this.source,
						sourceStateMachine,
						sourceConsensusStateId,
						sourceHeight,
					)
					await this.cancellationStorage.setItem(STORAGE_KEYS.sourceProof(orderId), sourceIProof)
				}

				await waitForChallengePeriod(hyperbridge, {
					height: sourceIProof.height,
					id: {
						stateId: parseStateMachineId(sourceStateMachine).stateId,
						consensusStateId: sourceConsensusStateId,
					},
				})

				const getRequestMessage: IGetRequestMessage = {
					kind: "GetRequest",
					requests: [getRequest],
					source: sourceIProof,
					response: destIProof,
					signer: pad("0x"),
				}

				await this.submitAndConfirmReceipt(hyperbridge, commitment, getRequestMessage)
				continue
			}

			if (statusUpdate.status === RequestStatus.HYPERBRIDGE_DELIVERED) {
				yield {
					status: "HYPERBRIDGE_DELIVERED",
					data: statusUpdate as RequestStatusWithMetadata,
				}
				continue
			}

			if (statusUpdate.status === RequestStatus.HYPERBRIDGE_FINALIZED) {
				yield {
					status: "HYPERBRIDGE_FINALIZED",
					data: statusUpdate as RequestStatusWithMetadata,
				}
				await this.cancellationStorage.removeItem(STORAGE_KEYS.destProof(orderId))
				await this.cancellationStorage.removeItem(STORAGE_KEYS.getRequest(orderId))
				await this.cancellationStorage.removeItem(STORAGE_KEYS.sourceProof(orderId))
				return
			}
		}
	}

	// =========================================================================
	// Bid Management
	// =========================================================================

	/**
	 * Prepares a bid UserOperation for submitting to Hyperbridge (used by fillers/solvers).
	 *
	 * The callData is encoded using ERC-7821 batch executor format since SolverAccount
	 * extends ERC7821. Format: `execute(bytes32 mode, bytes executionData)`
	 * where executionData contains the fillOrder call to IntentGatewayV2.
	 *
	 * @param options - Bid submission options including order, fillOptions, and gas parameters
	 * @returns PackedUserOperation ready for submission to Hyperbridge
	 */
	async prepareSubmitBid(options: SubmitBidOptions): Promise<PackedUserOperation> {
		const {
			order,
			fillOptions,
			solverAccount,
			solverPrivateKey,
			nonce,
			entryPointAddress,
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
		} = options

		const chainId = BigInt(
			this.dest.client.chain?.id ?? Number.parseInt(this.dest.config.stateMachineId.split("-")[1]),
		)

		// Encode the inner fillOrder call to IntentGatewayV2
		const fillOrderCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [transformOrderForContract(order), fillOptions],
		}) as HexString

		// Calculate the native value needed for fillOrder (native outputs + dispatch fee)
		const nativeOutputValue = order.output.assets
			.filter((asset) => bytes32ToBytes20(asset.token) === ADDRESS_ZERO)
			.reduce((sum, asset) => sum + asset.amount, 0n)
		const totalNativeValue = nativeOutputValue + fillOptions.nativeDispatchFee

		const intentGatewayV2Address = this.dest.configService.getIntentGatewayV2Address(order.destination)

		const callData = this.encodeERC7821Execute([
			{
				target: intentGatewayV2Address,
				value: totalNativeValue,
				data: fillOrderCalldata,
			},
		])

		const accountGasLimits = this.packGasLimits(verificationGasLimit, callGasLimit)
		const gasFees = this.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

		const userOp: PackedUserOperation = {
			sender: solverAccount,
			nonce,
			initCode: "0x" as HexString,
			callData,
			accountGasLimits,
			preVerificationGas,
			gasFees,
			paymasterAndData: "0x" as HexString,
			signature: "0x" as HexString, // Will be signed later
		}

		const userOpHash = this.computeUserOpHash(userOp, entryPointAddress, chainId)
		const sessionKey = order.session

		// Sign: keccak256(abi.encodePacked(userOpHash, commitment, sessionKey))
		// sessionKey is address (20 bytes), not padded to 32
		const messageHash = keccak256(concat([userOpHash, order.id as HexString, sessionKey as Hex]))

		const solverAccount_ = privateKeyToAccount(solverPrivateKey as Hex)
		const solverSignature = await solverAccount_.signMessage({ message: { raw: messageHash } })

		// Signature: commitment (32 bytes) + solverSignature (65 bytes)
		const signature = concat([order.id as HexString, solverSignature as Hex]) as HexString

		return { ...userOp, signature }
	}

	/**
	 * Selects the best bid from Hyperbridge and submits to the bundler.
	 *
	 * Flow:
	 * 1. Fetches bids from Hyperbridge
	 * 2. Validates and sorts bids by USD value (WETH price fetched via swap, USDC/USDT at $1)
	 * 3. Tries each bid (best to worst) until one passes simulation
	 * 4. Signs and submits the winning bid to the bundler
	 *
	 * Requires `bundlerUrl` and `intentsCoprocessor` to be set in the constructor.
	 *
	 * @param order - The order to select a bid for
	 * @param bids - Array of filler bids to evaluate
	 * @param sessionPrivateKey - Optional session private key (retrieved from storage if not provided)
	 * @returns Result containing the selected bid, userOp, and transaction details
	 */
	async selectBid(order: OrderV2, bids: FillerBid[], sessionPrivateKey?: HexString): Promise<SelectBidResult> {
		const commitment = order.id as HexString
		const sessionKeyData = sessionPrivateKey
			? { privateKey: sessionPrivateKey as HexString }
			: await this.sessionKeyStorage.getSessionKey(commitment)
		if (!sessionKeyData) {
			throw new Error("SessionKey not found for commitment: " + commitment)
		}

		if (!this.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}

		if (!this.intentsCoprocessor) {
			throw new Error("IntentsCoprocessor required")
		}

		// Validate and sort bids by USD value (best to worst)
		const sortedBids = await this.validateAndSortBids(bids, order)
		if (sortedBids.length === 0) {
			throw new Error("No valid bids found")
		}

		const intentGatewayV2Address = this.dest.configService.getIntentGatewayV2Address(hexToString(order.destination))

		const domainSeparator = await this.getDomainSeparator(
			"IntentGateway",
			"2",
			BigInt(this.dest.client.chain?.id ?? Number.parseInt(this.dest.config.stateMachineId.split("-")[1])),
			intentGatewayV2Address,
		)

		// Try each bid in order (best to worst) until one passes simulation
		let selectedBid: { bid: FillerBid; options: FillOptionsV2 } | null = null
		let sessionSignature: HexString | null = null

		for (const bidWithOptions of sortedBids) {
			const solverAddress = bidWithOptions.bid.userOp.sender

			// Sign for this solver (must re-sign for each different solver)
			const signature = await this.signSolverSelection(
				commitment,
				solverAddress,
				domainSeparator,
				sessionKeyData.privateKey,
			)
			if (!signature) {
				console.log("Signature is null")
				continue
			}

			const selectOptions: SelectOptions = {
				commitment,
				solver: solverAddress,
				signature,
			}

			// Try simulation
			try {
				await this.simulateAndValidate(
					order,
					selectOptions,
					bidWithOptions.options,
					solverAddress,
					intentGatewayV2Address,
				)
				// Simulation succeeded, use this bid
				selectedBid = bidWithOptions
				sessionSignature = signature
				break
			} catch {
				// Simulation failed, try next bid
				continue
			}
		}

		if (!selectedBid || !sessionSignature) {
			throw new Error("No bids passed simulation")
		}

		const solverAddress = selectedBid.bid.userOp.sender

		const finalSignature = concat([selectedBid.bid.userOp.signature as Hex, sessionSignature as Hex]) as HexString

		const signedUserOp: PackedUserOperation = {
			...selectedBid.bid.userOp,
			signature: finalSignature,
		}

		const entryPointAddress = this.dest.configService.getEntryPointV08Address(hexToString(order.destination))
		const chainId = BigInt(
			this.dest.client.chain?.id ?? Number.parseInt(this.dest.config.stateMachineId.split("-")[1]),
		)
		const userOpHash = this.computeUserOpHash(signedUserOp, entryPointAddress, chainId)

		const bundlerResult = await this.sendBundler<HexString>(BundlerMethod.ETH_SEND_USER_OPERATION, [
			this.prepareBundlerCall(signedUserOp),
			entryPointAddress,
		])

		const finalUserOpHash = bundlerResult || userOpHash

		// Poll for receipt to get txnHash
		let txnHash: HexString | undefined
		try {
			const receipt = await retryPromise(
				async () => {
					const result = await this.sendBundler<{ receipt: { transactionHash: HexString } } | null>(
						BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT,
						[finalUserOpHash],
					)
					if (!result?.receipt?.transactionHash) {
						throw new Error("Receipt not available yet")
					}
					return result
				},
				{ maxRetries: 5, backoffMs: 2000, logMessage: "Fetching user operation receipt" },
			)
			txnHash = receipt.receipt.transactionHash
		} catch {
			// Receipt may not be available after retries, txnHash will be undefined
		}

		return {
			userOp: signedUserOp,
			userOpHash: finalUserOpHash,
			solverAddress,
			commitment,
			txnHash,
		}
	}

	// =========================================================================
	// Gas Estimation
	// =========================================================================

	/**
	 * Estimates gas costs for fillOrder execution via ERC-4337.
	 *
	 * Calculates all gas parameters needed for UserOperation submission:
	 * - `callGasLimit`: Gas for fillOrder execution
	 * - `verificationGasLimit`: Gas for SolverAccount.validateUserOp
	 * - `preVerificationGas`: Bundler overhead for calldata
	 * - Gas prices based on current network conditions
	 *
	 * Uses the bundler's eth_estimateUserOperationGas method for accurate gas estimation
	 * when a bundler URL is configured.
	 *
	 * @param params - Estimation parameters including order and solver account
	 * @returns Complete gas estimate with all ERC-4337 parameters
	 */
	async estimateFillOrderV2(params: EstimateFillOrderV2Params): Promise<FillOrderEstimateV2> {
		await this.ensureInitialized()

		const { order, solverPrivateKey } = params
		const solverAccountAddress = privateKeyToAddress(solverPrivateKey)
		const intentGatewayV2Address = this.dest.configService.getIntentGatewayV2Address(order.destination)
		const entryPointAddress = this.dest.configService.getEntryPointV08Address(order.destination)
		const chainId = BigInt(
			this.dest.client.chain?.id ?? Number.parseInt(this.dest.config.stateMachineId.split("-")[1]),
		)

		// Calculate total native value from output assets
		const totalEthValue = order.output.assets
			.filter((output) => bytes32ToBytes20(output.token) === ADDRESS_ZERO)
			.reduce((sum, output) => sum + output.amount, 0n)

		// Build assets array for state overrides, including fee token if not already present
		const sourceFeeToken = this.feeTokenCache.get(this.source.config.stateMachineId)!
		const destFeeToken = this.feeTokenCache.get(this.dest.config.stateMachineId)!
		const feeTokenAsBytes32 = bytes20ToBytes32(destFeeToken.address)
		const assetsForOverrides = [...order.output.assets]
		if (!assetsForOverrides.some((asset) => asset.token.toLowerCase() === feeTokenAsBytes32.toLowerCase())) {
			assetsForOverrides.push({ token: feeTokenAsBytes32, amount: 0n })
		}

		// Build state overrides once - used for both viem and bundler estimation
		const { viem: stateOverrides, bundler: bundlerStateOverrides } = this.buildStateOverride({
			accountAddress: solverAccountAddress,
			chain: order.destination,
			outputAssets: assetsForOverrides,
			spenderAddress: intentGatewayV2Address,
			intentGatewayV2Address,
			entryPointAddress,
		})

		// Calculate protocol fees for the post request
		const postRequestGas = 400_000n
		const postRequestFeeInSourceFeeToken = await this.convertGasToFeeToken(postRequestGas, "source", order.source)
		let postRequestFeeInDestFeeToken = adjustDecimals(
			postRequestFeeInSourceFeeToken,
			sourceFeeToken.decimals,
			destFeeToken.decimals,
		)

		const postRequest: IPostRequest = {
			source: order.destination,
			dest: order.source,
			body: constructRedeemEscrowRequestBody({ ...order, id: orderV2Commitment(order) }, MOCK_ADDRESS),
			timeoutTimestamp: 0n,
			nonce: await this.source.getHostNonce(),
			from: this.source.configService.getIntentGatewayV2Address(order.destination),
			to: this.source.configService.getIntentGatewayV2Address(order.source),
		}

		let protocolFeeInNativeToken = await this.quoteNative(postRequest, postRequestFeeInDestFeeToken).catch(() =>
			this.dest.quoteNative(postRequest, postRequestFeeInDestFeeToken).catch(() => 0n),
		)

		// Add 0.5% buffer to fees
		protocolFeeInNativeToken = (protocolFeeInNativeToken * 1005n) / 1000n
		postRequestFeeInDestFeeToken = (postRequestFeeInDestFeeToken * 1005n) / 1000n

		const fillOptions: FillOptionsV2 = {
			relayerFee: postRequestFeeInDestFeeToken,
			nativeDispatchFee: protocolFeeInNativeToken,
			outputs: order.output.assets,
		}

		const totalNativeValue = totalEthValue + fillOptions.nativeDispatchFee

		// Calculate gas prices with configurable bumps (defaults: 8% for priority, 10% for max)
		const gasPrice = await this.dest.client.getGasPrice()
		const priorityFeeBumpPercent = params.maxPriorityFeePerGasBumpPercent ?? 8
		const maxFeeBumpPercent = params.maxFeePerGasBumpPercent ?? 10
		const maxPriorityFeePerGas = gasPrice + (gasPrice * BigInt(priorityFeeBumpPercent)) / 100n
		const maxFeePerGas = gasPrice + (gasPrice * BigInt(maxFeeBumpPercent)) / 100n

		// Create order for estimation with solver's address as session
		const orderForEstimation = { ...order, session: solverAccountAddress }
		const commitment = orderV2Commitment(orderForEstimation)

		// Build fillOrder calldata once
		const fillOrderCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [transformOrderForContract(orderForEstimation), fillOptions],
		}) as HexString

		// Get nonce from EntryPoint (2D nonce with commitment as key)
		const nonce = await this.dest.client.readContract({
			address: entryPointAddress,
			abi: [
				{
					inputs: [
						{ name: "sender", type: "address" },
						{ name: "key", type: "uint192" },
					],
					name: "getNonce",
					outputs: [{ name: "nonce", type: "uint256" }],
					stateMutability: "view",
					type: "function",
				},
			],
			functionName: "getNonce",
			args: [solverAccountAddress, BigInt(commitment) & ((1n << 192n) - 1n)],
		})

		// Initialize gas values with fallbacks
		let callGasLimit: bigint = 500_000n
		let verificationGasLimit: bigint = 100_000n
		let preVerificationGas: bigint = 100_000n

		// Estimate gas using bundler if configured, otherwise use direct estimation
		if (this.bundlerUrl && solverPrivateKey) {
			try {
				const callData = this.encodeERC7821Execute([
					{ target: intentGatewayV2Address, value: totalNativeValue, data: fillOrderCalldata },
				])

				const solverAccount = privateKeyToAccount(solverPrivateKey as Hex)
				const accountGasLimits = this.packGasLimits(100_000n, callGasLimit)
				const gasFees = this.packGasFees(maxPriorityFeePerGas, maxFeePerGas)

				// Build preliminary UserOp for bundler estimation
				const preliminaryUserOp: PackedUserOperation = {
					sender: solverAccountAddress,
					nonce,
					initCode: "0x" as HexString,
					callData,
					accountGasLimits,
					preVerificationGas: 100_000n,
					gasFees,
					paymasterAndData: "0x" as HexString,
					signature: "0x" as HexString,
				}

				// Sign the UserOp
				const userOpHash = this.computeUserOpHash(preliminaryUserOp, entryPointAddress, chainId)
				const messageHash = keccak256(
					concat([userOpHash, commitment as HexString, solverAccountAddress as Hex]),
				)
				const solverSignature = await solverAccount.signMessage({ message: { raw: messageHash } })
				const solverSig = concat([commitment as HexString, solverSignature as Hex]) as HexString

				const domainSeparator = this.getDomainSeparator("IntentGateway", "2", chainId, intentGatewayV2Address)
				const sessionSignature = await this.signSolverSelection(
					commitment as HexString,
					solverAccountAddress,
					domainSeparator,
					solverPrivateKey,
				)

				preliminaryUserOp.signature = concat([solverSig as Hex, sessionSignature as Hex]) as HexString

				const bundlerUserOp = this.prepareBundlerCall(preliminaryUserOp)
				const gasEstimate = await this.sendBundler<BundlerGasEstimate>(
					BundlerMethod.ETH_ESTIMATE_USER_OPERATION_GAS,
					[bundlerUserOp, entryPointAddress, bundlerStateOverrides],
				)

				// Parse gas values and add 5% buffer for safety margin
				callGasLimit = (BigInt(gasEstimate.callGasLimit) * 105n) / 100n
				verificationGasLimit = (BigInt(gasEstimate.verificationGasLimit) * 105n) / 100n
				preVerificationGas = (BigInt(gasEstimate.preVerificationGas) * 105n) / 100n
			} catch (e) {
				console.warn("Bundler gas estimation failed, using fallback values:", e)
			}
		} else {
			// Direct gas estimation without bundler
			try {
				const estimatedGas = await this.dest.client.estimateContractGas({
					abi: IntentGatewayV2ABI,
					address: intentGatewayV2Address,
					functionName: "fillOrder",
					args: [transformOrderForContract(order), fillOptions],
					account: solverAccountAddress,
					value: totalNativeValue,
					stateOverride: stateOverrides as any,
				})
				callGasLimit = (estimatedGas * 105n) / 100n // Add 5% buffer
			} catch (e) {
				console.warn("fillOrder gas estimation failed, using fallback:", e)
			}
		}

		// Calculate total gas cost
		const totalGas = callGasLimit + verificationGasLimit + preVerificationGas
		const totalGasCostWei = totalGas * maxFeePerGas
		const totalGasInFeeToken = await this.convertGasToFeeToken(totalGasCostWei, "dest", order.destination)

		return {
			callGasLimit,
			verificationGasLimit,
			preVerificationGas,
			maxFeePerGas,
			maxPriorityFeePerGas,
			totalGasCostWei,
			totalGasInFeeToken,
			fillOptions,
			nonce,
		}
	}

	// =========================================================================
	// EIP-712 and Signature Utilities
	// =========================================================================

	/**
	 * Computes the EIP-712 domain separator for a contract.
	 *
	 * @param contractName - Contract name (e.g., "IntentGateway", "ERC4337")
	 * @param version - Contract version
	 * @param chainId - Chain ID
	 * @param contractAddress - Contract address
	 * @returns The domain separator hash
	 */
	getDomainSeparator(contractName: string, version: string, chainId: bigint, contractAddress: HexString): HexString {
		return keccak256(
			encodeAbiParameters(parseAbiParameters("bytes32, bytes32, bytes32, uint256, address"), [
				IntentGatewayV2.DOMAIN_TYPEHASH,
				keccak256(toHex(contractName)),
				keccak256(toHex(version)),
				chainId,
				contractAddress,
			]),
		)
	}

	/**
	 * Signs a solver selection message using the session key (EIP-712).
	 *
	 * @param commitment - Order commitment hash
	 * @param solverAddress - Address of the selected solver
	 * @param domainSeparator - EIP-712 domain separator
	 * @param privateKey - Session private key
	 * @returns The signature or null if signing fails
	 */
	async signSolverSelection(
		commitment: HexString,
		solverAddress: HexString,
		domainSeparator: HexString,
		privateKey: HexString,
	): Promise<HexString | null> {
		const account = privateKeyToAccount(privateKey as Hex)

		const structHash = keccak256(
			encodeAbiParameters(
				[{ type: "bytes32" }, { type: "bytes32" }, { type: "address" }],
				[IntentGatewayV2.SELECT_SOLVER_TYPEHASH, commitment, solverAddress],
			),
		)

		const digest = keccak256(concat(["0x1901" as Hex, domainSeparator as Hex, structHash]))
		const signature = await account.sign({ hash: digest })

		return signature as HexString
	}

	// =========================================================================
	// UserOperation Utilities
	// =========================================================================

	/**
	 * Computes the EIP-4337 UserOperation hash.
	 *
	 * @param userOp - The packed user operation
	 * @param entryPoint - EntryPoint contract address
	 * @param chainId - Chain ID
	 * @returns The UserOperation hash
	 */
	computeUserOpHash(userOp: PackedUserOperation, entryPoint: Hex, chainId: bigint): Hex {
		const structHash = this.getPackedUserStructHash(userOp)

		const domainSeparator = this.getDomainSeparator("ERC4337", "1", chainId, entryPoint)

		return keccak256(
			encodePacked(["bytes1", "bytes1", "bytes32", "bytes32"], ["0x19", "0x01", domainSeparator, structHash]),
		)
	}

	/**
	 * Gets the packed user struct hash for a UserOperation.
	 *
	 * @param userOp - The UserOperation to hash
	 * @returns The struct hash
	 */
	getPackedUserStructHash(userOp: PackedUserOperation): HexString {
		return keccak256(
			encodeAbiParameters(
				parseAbiParameters("bytes32, address, uint256, bytes32, bytes32, bytes32, uint256, bytes32, bytes32"),
				[
					IntentGatewayV2.PACKED_USEROP_TYPEHASH,
					userOp.sender,
					userOp.nonce,
					keccak256(userOp.initCode),
					keccak256(userOp.callData),
					userOp.accountGasLimits as Hex,
					userOp.preVerificationGas,
					userOp.gasFees as Hex,
					keccak256(userOp.paymasterAndData),
				],
			),
		) as HexString
	}

	/** Packs verificationGasLimit and callGasLimit into bytes32 */
	packGasLimits(verificationGasLimit: bigint, callGasLimit: bigint): HexString {
		const verificationGasHex = pad(toHex(verificationGasLimit), { size: 16 })
		const callGasHex = pad(toHex(callGasLimit), { size: 16 })
		return concat([verificationGasHex, callGasHex]) as HexString
	}

	/** Packs maxPriorityFeePerGas and maxFeePerGas into bytes32 */
	packGasFees(maxPriorityFeePerGas: bigint, maxFeePerGas: bigint): HexString {
		const priorityFeeHex = pad(toHex(maxPriorityFeePerGas), { size: 16 })
		const maxFeeHex = pad(toHex(maxFeePerGas), { size: 16 })
		return concat([priorityFeeHex, maxFeeHex]) as HexString
	}

	/** Unpacks accountGasLimits (bytes32) into verificationGasLimit and callGasLimit */
	unpackGasLimits(accountGasLimits: HexString): { verificationGasLimit: bigint; callGasLimit: bigint } {
		const hex = accountGasLimits.slice(2) // remove 0x
		const verificationGasLimit = BigInt(`0x${hex.slice(0, 32)}`)
		const callGasLimit = BigInt(`0x${hex.slice(32, 64)}`)
		return { verificationGasLimit, callGasLimit }
	}

	/** Unpacks gasFees (bytes32) into maxPriorityFeePerGas and maxFeePerGas */
	unpackGasFees(gasFees: HexString): { maxPriorityFeePerGas: bigint; maxFeePerGas: bigint } {
		const hex = gasFees.slice(2) // remove 0x
		const maxPriorityFeePerGas = BigInt(`0x${hex.slice(0, 32)}`)
		const maxFeePerGas = BigInt(`0x${hex.slice(32, 64)}`)
		return { maxPriorityFeePerGas, maxFeePerGas }
	}

	/**
	 * Converts a PackedUserOperation to bundler-compatible v0.7/v0.8 format.
	 * Unpacks gas limits and fees, extracts factory/paymaster data from packed fields.
	 *
	 * @param userOp - The packed user operation to convert
	 * @returns Bundler-compatible user operation object
	 */
	prepareBundlerCall(userOp: PackedUserOperation): Record<string, unknown> {
		const { verificationGasLimit, callGasLimit } = this.unpackGasLimits(userOp.accountGasLimits)
		const { maxPriorityFeePerGas, maxFeePerGas } = this.unpackGasFees(userOp.gasFees)

		// Convert initCode to factory/factoryData
		const hasFactory = userOp.initCode && userOp.initCode !== "0x" && userOp.initCode.length > 2
		const factory = hasFactory ? (`0x${userOp.initCode.slice(2, 42)}` as HexString) : undefined
		const factoryData = hasFactory ? (`0x${userOp.initCode.slice(42)}` as HexString) : undefined

		const hasPaymaster =
			userOp.paymasterAndData && userOp.paymasterAndData !== "0x" && userOp.paymasterAndData.length > 2
		const paymaster = hasPaymaster ? (`0x${userOp.paymasterAndData.slice(2, 42)}` as HexString) : undefined
		const paymasterData = hasPaymaster ? (`0x${userOp.paymasterAndData.slice(42)}` as HexString) : undefined

		// Build bundler-compatible userOp (only include defined fields)
		const userOpBundler: Record<string, unknown> = {
			sender: userOp.sender,
			nonce: toHex(userOp.nonce),
			callData: userOp.callData,
			callGasLimit: toHex(callGasLimit),
			verificationGasLimit: toHex(verificationGasLimit),
			preVerificationGas: toHex(userOp.preVerificationGas),
			maxFeePerGas: toHex(maxFeePerGas),
			maxPriorityFeePerGas: toHex(maxPriorityFeePerGas),
			signature: userOp.signature,
		}

		// Only add factory fields if present
		if (factory) {
			userOpBundler.factory = factory
			userOpBundler.factoryData = factoryData || "0x"
		}

		// Only add paymaster fields if present
		if (paymaster) {
			userOpBundler.paymaster = paymaster
			userOpBundler.paymasterData = paymasterData || "0x"
			userOpBundler.paymasterVerificationGasLimit = toHex(50_000n)
			userOpBundler.paymasterPostOpGasLimit = toHex(50_000n)
		}

		return userOpBundler
	}

	/**
	 * Sends a JSON-RPC request to the bundler endpoint.
	 *
	 * @param method - The bundler method to call
	 * @param params - Parameters array for the RPC call
	 * @returns The result from the bundler
	 * @throws Error if bundler URL not configured or bundler returns an error
	 */
	async sendBundler<T = unknown>(method: BundlerMethod, params: unknown[] = []): Promise<T> {
		if (!this.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}

		const response = await fetch(this.bundlerUrl, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
		})

		const result = await response.json()

		if (result.error) {
			throw new Error(`Bundler error: ${result.error.message || JSON.stringify(result.error)}`)
		}

		return result.result
	}

	// =========================================================================
	// ERC-7821 Batch Executor Utilities
	// =========================================================================

	/**
	 * Encodes calls into ERC-7821 execute function calldata.
	 *
	 * Format: `execute(bytes32 mode, bytes executionData)`
	 * Where executionData = abi.encode(calls) and calls = (address target, uint256 value, bytes data)[]
	 *
	 * @param calls - Array of calls to encode
	 * @returns Encoded calldata for execute function
	 */
	encodeERC7821Execute(calls: ERC7821Call[]): HexString {
		const executionData = encodeAbiParameters(
			[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
			[calls.map((call) => ({ target: call.target, value: call.value, data: call.data }))],
		) as HexString

		return encodeFunctionData({
			abi: ERC7821ABI.ABI,
			functionName: "execute",
			args: [ERC7821_BATCH_MODE, executionData],
		}) as HexString
	}

	/**
	 * Decodes ERC-7821 execute function calldata back into individual calls.
	 *
	 * @param callData - The execute function calldata to decode
	 * @returns Array of decoded calls, or null if decoding fails
	 */
	decodeERC7821Execute(callData: HexString): ERC7821Call[] | null {
		try {
			const decoded = decodeFunctionData({
				abi: ERC7821ABI.ABI,
				data: callData,
			})

			if (decoded?.functionName !== "execute" || !decoded.args || decoded.args.length < 2) {
				return null
			}

			const executionData = decoded.args[1] as HexString

			const [calls] = decodeAbiParameters(
				[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
				executionData,
			) as [ERC7821Call[]]

			return calls.map((call) => ({
				target: call.target as HexString,
				value: call.value,
				data: call.data as HexString,
			}))
		} catch {
			return null
		}
	}

	// =========================================================================
	// Private Methods - Initialization
	// =========================================================================

	/** Initializes the fee token cache for source and destination chains */
	private async initFeeTokenCache(): Promise<void> {
		const sourceFeeToken = await this.source.getFeeTokenWithDecimals()
		this.feeTokenCache.set(this.source.config.stateMachineId, sourceFeeToken)
		const destFeeToken = await this.dest.getFeeTokenWithDecimals()
		this.feeTokenCache.set(this.dest.config.stateMachineId, destFeeToken)
	}

	// =========================================================================
	// Private Methods - Bid Validation
	// =========================================================================

	/**
	 * Validates bids and sorts them by USD value (best to worst).
	 *
	 * A bid is valid if `fillOptions.outputs[i].amount >= order.output.assets[i].amount` for all i.
	 * USD value is calculated using USDC/USDT at $1 and WETH price fetched via swap.
	 */
	private async validateAndSortBids(
		bids: FillerBid[],
		order: OrderV2,
	): Promise<{ bid: FillerBid; options: FillOptionsV2; usdValue: Decimal }[]> {
		const validBids: { bid: FillerBid; options: FillOptionsV2; usdValue: Decimal }[] = []

		const destChain = hexToString(order.destination)

		const wethAddress = this.dest.configService.getWrappedNativeAssetWithDecimals(destChain).asset.toLowerCase()
		const usdcAddress = this.dest.configService.getUsdcAsset(destChain).toLowerCase()
		const usdtAddress = this.dest.configService.getUsdtAsset(destChain).toLowerCase()
		const usdcDecimals = this.dest.configService.getUsdcDecimals(destChain)
		const usdtDecimals = this.dest.configService.getUsdtDecimals(destChain)

		let wethPriceUsd = new Decimal(0)
		try {
			const oneWeth = 10n ** 18n
			const { amountOut } = await this.swap.findBestProtocolWithAmountIn(
				this.dest.client,
				wethAddress as HexString,
				usdcAddress as HexString,
				oneWeth,
				destChain,
				{ selectedProtocol: "v2" },
			)
			wethPriceUsd = new Decimal(formatUnits(amountOut, usdcDecimals))
		} catch {
			throw new Error("Failed to fetch WETH price")
		}

		for (const bid of bids) {
			try {
				const innerCalls = this.decodeERC7821Execute(bid.userOp.callData)
				if (!innerCalls || innerCalls.length === 0) {
					continue
				}

				let fillOptions: FillOptionsV2 | null = null
				for (const call of innerCalls) {
					try {
						const decoded = decodeFunctionData({
							abi: IntentGatewayV2ABI,
							data: call.data,
						})

						if (decoded?.functionName === "fillOrder" && decoded.args && decoded.args.length >= 2) {
							fillOptions = decoded.args[1] as FillOptionsV2
							break
						}
					} catch {
						continue
					}
				}

				if (!fillOptions) {
					throw new Error("Could not find fillOptions in calldata")
				}

				const bidOutputs = fillOptions.outputs
				if (!bidOutputs) {
					continue
				}

				let isValid = true
				for (let i = 0; i < order.output.assets.length; i++) {
					const requiredAmount = order.output.assets[i].amount
					const bidAmount = bidOutputs[i]?.amount ?? 0n

					if (bidAmount < requiredAmount) {
						isValid = false
						break
					}
				}

				if (!isValid) {
					continue
				}

				// Calculate USD value of bid outputs
				let totalUsdValue = new Decimal(0)
				for (let i = 0; i < bidOutputs.length; i++) {
					const tokenAddress = bytes32ToBytes20(order.output.assets[i].token).toLowerCase()
					const amount = bidOutputs[i].amount

					if (tokenAddress === usdcAddress) {
						totalUsdValue = totalUsdValue.plus(new Decimal(formatUnits(amount, usdcDecimals)))
					} else if (tokenAddress === usdtAddress) {
						totalUsdValue = totalUsdValue.plus(new Decimal(formatUnits(amount, usdtDecimals)))
					} else if (tokenAddress === wethAddress) {
						const wethAmount = new Decimal(formatUnits(amount, 18))
						totalUsdValue = totalUsdValue.plus(wethAmount.times(wethPriceUsd))
					}
				}

				validBids.push({ bid, options: fillOptions, usdValue: totalUsdValue })
			} catch {
				continue
			}
		}

		// Sort by USD value (highest first)
		validBids.sort((a, b) => b.usdValue.minus(a.usdValue).toNumber())

		return validBids
	}

	/**
	 * Simulates select + fillOrder to verify execution will succeed.
	 *
	 * No state overrides are used - the solver should already have tokens and approvals.
	 * The contract validates that outputs >= order.output.assets.
	 */
	private async simulateAndValidate(
		order: OrderV2,
		selectOptions: SelectOptions,
		fillOptions: FillOptionsV2,
		solverAddress: HexString,
		intentGatewayV2Address: HexString,
	): Promise<void> {
		const nativeOutputValue = order.output.assets
			.filter((asset) => bytes32ToBytes20(asset.token) === ADDRESS_ZERO)
			.reduce((sum, asset) => sum + asset.amount, 0n)
		const totalNativeValue = nativeOutputValue + fillOptions.nativeDispatchFee

		const selectCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "select",
			args: [selectOptions],
		}) as HexString

		const fillOrderCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "fillOrder",
			args: [transformOrderForContract(order), fillOptions],
		}) as HexString

		// Batch calls through ERC7821 execute to ensure transient storage persists
		// This simulates exactly what happens on-chain: SolverAccount.execute([select, fillOrder])
		const batchedCalldata = this.encodeERC7821Execute([
			{
				target: intentGatewayV2Address,
				value: 0n,
				data: selectCalldata,
			},
			{
				target: intentGatewayV2Address,
				value: totalNativeValue,
				data: fillOrderCalldata,
			},
		])

		try {
			await this.dest.client.call({
				account: solverAddress,
				to: solverAddress, // SolverAccount (the delegated EOA)
				data: batchedCalldata,
				value: totalNativeValue,
			})
		} catch (e: unknown) {
			throw new Error(`Simulation failed: ${e instanceof Error ? e.message : String(e)}`)
		}
	}

	// =========================================================================
	// Private Methods - Gas and Fee Calculation
	// =========================================================================

	/**
	 * Unified state override builder for gas estimation.
	 *
	 * Builds state overrides for:
	 * - EntryPoint deposit (for ERC-4337 UserOps)
	 * - Native balance
	 * - Token balances and allowances
	 * - IntentGatewayV2 params (solverSelection disabled)
	 *
	 * Returns both viem format (for estimateContractGas) and bundler format (for eth_estimateUserOperationGas).
	 *
	 * @param params - Configuration for state overrides
	 * @returns Object with both viem and bundler format state overrides
	 */
	buildStateOverride(params: {
		accountAddress: HexString
		chain: string
		outputAssets: { token: HexString; amount: bigint }[]
		spenderAddress: HexString
		intentGatewayV2Address?: HexString
		entryPointAddress?: HexString
	}): {
		viem: { address: HexString; balance?: bigint; stateDiff?: { slot: HexString; value: HexString }[] }[]
		bundler: Record<string, { balance?: string; stateDiff?: Record<string, string> }>
	} {
		const { accountAddress, chain, outputAssets, spenderAddress, intentGatewayV2Address, entryPointAddress } =
			params
		const testValue = toHex(maxUint256 / 2n, { size: 32 }) as HexString

		// Initialize both formats
		const viemOverrides: {
			address: HexString
			balance?: bigint
			stateDiff?: { slot: HexString; value: HexString }[]
		}[] = []
		const bundlerOverrides: Record<string, { balance?: string; stateDiff?: Record<string, string> }> = {}

		// 1. IntentGatewayV2 params override (disable solverSelection for simulation)
		if (intentGatewayV2Address) {
			const paramsSlot5 = pad(toHex(5n), { size: 32 }) as HexString
			const dispatcherAddress = this.dest.configService.getCalldispatcherAddress(chain)
			const newSlot5Value = ("0x" + "0".repeat(22) + "00" + dispatcherAddress.slice(2).toLowerCase()) as HexString

			viemOverrides.push({
				address: intentGatewayV2Address,
				stateDiff: [{ slot: paramsSlot5, value: newSlot5Value }],
			})
			bundlerOverrides[intentGatewayV2Address] = {
				stateDiff: { [paramsSlot5]: newSlot5Value },
			}
		}

		// 2. EntryPoint deposit override (for ERC-4337)
		// Base slot for deposit mapping is 0
		if (entryPointAddress) {
			const entryPointDepositSlot = calculateBalanceMappingLocation(0n, accountAddress, EvmLanguage.Solidity)

			viemOverrides.push({
				address: entryPointAddress,
				stateDiff: [{ slot: entryPointDepositSlot, value: testValue }],
			})
			bundlerOverrides[entryPointAddress] = {
				stateDiff: { [entryPointDepositSlot]: testValue },
			}
		}

		// 3. Native balance override for the account
		viemOverrides.push({
			address: accountAddress,
			balance: maxUint256,
		})
		bundlerOverrides[accountAddress] = {
			balance: testValue,
		}

		// 4. Token balance and allowance overrides
		for (const output of outputAssets) {
			const tokenAddress = bytes32ToBytes20(output.token)

			// Skip native token (handled by balance override above)
			if (tokenAddress === ADDRESS_ZERO) {
				continue
			}

			try {
				const viemStateDiffs: { slot: HexString; value: HexString }[] = []
				const bundlerStateDiffs: Record<string, string> = {}

				// Get balance storage slot
				const balanceData = (ERC20Method.BALANCE_OF + bytes20ToBytes32(accountAddress).slice(2)) as HexString
				const balanceSlot = getRecordedStorageSlot(chain, tokenAddress, balanceData)
				if (balanceSlot) {
					viemStateDiffs.push({ slot: balanceSlot, value: testValue })
					bundlerStateDiffs[balanceSlot] = testValue
				}

				// Get allowance storage slot
				try {
					const allowanceData = (ERC20Method.ALLOWANCE +
						bytes20ToBytes32(accountAddress).slice(2) +
						bytes20ToBytes32(spenderAddress).slice(2)) as HexString
					const allowanceSlot = getRecordedStorageSlot(chain, tokenAddress, allowanceData)
					if (allowanceSlot) {
						viemStateDiffs.push({ slot: allowanceSlot, value: testValue })
						bundlerStateDiffs[allowanceSlot] = testValue
					}
				} catch {
					// Allowance slot not found, continue without it
				}

				// Add overrides if we have at least one slot
				if (viemStateDiffs.length > 0) {
					viemOverrides.push({ address: tokenAddress, stateDiff: viemStateDiffs })
				}
				if (Object.keys(bundlerStateDiffs).length > 0) {
					bundlerOverrides[tokenAddress] = { stateDiff: bundlerStateDiffs }
				}
			} catch {
				// Balance slot not found for this token, skip
			}
		}

		return { viem: viemOverrides, bundler: bundlerOverrides }
	}

	/**
	 * Converts gas costs to the equivalent amount in the fee token (DAI).
	 * Uses USD pricing to convert between native token gas costs and fee token amounts.
	 */
	private async convertGasToFeeToken(
		gasEstimate: bigint,
		gasEstimateIn: "source" | "dest",
		evmChainID: string,
	): Promise<bigint> {
		const client = this[gasEstimateIn].client
		const gasPrice = await retryPromise(() => client.getGasPrice(), { maxRetries: 3, backoffMs: 250 })
		const gasCostInWei = gasEstimate * gasPrice
		const wethAddr = this[gasEstimateIn].configService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = this.feeTokenCache.get(evmChainID)!

		try {
			const { amountOut } = await this.swap.findBestProtocolWithAmountIn(
				this[gasEstimateIn].client,
				wethAddr,
				feeToken.address,
				gasCostInWei,
				evmChainID,
				{ selectedProtocol: "v2" },
			)
			if (amountOut === 0n) {
				console.log("Amount out not found")
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = Number.parseInt(evmChainID.split("-")[1])
			const gasCostInToken = new Decimal(formatUnits(gasCostInWei, nativeCurrency?.decimals!))
			console.log("nativeCurrency?.symbol!", nativeCurrency?.symbol!)
			const tokenPriceUsd = await fetchPrice("pol", chainId)
			const gasCostUsd = gasCostInToken.times(tokenPriceUsd)
			const feeTokenPriceUsd = new Decimal(1) // stable coin
			const gasCostInFeeToken = gasCostUsd.dividedBy(feeTokenPriceUsd)
			return parseUnits(gasCostInFeeToken.toFixed(feeToken.decimals), feeToken.decimals)
		}
	}

	/** Gets a quote for the native token cost of dispatching a post request */
	private async quoteNative(postRequest: IPostRequest, fee: bigint): Promise<bigint> {
		const dispatchPost: DispatchPost = {
			dest: toHex(postRequest.dest),
			to: postRequest.to,
			body: postRequest.body,
			timeout: postRequest.timeoutTimestamp,
			fee: fee,
			payer: postRequest.from,
		}

		const quoteNative = await this.dest.client.readContract({
			address: this.dest.configService.getIntentGatewayAddress(postRequest.dest),
			abi: IntentGateway.ABI,
			functionName: "quoteNative",
			args: [dispatchPost] as any,
		})

		return quoteNative
	}

	// =========================================================================
	// Private Methods - Cancellation Helpers
	// =========================================================================

	/** Fetches proof for the destination chain that the order hasn't been filled */
	private async *fetchDestinationProof(
		order: OrderV2,
		indexerClient: IndexerClient,
	): AsyncGenerator<CancelEvent, IProof, void> {
		let latestHeight = 0n
		let lastFailedHeight: bigint | null = null

		while (true) {
			const height = await indexerClient.queryLatestStateMachineHeight({
				statemachineId: this.dest.config.stateMachineId,
				chain: indexerClient.hyperbridge.config.stateMachineId,
			})

			latestHeight = height ?? 0n
			const shouldFetch =
				lastFailedHeight === null ? latestHeight > order.deadline : latestHeight > lastFailedHeight

			if (!shouldFetch) {
				await sleep(10000)
				continue
			}

			try {
				const intentGatewayV2Address = this.dest.configService.getIntentGatewayV2Address(
					this.dest.config.stateMachineId,
				)
				const orderId = order.id!
				const slotHash = (await this.dest.client.readContract({
					abi: IntentGatewayV2ABI,
					address: intentGatewayV2Address,
					functionName: "calculateCommitmentSlotHash",
					args: [orderId],
				})) as HexString

				const proofHex = await this.dest.queryStateProof(latestHeight, [slotHash], intentGatewayV2Address)

				const proof: IProof = {
					consensusStateId: this.dest.config.consensusStateId,
					height: latestHeight,
					proof: proofHex,
					stateMachine: this.dest.config.stateMachineId,
				}

				yield { status: "DESTINATION_FINALIZED", data: { proof } }
				return proof
			} catch (e) {
				lastFailedHeight = latestHeight
				await sleep(10000)
			}
		}
	}

	/** Submits a GET request message to Hyperbridge and confirms receipt */
	private async submitAndConfirmReceipt(
		hyperbridge: SubstrateChain,
		commitment: HexString,
		message: IGetRequestMessage,
	) {
		let storageValue = await hyperbridge.queryRequestReceipt(commitment)

		if (!storageValue) {
			console.log("No receipt found. Attempting to submit...")
			try {
				await hyperbridge.submitUnsigned(message)
			} catch {
				console.warn("Submission failed. Awaiting network confirmation...")
			}

			console.log("Waiting for network state update...")
			await sleep(30000)

			storageValue = await retryPromise(
				async () => {
					const value = await hyperbridge.queryRequestReceipt(commitment)
					if (!value) throw new Error("Receipt not found")
					return value
				},
				{ maxRetries: 10, backoffMs: 5000, logMessage: "Checking for receipt" },
			)
		}

		console.log("Hyperbridge Receipt confirmed.")
	}
}

// =============================================================================
// Standalone Utility Functions
// =============================================================================

/**
 * Fetches proof for the source chain.
 *
 * @internal
 */
async function fetchSourceProof(
	commitment: HexString,
	source: EvmChain,
	sourceStateMachine: string,
	sourceConsensusStateId: string,
	sourceHeight: bigint,
): Promise<IProof> {
	const { slot1, slot2 } = requestCommitmentKey(commitment)
	const proofHex = await source.queryStateProof(sourceHeight, [slot1, slot2])

	return {
		height: sourceHeight,
		stateMachine: sourceStateMachine,
		consensusStateId: sourceConsensusStateId,
		proof: proofHex,
	}
}

/**
 * Transforms an OrderV2 (SDK type) to the Order struct expected by the contract.
 *
 * Removes SDK-specific fields (`id`, `transactionHash`) and converts
 * source/destination to hex if not already.
 *
 * @param order - The SDK order to transform
 * @returns Contract-compatible order struct
 */
export function transformOrderForContract(order: OrderV2): Omit<OrderV2, "id" | "transactionHash"> {
	const { id: _id, transactionHash: _txHash, ...contractOrder } = order
	return {
		...contractOrder,
		source: order.source.startsWith("0x") ? order.source : toHex(order.source),
		destination: order.destination.startsWith("0x") ? order.destination : toHex(order.destination),
	}
}

// =============================================================================
// Legacy Exports (for backward compatibility)
// =============================================================================

/** @deprecated Use `IntentGatewayV2.SELECT_SOLVER_TYPEHASH` instead */
export const SELECT_SOLVER_TYPEHASH = IntentGatewayV2.SELECT_SOLVER_TYPEHASH

/** @deprecated Use `IntentGatewayV2.PACKED_USEROP_TYPEHASH` instead */
export const PACKED_USEROP_TYPEHASH = IntentGatewayV2.PACKED_USEROP_TYPEHASH

/** @deprecated Use `IntentGatewayV2.DOMAIN_TYPEHASH` instead */
export const DOMAIN_TYPEHASH = IntentGatewayV2.DOMAIN_TYPEHASH
