import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { ENTRY_POINT_V09 } from "@/configs/chain"
import type {
	Bid,
	FillOptions,
	FillerBid,
	HexString,
	Order,
	PackedUserOperation,
	SelectBidResult,
	TokenInfo,
} from "@/types"
import { normalizeStateMachineId, retryPromise } from "@/utils"
import type Decimal from "decimal.js"
import { concat, decodeEventLog, parseEventLogs, recoverAddress, slice, toEventSelector } from "viem"
import type { Hex, Log } from "viem"
import { retryTransientBundlerErrors } from "./bundlerRetry"
import { CryptoUtils } from "./CryptoUtils"
import { readSelectionFormat, selectionOffStateDiff } from "./selection"
import type { IntentGatewayContext } from "./types"
import { BundlerMethod } from "./types"

/** The EntryPoint events that delimit one operation's logs inside a bundle transaction. */
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

/**
 * The logs `userOpHash` emitted inside a bundle transaction. A bundle can carry other operations,
 * including other fills of the same order, so fill events are only read from this range. The
 * EntryPoint emits an operation's logs and then its `UserOperationEvent`; the range opens at the
 * previous such event, or at `BeforeExecution` for the first operation.
 *
 * @throws If the operation is not in the transaction, or it reverted.
 */
export function userOperationLogs(logs: Log[], entryPoint: HexString, userOpHash: HexString): Log[] {
	const [beforeExecution, userOperationEvent] = ENTRY_POINT_EVENT_ABI.map((event) => toEventSelector(event))
	const isBoundary = (log: Log) =>
		log.address.toLowerCase() === entryPoint.toLowerCase() &&
		(log.topics[0] === beforeExecution || log.topics[0] === userOperationEvent)

	const end = logs.findIndex(
		(log) =>
			isBoundary(log) &&
			log.topics[0] === userOperationEvent &&
			log.topics[1]?.toLowerCase() === userOpHash.toLowerCase(),
	)
	if (end === -1) throw new Error(`UserOperation ${userOpHash} is not in the transaction`)

	const { args } = decodeEventLog({
		abi: ENTRY_POINT_EVENT_ABI,
		eventName: "UserOperationEvent",
		data: logs[end].data,
		topics: logs[end].topics,
	})
	if (!args.success) throw new Error(`UserOperation ${userOpHash} reverted`)

	let start = end - 1
	while (start >= 0 && !isBoundary(logs[start])) start--
	return logs.slice(start + 1, end)
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
 * the decoded fill options.
 *
 * Bids run on EntryPoint v0.9 and are selected by their userOpHash. A bid its solver
 * did not sign for v0.9, or one on a gateway that does not select by userOpHash, fails
 * {@link simulate} and {@link execute}.
 *
 * The userOpHash and the session-key signature over the `SelectSolver` message are
 * resolved lazily and cached, so each is worked out at most once whether the consumer
 * calls {@link simulate}, {@link execute}, or both.
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
	private readonly priceOutputs: (outputs: TokenInfo[]) => Promise<Decimal | null>
	private readonly sessionPrivateKey?: HexString

	private readonly intentGatewayV2Address: HexString
	private readonly domainSeparator: HexString

	/** Cached session-key signature over the `SelectSolver` message. */
	private cachedSignature?: HexString

	/** Cached result of {@link selectableUserOpHash}. */
	private cachedUserOpHash?: HexString

	constructor(params: BidParams) {
		this.ctx = params.ctx
		this.crypto = params.crypto
		this.order = params.order
		this.priceOutputs = params.priceOutputs
		this.sessionPrivateKey = params.sessionPrivateKey

		this.solverAddress = params.fillerBid.userOp.sender
		this.outputs = params.fillOptions.outputs
		this.inputs = params.fillOptions.inputs
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
	 * This bid's userOpHash on EntryPoint v0.9, once it is checked that the solver
	 * signed the bid for v0.9 and that the destination gateway selects by userOpHash.
	 * Caches the result.
	 *
	 * @throws If the solver's signature does not recover under v0.9, the gateway
	 *   selects by address, or its selection format cannot be read.
	 */
	private async selectableUserOpHash(): Promise<HexString> {
		if (this.cachedUserOpHash) return this.cachedUserOpHash

		const chainId = this.chainId()
		const userOpHash = CryptoUtils.computeUserOpHash(this.userOp, ENTRY_POINT_V09, chainId)
		if (!(await this.signedBySolver(userOpHash))) {
			throw new Error(`Bid from solver=${this.solverAddress} is not signed for EntryPoint v0.9`)
		}

		const format = await readSelectionFormat(this.ctx.dest.client, chainId, this.intentGatewayV2Address)
		if (format !== "userOpHash") {
			throw new Error(
				`Bid from solver=${this.solverAddress} cannot be selected: the destination gateway selects by ${format}`,
			)
		}

		this.cachedUserOpHash = userOpHash
		return userOpHash
	}

	/**
	 * Whether the solver's signature in the bid, `signature[32:97]` after the order
	 * commitment, recovers to the solver over `userOpHash`.
	 */
	private async signedBySolver(userOpHash: HexString): Promise<boolean> {
		try {
			const recovered = await recoverAddress({
				hash: userOpHash,
				signature: slice(this.userOp.signature, 32, 97),
			})
			return recovered.toLowerCase() === this.solverAddress.toLowerCase()
		} catch {
			return false
		}
	}

	/**
	 * Resolves the session key, signs the `SelectSolver` message over this bid's
	 * userOpHash, and caches the signature. Signs at most once per bid.
	 *
	 * @throws If the bid cannot be selected, or the session key is missing.
	 */
	private async signSelection(): Promise<HexString> {
		if (this.cachedSignature) return this.cachedSignature

		const userOpHash = await this.selectableUserOpHash()
		const commitment = this.order.id as HexString
		const sessionKeyAddress = this.order.session as HexString

		const sessionKeyData = this.sessionPrivateKey
			? { privateKey: this.sessionPrivateKey }
			: await this.ctx.sessionKeyStorage.getSessionKeyByAddress(sessionKeyAddress)
		if (!sessionKeyData) {
			throw new Error(`SessionKey not found for commitment: ${commitment}`)
		}

		this.cachedSignature = await CryptoUtils.signUserOpHashSelection(
			commitment,
			userOpHash,
			this.domainSeparator,
			sessionKeyData.privateKey,
		)
		return this.cachedSignature
	}

	/**
	 * Simulates this bid with the bundler's `eth_estimateUserOperationGas` on
	 * EntryPoint v0.9. The solver's UserOperation is sent as signed, so a spent
	 * nonce, a failing paymaster or a fill that reverts at the signed
	 * `callGasLimit` fails here as it would fail submission.
	 *
	 * The gateway's solver selection is turned off with {@link selectionOffStateDiff}
	 * and no selection is attached: the bundler simulates the operation with its
	 * `preVerificationGas` and fees changed, so the userOpHash there is not the one a
	 * selection signs. The account's own checks therefore run only on submission.
	 *
	 * @throws If the bid cannot be selected, or the bundler rejects the operation. A
	 *   transient bundler failure is retried first.
	 */
	async simulate(): Promise<void> {
		await this.selectableUserOpHash()

		try {
			const { slot, value } = await selectionOffStateDiff(this.ctx.dest.client, this.intentGatewayV2Address)
			await retryTransientBundlerErrors(() =>
				this.crypto.sendBundler(BundlerMethod.ETH_ESTIMATE_USER_OPERATION_GAS, [
					CryptoUtils.prepareBundlerCall(this.userOp),
					ENTRY_POINT_V09,
					{ [this.intentGatewayV2Address]: { stateDiff: { [slot]: value } } },
				]),
			)
		} catch (e: unknown) {
			throw new Error(`Simulation failed: ${e instanceof Error ? e.message : String(e)}`, { cause: e })
		}
	}

	/**
	 * Signs the `SelectSolver` message with the session key, appends it to the
	 * solver's existing UserOp signature, and submits the UserOperation to the
	 * bundler on EntryPoint v0.9. Waits for the receipt and reads `OrderFilled` /
	 * `PartialFill` logs to determine fill status.
	 *
	 * @returns A {@link SelectBidResult} with the submitted UserOperation, its hash,
	 *   the solver address, transaction hash, and fill status.
	 * @throws If the bundler is not configured, the bid cannot be selected, the
	 *   session key is missing, or the bundler rejects the UserOperation.
	 */
	async execute(): Promise<SelectBidResult> {
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

		const userOpHash = await this.crypto.sendBundler<HexString>(BundlerMethod.ETH_SEND_USER_OPERATION, [
			CryptoUtils.prepareBundlerCall(signedUserOp),
			ENTRY_POINT_V09,
		])

		let txnHash: HexString | undefined
		let fillStatus: "full" | "partial" | undefined
		let filledAssets: TokenInfo[] | undefined
		try {
			const receipt = await retryPromise(
				async () => {
					const result = await this.crypto.sendBundler<{
						receipt: { transactionHash: HexString }
					} | null>(BundlerMethod.ETH_GET_USER_OPERATION_RECEIPT, [userOpHash])
					if (!result?.receipt?.transactionHash) {
						throw new Error("Receipt not available yet")
					}
					return result
				},
				{ maxRetries: 5, backoffMs: 2000, logMessage: "Fetching user operation receipt" },
			)
			txnHash = receipt.receipt.transactionHash

			const chainReceipt = await this.ctx.dest.client.waitForTransactionReceipt({
				hash: txnHash,
				confirmations: 1,
			})
			const gateway = this.ctx.dest.configService.getIntentGatewayAddress(
				normalizeStateMachineId(this.order.destination),
			)
			const fills = parseEventLogs({
				abi: IntentGatewayV2ABI,
				logs: userOperationLogs(chainReceipt.logs, ENTRY_POINT_V09, userOpHash),
				eventName: ["OrderFilled", "PartialFill"],
			}).filter(
				(event) =>
					event.address.toLowerCase() === gateway.toLowerCase() &&
					event.args.commitment.toLowerCase() === commitment.toLowerCase(),
			)

			if (fills.some((event) => event.eventName === "OrderFilled")) {
				fillStatus = "full"
			} else if (fills.length > 0) {
				fillStatus = "partial"
				// An operation can call `fillOrder` more than once; each event credits its legs by index.
				filledAssets = fills[0].args.outputs.map(({ token }, index) => ({
					token,
					amount: fills.reduce((sum, event) => sum + (event.args.outputs[index]?.amount ?? 0n), 0n),
				}))
			}
		} catch (err) {
			throw new Error(`Failed to execute bid: ${err instanceof Error ? err.message : String(err)}`)
		}

		return {
			userOp: signedUserOp,
			userOpHash,
			solverAddress: this.solverAddress,
			commitment,
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
