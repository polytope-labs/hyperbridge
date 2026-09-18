import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type {
	Bid,
	ERC7821Call,
	FillOptions,
	FillerBid,
	HexString,
	Order,
	PackedUserOperation,
	SelectBidResult,
	SelectOptions,
	TokenInfo,
} from "@/types"
import { ADDRESS_ZERO, bytes20ToBytes32, bytes32ToBytes20, normalizeStateMachineId, retryPromise } from "@/utils"
import type Decimal from "decimal.js"
import { concat, encodeFunctionData, parseEventLogs, toEventSelector } from "viem"
import type { Hex } from "viem"
import { BundlerRpcError, CryptoUtils } from "./CryptoUtils"
import type { IntentGatewayContext } from "./types"
import { BundlerMethod } from "./types"

const ENTRY_POINT_EVENT_ABI = [
	{ type: "event", name: "BeforeExecution", inputs: [] },
	{
		type: "event",
		name: "UserOperationEvent",
		inputs: [
			{ name: "userOpHash", type: "bytes32", indexed: true },
			{ name: "sender", type: "address", indexed: true },
			{ name: "paymaster", type: "address", indexed: true },
			{ name: "nonce", type: "uint256", indexed: false },
			{ name: "success", type: "bool", indexed: false },
			{ name: "actualGasCost", type: "uint256", indexed: false },
			{ name: "actualGasUsed", type: "uint256", indexed: false },
		],
	},
] as const

/** Submission was accepted, but inclusion/fill outcome could not be established safely. */
export class BidExecutionPendingError extends Error {
	constructor(
		readonly userOpHash: HexString,
		message: string,
	) {
		super(message)
		this.name = "BidExecutionPendingError"
	}
}

/** The operation was rejected on its first send, or failed with verified chain evidence. */
export class BidExecutionRejectedError extends Error {}

const REJECTION_CODES = new Set([-32602, -32500, -32501, -32502, -32503, -32504, -32505, -32507, -32508])
function isFirstSendRejection(error: unknown): boolean {
	return (
		error instanceof BundlerRpcError &&
		REJECTION_CODES.has(error.code) &&
		!/already\s+(known|seen)|AA25|nonce/i.test(error.message)
	)
}

/** Constructor parameters for {@link BidImpl}. */
export interface BidParams {
	ctx: IntentGatewayContext
	crypto: CryptoUtils
	order: Order
	fillerBid: FillerBid
	fillOptions: FillOptions
	/** Prices the bid outputs in USD; bound by {@link BidManager} to the destination chain. */
	priceOutputs: (outputs: TokenInfo[]) => Promise<Decimal | null>
	/** Optional session-key override; looked up from storage by `order.session` if omitted. */
	sessionPrivateKey?: HexString
}

/**
 * Concrete {@link Bid} implementation wrapping a single decoded {@link FillerBid}.
 *
 * Holds everything needed to simulate and execute one solver bid: the shared
 * IntentsV2 context, crypto utilities, the order, the solver's UserOperation, and
 * the decoded fill options. The session-key signature over the `SelectSolver`
 * message is resolved lazily and cached, so it is signed at most once whether the
 * consumer calls {@link simulate}, {@link execute}, or both.
 */
export class BidImpl implements Bid {
	readonly solverAddress: HexString
	readonly outputs: TokenInfo[]
	readonly inputs: TokenInfo[]
	readonly relayerFee: bigint
	readonly nativeDispatchFee: bigint
	readonly userOp: PackedUserOperation

	private readonly ctx: IntentGatewayContext
	private readonly crypto: CryptoUtils
	private readonly order: Order
	private readonly fillOptions: FillOptions
	private readonly priceOutputs: (outputs: TokenInfo[]) => Promise<Decimal | null>
	private readonly sessionPrivateKey?: HexString

	private readonly intentGatewayV2Address: HexString
	private readonly domainSeparator: HexString

	/** Cached session-key signature over the `SelectSolver` message. */
	private cachedSignature?: HexString
	private broadcastAttempted = false

	constructor(params: BidParams) {
		this.ctx = params.ctx
		this.crypto = params.crypto
		this.order = params.order
		this.fillOptions = params.fillOptions
		this.priceOutputs = params.priceOutputs
		this.sessionPrivateKey = params.sessionPrivateKey

		this.solverAddress = params.fillerBid.userOp.sender
		this.outputs = params.fillOptions.outputs
		this.inputs = params.fillOptions.inputs ?? []
		this.relayerFee = params.fillOptions.relayerFee
		this.nativeDispatchFee = params.fillOptions.nativeDispatchFee
		this.userOp = params.fillerBid.userOp

		this.intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayAddress(
			normalizeStateMachineId(this.order.destination),
		)
		this.domainSeparator = CryptoUtils.getDomainSeparator(
			"IntentGateway",
			"2",
			this.chainId(),
			this.intentGatewayV2Address,
		)
	}

	/** Resolves the destination chain id from the client or the state-machine id. */
	private chainId(): bigint {
		return BigInt(
			this.ctx.dest.client.chain?.id ?? Number.parseInt(this.ctx.dest.config.stateMachineId.split("-")[1]),
		)
	}

	/**
	 * Resolves the session key, signs the `SelectSolver` message for this bid's
	 * solver, and caches the signature. Signs at most once per bid.
	 *
	 * @throws If the session key is missing or signing fails.
	 */
	private async signSelection(): Promise<HexString> {
		if (this.cachedSignature) return this.cachedSignature

		const commitment = this.order.id as HexString
		const sessionKeyAddress = this.order.session as HexString

		const sessionKeyData = this.sessionPrivateKey
			? { privateKey: this.sessionPrivateKey }
			: await this.ctx.sessionKeyStorage.getSessionKeyByAddress(sessionKeyAddress)
		if (!sessionKeyData) {
			throw new Error(`SessionKey not found for commitment: ${commitment}`)
		}

		const signature = await CryptoUtils.signSolverSelection(
			commitment,
			this.solverAddress,
			this.domainSeparator,
			sessionKeyData.privateKey,
		)
		if (!signature) {
			throw new Error("Failed to sign solver selection")
		}

		this.cachedSignature = signature
		return signature
	}

	/**
	 * Simulates this bid on-chain by batching the `select` and `fillOrder` calls
	 * via `eth_call` from the solver's account, using the IntentGatewayV2 ERC-7821
	 * batch-execute pattern.
	 *
	 * The native value forwarded to the simulation is the sum of any native-token
	 * (`address(0)`) output amounts plus the Hyperbridge dispatch fee.
	 *
	 * @throws If the `eth_call` simulation reverts or errors.
	 */
	async simulate(): Promise<void> {
		const signature = await this.signSelection()

		const selectOptions: SelectOptions = {
			commitment: this.order.id as HexString,
			solver: this.solverAddress,
			signature,
		}

		// Compute the native ETH the fillOrder call requires:
		// native token outputs (address(0)) + Hyperbridge dispatch fee
		const nativeOutputs = this.fillOptions.outputs.reduce(
			(acc, o) => (bytes32ToBytes20(o.token) === ADDRESS_ZERO ? acc + o.amount : acc),
			0n,
		)
		const simulationValue = nativeOutputs + this.fillOptions.nativeDispatchFee

		const selectCalldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "select",
			args: [selectOptions],
		}) as HexString

		const calls: ERC7821Call[] = [
			{ target: this.intentGatewayV2Address, value: 0n, data: selectCalldata },
			{ target: this.solverAddress, value: simulationValue, data: this.userOp.callData },
		]
		const batchedCalldata = this.crypto.encodeERC7821Execute(calls)

		try {
			await this.ctx.dest.client.call({
				account: this.solverAddress,
				to: this.solverAddress,
				data: batchedCalldata,
				value: simulationValue,
			})
		} catch (e: unknown) {
			throw new Error(`Simulation failed: ${e instanceof Error ? e.message : String(e)}`)
		}
	}

	/**
	 * Signs the `SelectSolver` message with the session key, appends it to the
	 * solver's existing UserOp signature, and submits the UserOperation to the
	 * bundler. Waits for the receipt and reads `OrderFilled` / `PartialFill`
	 * logs to determine fill status.
	 *
	 * @returns A {@link SelectBidResult} with the submitted UserOperation, its hash,
	 *   the solver address, transaction hash, and fill status.
	 * @throws If the bundler is not configured, the session key is missing, or the
	 *   bundler rejects the UserOperation.
	 */
	async execute(
		onSubmitted?: (submission: SelectBidResult) => Promise<void>,
		onTerminal?: (submission: SelectBidResult) => Promise<void>,
	): Promise<SelectBidResult> {
		const commitment = this.order.id as HexString

		if (!this.ctx.bundlerUrl) {
			throw new Error("Bundler URL not configured")
		}

		const sessionSignature = await this.signSelection()

		const finalSignature = concat([this.userOp.signature as Hex, sessionSignature as Hex]) as HexString
		const signedUserOp: PackedUserOperation = {
			...this.userOp,
			signature: finalSignature,
		}

		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(
			normalizeStateMachineId(this.order.destination),
		)

		// The EntryPoint hash excludes the signature, so it is deterministic before the RPC call.
		// Persist the complete signed operation before sending: an HTTP timeout can happen after the bundler accepted the op,
		// and treating that timeout as a safe rejection could execute a second solver bid.
		const userOpHash = CryptoUtils.computeUserOpHash(signedUserOp, entryPointAddress, this.chainId())
		const accepted: SelectBidResult = {
			userOp: signedUserOp,
			userOpHash,
			solverAddress: this.solverAddress,
			commitment,
		}
		try {
			await onSubmitted?.(accepted)
		} catch (err) {
			throw new BidExecutionPendingError(
				userOpHash,
				`Bid submission attempt could not be recorded durably: ${err instanceof Error ? err.message : String(err)}`,
			)
		}
		try {
			const replay = this.broadcastAttempted
			this.broadcastAttempted = true
			await BidImpl.broadcast(this.crypto, accepted, entryPointAddress, replay)
			const receipt = await retryPromise(
				async () => {
					const result = await BidImpl.receipt(this.crypto, userOpHash)
					if (!result) throw new Error("Receipt not available yet")
					return result
				},
				{ maxRetries: 5, backoffMs: 2000, logMessage: "Fetching user operation receipt" },
			)
			const result = await BidImpl.verifyReceipt(this.ctx, this.order, accepted, receipt)
			await this.retire(result, onTerminal)
			return result
		} catch (error) {
			if (error instanceof BidExecutionRejectedError) {
				await this.retire(accepted, onTerminal)
				throw error
			}
			if (error instanceof BidExecutionPendingError) throw error
			throw new BidExecutionPendingError(
				userOpHash,
				`Bid submission outcome is uncertain: ${error instanceof Error ? error.message : String(error)}`,
			)
		}
	}

	private async retire(
		submission: SelectBidResult,
		onTerminal?: (submission: SelectBidResult) => Promise<void>,
	): Promise<void> {
		try {
			await onTerminal?.(submission)
		} catch (error) {
			throw new BidExecutionPendingError(
				submission.userOpHash,
				`Could not retire submission durably: ${error instanceof Error ? error.message : String(error)}`,
			)
		}
	}

	/** Replay always preserves the stored signature and never authorizes fallback on an RPC rejection. */
	static async broadcast(
		crypto: CryptoUtils,
		submission: SelectBidResult,
		entryPoint: HexString,
		replay: boolean,
	): Promise<void> {
		try {
			const hash = await crypto.sendBundler<HexString>(BundlerMethod.ETH_SEND_USER_OPERATION, [
				CryptoUtils.prepareBundlerCall(submission.userOp),
				entryPoint,
			])
			if (typeof hash !== "string" || hash.toLowerCase() !== submission.userOpHash.toLowerCase())
				throw new Error("Bundler returned an unexpected operation hash")
		} catch (error) {
			if (!replay && isFirstSendRejection(error)) throw new BidExecutionRejectedError((error as Error).message)
			throw new BidExecutionPendingError(
				submission.userOpHash,
				`Bid send outcome is uncertain: ${error instanceof Error ? error.message : String(error)}`,
			)
		}
	}

	static async receipt(
		crypto: CryptoUtils,
		hash: HexString,
	): Promise<{ receipt: { transactionHash: HexString } } | null> {
		const result = await crypto.sendBundler<{ receipt: { transactionHash: HexString } } | null>(
			BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT,
			[hash],
		)
		if (result === null) return null
		if (!/^0x[0-9a-fA-F]{64}$/.test(result?.receipt?.transactionHash))
			throw new BidExecutionPendingError(hash, "Malformed UserOperation receipt")
		return result
	}

	/** Shared by normal submission and recovery; bundler success flags are not chain evidence. */
	static async verifyReceipt(
		ctx: IntentGatewayContext,
		order: Order,
		accepted: SelectBidResult,
		receipt: { receipt: { transactionHash: HexString } },
	): Promise<SelectBidResult> {
		const { userOpHash, userOp: signedUserOp, commitment } = accepted
		const entryPointAddress = ctx.dest.configService.getEntryPointV08Address(
			normalizeStateMachineId(order.destination),
		)
		const intentGatewayV2Address = ctx.dest.configService.getIntentGatewayAddress(
			normalizeStateMachineId(order.destination),
		)

		let txnHash: HexString | undefined
		let fillStatus: "full" | "partial" | undefined
		let filledAssets: TokenInfo[] | undefined

		txnHash = receipt.receipt.transactionHash

		let chainReceipt: Awaited<ReturnType<typeof ctx.dest.client.waitForTransactionReceipt>>
		try {
			chainReceipt = await ctx.dest.client.waitForTransactionReceipt({
				hash: txnHash,
				confirmations: 1,
			})
		} catch (err) {
			throw new BidExecutionPendingError(
				userOpHash,
				`Bid submission outcome is uncertain: ${err instanceof Error ? err.message : String(err)}`,
			)
		}
		if (chainReceipt.status === "reverted") {
			throw new BidExecutionPendingError(
				userOpHash,
				`Bundle reverted without proof of operation inclusion in ${txnHash}`,
			)
		}
		let userOpSucceeded: boolean | undefined
		let executionLogs: typeof chainReceipt.logs | undefined
		try {
			const boundaryTopics = ENTRY_POINT_EVENT_ABI.map((event) => toEventSelector(event))
			const boundaryLogs = chainReceipt.logs.filter(
				(log) =>
					log.address.toLowerCase() === entryPointAddress.toLowerCase() &&
					boundaryTopics.some((topic) => topic === log.topics[0]),
			)
			const boundaries = parseEventLogs({
				abi: ENTRY_POINT_EVENT_ABI,
				logs: boundaryLogs,
				eventName: ["BeforeExecution", "UserOperationEvent"],
			})
			// Skipping a malformed boundary could attribute a previous operation's fill.
			if (boundaries.length !== boundaryLogs.length) throw new Error("Malformed EntryPoint operation boundary")
			const matching = boundaries.filter(
				(event) =>
					event.eventName === "UserOperationEvent" &&
					event.args.userOpHash.toLowerCase() === userOpHash.toLowerCase() &&
					event.args.sender.toLowerCase() === signedUserOp.sender.toLowerCase() &&
					event.args.nonce === signedUserOp.nonce,
			)
			const matched = matching.length === 1 ? matching[0] : undefined
			if (matched?.eventName === "UserOperationEvent") {
				userOpSucceeded = matched.args.success
				// v0.8 emits operation logs before UserOperationEvent. The previous
				// operation event (or BeforeExecution) separates this fill from the bundle.
				const previous = boundaries[boundaries.indexOf(matched) - 1]
				const start = previous?.logIndex
				const end = matched.logIndex
				const ordered = chainReceipt.logs.every(
					(log, index, logs) =>
						log.logIndex !== null &&
						Number.isSafeInteger(log.logIndex) &&
						log.logIndex >= 0 &&
						(index === 0 || log.logIndex > logs[index - 1].logIndex!),
				)
				if (ordered && start != null && end != null && start < end) {
					executionLogs = chainReceipt.logs.filter((log) => log.logIndex! > start && log.logIndex! < end)
				}
			}
		} catch {
			// Missing or malformed operation evidence remains pending.
		}
		if (userOpSucceeded === undefined)
			throw new BidExecutionPendingError(userOpHash, "No unique matching EntryPoint UserOperationEvent")
		if (userOpSucceeded === false) {
			throw new BidExecutionRejectedError(`UserOperation failed in confirmed transaction ${txnHash}`)
		}
		if (!executionLogs) {
			throw new BidExecutionPendingError(userOpHash, "Missing or ambiguous EntryPoint operation boundaries")
		}

		try {
			const events = parseEventLogs({
				abi: IntentGatewayV2ABI,
				logs: executionLogs,
				eventName: ["OrderFilled", "PartialFill"],
			})
			const matchingFills = events.filter((e) => {
				if (e.address.toLowerCase() !== intentGatewayV2Address.toLowerCase()) return false
				return (
					e.args.commitment.toLowerCase() === commitment.toLowerCase() &&
					e.args.filler.toLowerCase() === accepted.solverAddress.toLowerCase()
				)
			})
			if (matchingFills.length === 0) throw new Error("No fill event found")
			if (matchingFills.some((event) => event.eventName === "OrderFilled")) {
				fillStatus = "full"
			} else {
				fillStatus = "partial"
				filledAssets = order.output.assets.map(({ token }) => ({ token: bytes20ToBytes32(token), amount: 0n }))
				// One operation can fill repeatedly; each event credits its positional legs.
				for (const { args } of matchingFills) {
					if (args.outputs.length !== filledAssets.length)
						throw new Error("Fill output length does not match order")
					for (const [index, output] of args.outputs.entries()) {
						const leg = filledAssets[index]
						if (output.token.toLowerCase() !== leg.token.toLowerCase()) {
							throw new Error("Fill output token does not match order leg")
						}
						leg.amount += output.amount
					}
				}
			}
		} catch (err) {
			throw new BidExecutionPendingError(
				userOpHash,
				`Bid submission outcome is uncertain: ${err instanceof Error ? err.message : String(err)}`,
			)
		}

		return {
			...accepted,
			txnHash,
			fillStatus,
			filledAssets,
		}
	}

	/**
	 * Prices this bid's outputs in USD using the destination chain's DEX-quote
	 * helpers. Returns `null` when any output token cannot be priced.
	 */
	async outputUsdValue(): Promise<Decimal | null> {
		return this.priceOutputs(this.outputs)
	}
}
