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
import { ADDRESS_ZERO, bytes32ToBytes20, normalizeStateMachineId, retryPromise } from "@/utils"
import type Decimal from "decimal.js"
import { concat, decodeEventLog, encodeFunctionData, parseEventLogs, toEventSelector } from "viem"
import type { Hex, Log } from "viem"
import { CryptoUtils } from "./CryptoUtils"
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

	constructor(params: BidParams) {
		this.ctx = params.ctx
		this.crypto = params.crypto
		this.order = params.order
		this.fillOptions = params.fillOptions
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

		const entryPointAddress = this.ctx.dest.configService.getEntryPointV08Address(
			normalizeStateMachineId(this.order.destination),
		)

		const userOpHash = await this.crypto.sendBundler<HexString>(BundlerMethod.ETH_SEND_USER_OPERATION, [
			CryptoUtils.prepareBundlerCall(signedUserOp),
			entryPointAddress,
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
				logs: userOperationLogs(chainReceipt.logs, entryPointAddress, userOpHash),
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
