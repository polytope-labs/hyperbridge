import { createSessionKeyStorage, createCancellationStorage, createUsedUserOpsStorage } from "@/storage"
import { Swap } from "@/utils/swap"
import type { TransactionReceipt } from "viem"
import type { Order, HexString, CancelQuote } from "@/types"
import type {
	PackedUserOperation,
	SubmitBidOptions,
	EstimateFillOrderParams,
	FillOrderEstimate,
	IntentOrderStatusUpdate,
	SelectBidResult,
	FillerBid,
} from "@/types"
import type { ResumeIntentOrderOptions } from "@/types"
import type { IEvmChain } from "@/chain"
import type { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { IndexerClient } from "@/client"
import type { IntentGatewayContext } from "./types"
import type { CancelEvent } from "./types"
import { CryptoUtils } from "./CryptoUtils"
import { OrderPlacer } from "./OrderPlacer"
import { OrderExecutor } from "./OrderExecutor"
import { OrderCanceller } from "./OrderCanceller"
import { BidManager } from "./BidManager"
import { GasEstimator } from "./GasEstimator"
import { OrderStatusChecker } from "./OrderStatusChecker"
import type { ERC7821Call } from "@/types"
import { DEFAULT_GRAFFITI, ADDRESS_ZERO } from "@/utils"

/**
 * High-level facade for the IntentGatewayV2 protocol.
 *
 * `IntentGateway` orchestrates the complete lifecycle of an intent-based
 * cross-chain swap:
 * - **Order placement** — encodes and yields `placeOrder` calldata; caller
 *   signs and submits the transaction.
 * - **Order execution** — polls the Hyperbridge coprocessor for solver bids,
 *   selects the best bid, and submits an ERC-4337 UserOperation via a bundler.
 * - **Order cancellation** — guides the caller through the source- or
 *   destination-initiated cancellation flow, including ISMP proof fetching and
 *   Hyperbridge relay.
 * - **Status checks** — reads on-chain storage to determine whether an order
 *   has been filled or refunded.
 *
 * Internally delegates to specialised sub-modules: {@link OrderPlacer},
 * {@link OrderExecutor}, {@link OrderCanceller}, {@link BidManager},
 * {@link GasEstimator}, {@link OrderStatusChecker}, and {@link CryptoUtils}.
 *
 * Use `IntentGateway.create()` to obtain an initialised instance.
 */
export class IntentGateway {
	/** EVM chain on which orders are placed and escrowed. */
	readonly source: IEvmChain
	/** EVM chain on which solvers fill orders and deliver outputs. */
	readonly dest: IEvmChain
	/** Optional Hyperbridge coprocessor client for bid fetching and UserOp submission. */
	readonly intentsCoprocessor?: IntentsCoprocessor
	/** Optional ERC-4337 bundler URL for gas estimation and UserOp broadcasting. */
	readonly bundlerUrl?: string

	/** Shared context object passed to all sub-modules. */
	private readonly ctx: IntentGatewayContext
	/** Crypto and encoding utilities (EIP-712, gas packing, bundler calls). */
	private readonly _crypto: CryptoUtils
	/** Handles `placeOrder` calldata generation and session-key management. */
	private readonly orderPlacer: OrderPlacer
	/** Drives the bid-polling and UserOp-submission loop after order placement. */
	private readonly orderExecutor: OrderExecutor
	/** Manages source- and destination-initiated order cancellation flows. */
	private readonly orderCanceller: OrderCanceller
	/** Reads fill and refund status from on-chain storage. */
	private readonly orderStatusChecker: OrderStatusChecker
	/** Validates, sorts, simulates, and submits solver bids. */
	private readonly bidManager: BidManager
	/** Estimates gas costs for filling an order and converts them to fee-token amounts. */
	private readonly gasEstimator: GasEstimator

	/**
	 * Private constructor — use {@link IntentGateway.create} instead.
	 *
	 * Initialises all sub-modules and the shared context, including storage
	 * adapters, fee-token and solver-code caches, and the DEX-quote utility.
	 *
	 * @param source - Source chain client.
	 * @param dest - Destination chain client.
	 * @param intentsCoprocessor - Optional coprocessor for bid fetching.
	 */
	private constructor(source: IEvmChain, dest: IEvmChain, intentsCoprocessor?: IntentsCoprocessor) {
		this.source = source
		this.dest = dest
		this.intentsCoprocessor = intentsCoprocessor
		this.bundlerUrl = dest.bundlerUrl

		const sessionKeyStorage = createSessionKeyStorage()
		const cancellationStorage = createCancellationStorage()
		const usedUserOpsStorage = createUsedUserOpsStorage()
		const swap = new Swap()
		const feeTokenCache = new Map<string, { address: HexString; decimals: number; cachedAt: number }>()
		const solverCodeCache = new Map<string, string>()

		this.ctx = {
			source,
			dest,
			intentsCoprocessor,
			bundlerUrl: this.bundlerUrl,
			feeTokenCache,
			solverCodeCache,
			sessionKeyStorage,
			cancellationStorage,
			usedUserOpsStorage,
			swap,
		}

		const crypto = new CryptoUtils(this.ctx)
		const bidManager = new BidManager(this.ctx, crypto)
		const gasEstimator = new GasEstimator(this.ctx, crypto)

		this.orderPlacer = new OrderPlacer(this.ctx)
		this.orderExecutor = new OrderExecutor(this.ctx, bidManager, crypto)
		this.orderCanceller = new OrderCanceller(this.ctx)
		this.orderStatusChecker = new OrderStatusChecker(this.ctx)
		this.bidManager = bidManager
		this.gasEstimator = gasEstimator
		this._crypto = crypto
	}

	/**
	 * Creates an initialized IntentGateway instance.
	 *
	 * Fetches the fee tokens for both chains and optionally caches the solver
	 * account bytecode before returning, so the instance is ready for use
	 * without additional warm-up calls.
	 *
	 * The ERC-4337 bundler URL is read from `dest.bundlerUrl`, set when constructing
	 * the destination chain via {@link EvmChain.create} or {@link EvmChainParams.bundlerUrl}.
	 *
	 * @param source - Source chain for order placement
	 * @param dest - Destination chain for order fulfillment
	 * @param intentsCoprocessor - Optional coprocessor for bid fetching and order execution
	 * @returns Initialized IntentGateway instance
	 */
	static async create(
		source: IEvmChain,
		dest: IEvmChain,
		intentsCoprocessor?: IntentsCoprocessor,
	): Promise<IntentGateway> {
		const instance = new IntentGateway(source, dest, intentsCoprocessor)
		await instance.init()
		return instance
	}

	/**
	 * Pre-warms the fee-token cache for both chains and attempts to load the
	 * solver account bytecode into the solver-code cache.
	 *
	 * Called automatically by {@link IntentGateway.create}; not intended for direct use.
	 */
	private async init(): Promise<void> {
		const now = Date.now()
		const sourceFeeToken = await this.source.getFeeTokenWithDecimals()
		this.ctx.feeTokenCache.set(this.source.config.stateMachineId, { ...sourceFeeToken, cachedAt: now })
		const destFeeToken = await this.dest.getFeeTokenWithDecimals()
		this.ctx.feeTokenCache.set(this.dest.config.stateMachineId, { ...destFeeToken, cachedAt: now })

		const solverAccountContract = this.dest.configService.getSolverAccountAddress(this.dest.config.stateMachineId)
		if (solverAccountContract) {
			try {
				const solverCode = await this.dest.client.getCode({ address: solverAccountContract })
				if (solverCode && solverCode !== "0x") {
					this.ctx.solverCodeCache.set(solverAccountContract.toLowerCase(), solverCode)
				}
			} catch {
				// Ignore
			}
		}
	}

	/**
	 * Bidirectional async generator that orchestrates the full order lifecycle:
	 * placement, fee estimation, bid collection, and execution.
	 *
	 * **Yield/receive protocol:**
	 * 1. If `order.fees` is unset or zero, estimates gas and sets `order.fees`
	 *    with a 1% buffer and the wei cost with a 2% buffer for the `value` field.
	 * 2. Yields `AWAITING_PLACE_ORDER` with `{ to, data, value, sessionPrivateKey }`.
	 *    The caller must sign the transaction and pass it back via `gen.next(signedTx)`.
	 * 3. Yields `ORDER_PLACED` with the finalised order and transaction hash once
	 *    the `OrderPlaced` event is confirmed.
	 * 4. Delegates to {@link OrderExecutor.executeOrder} and forwards all
	 *    subsequent status updates until the order is filled, exhausted, or fails.
	 *
	 * @param order - The order to place and execute. `order.fees` may be 0; it
	 *   will be estimated automatically if so.
	 * @param graffiti - Optional bytes32 tag for orderflow attribution /
	 *   revenue share. Defaults to {@link DEFAULT_GRAFFITI}.
	 * @param options - Optional tuning parameters:
	 *   - `maxPriorityFeePerGasBumpPercent` — bump % for the priority fee estimate (default 8).
	 *   - `maxFeePerGasBumpPercent` — bump % for the max fee estimate (default 10).
	 *   - `minBids` — minimum bids to collect before selecting (default 1).
	 *   - `pollIntervalMs` — interval between bid-polling attempts.
	 * @yields {@link IntentOrderStatusUpdate} at each lifecycle stage.
	 * @throws If the `placeOrder` generator behaves unexpectedly, or if gas
	 *   estimation returns zero.
	 */
	async *execute(
		order: Order,
		graffiti: HexString = DEFAULT_GRAFFITI,
		options?: {
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
			minBids?: number
			pollIntervalMs?: number
			solver?: { address: HexString; timeoutMs: number }
		},
	): AsyncGenerator<IntentOrderStatusUpdate, void, HexString> {
		let value: bigint | undefined

		if (!order.fees || order.fees === 0n) {
			const estimate = await this.gasEstimator.estimateFillOrder({
				order,
				maxPriorityFeePerGasBumpPercent: options?.maxPriorityFeePerGasBumpPercent,
				maxFeePerGasBumpPercent: options?.maxFeePerGasBumpPercent,
			})

			if (estimate.totalGasCostWei === 0n || estimate.totalGasInFeeToken === 0n) {
				throw new Error("Gas estimation failed")
			}

			// Solvers using the same estimate algo will have tighter bounds, so we add a buffer.
			value = estimate.totalGasCostWei + (estimate.totalGasCostWei * 2n) / 100n
			order.fees = estimate.totalGasInFeeToken + (estimate.totalGasInFeeToken * 1n) / 100n
		}

		const placeOrderGen = this.orderPlacer.placeOrder(order, graffiti)
		const placeOrderFirst = await placeOrderGen.next()
		if (placeOrderFirst.done) {
			throw new Error("placeOrder generator completed without yielding")
		}
		const { to, data, sessionPrivateKey } = placeOrderFirst.value

		const signedTransaction = yield { status: "AWAITING_PLACE_ORDER", to, data, value, sessionPrivateKey }

		const placeOrderSecond = await placeOrderGen.next(signedTransaction)
		if (placeOrderSecond.done === false) {
			throw new Error("placeOrder generator yielded unexpectedly after signing")
		}
		const { order: finalizedOrder, receipt: placementReceipt } = placeOrderSecond.value as {
			order: Order
			receipt: TransactionReceipt
		}

		yield { status: "ORDER_PLACED", order: finalizedOrder, receipt: placementReceipt }

		for await (const status of this.orderExecutor.executeOrder({
			order: finalizedOrder,
			sessionPrivateKey,
			minBids: options?.minBids,
			pollIntervalMs: options?.pollIntervalMs,
			solver: options?.solver,
		})) {
			yield status
		}

		return
	}

	/**
	 * Validates that an order has the minimum fields required for post-placement
	 * resume (i.e. it was previously placed and has an on-chain identity).
	 *
	 * @throws If `order.id` or `order.session` is missing or zero-valued.
	 */
	private assertOrderCanResume(order: Order): void {
		if (!order.id) {
			throw new Error("Cannot resume execution without order.id")
		}
		if (!order.session || order.session === ADDRESS_ZERO) {
			throw new Error("Cannot resume execution without order.session")
		}
	}

	/**
	 * Resumes execution of a previously placed order.
	 *
	 * Use this method after an app restart or crash to pick up where
	 * {@link execute} left off. The order must already be placed on-chain
	 * (i.e. it must have a valid `id` and `session`).
	 *
	 * Internally delegates to {@link OrderExecutor.executeOrder} and
	 * yields the same status updates as the execution phase of {@link execute}:
	 * `AWAITING_BIDS`, `BIDS_RECEIVED`, `BID_SELECTED`,
	 * `FILLED`, `PARTIAL_FILL`, `EXPIRED`, or `FAILED`.
	 *
	 * Callers may check {@link isOrderFilled} or {@link isOrderRefunded} before
	 * calling this method to avoid resuming an already-terminal order.
	 *
	 * @param order - A previously placed order with a valid `id` and `session`.
	 * @param options - Optional tuning parameters for bid collection and execution.
	 * @yields {@link IntentOrderStatusUpdate} at each execution stage.
	 * @throws If the order is missing required fields for resumption.
	 */
	async *resume(
		order: Order,
		options?: ResumeIntentOrderOptions,
	): AsyncGenerator<IntentOrderStatusUpdate, void> {
		this.assertOrderCanResume(order)

		for await (const status of this.orderExecutor.executeOrder({
			order,
			sessionPrivateKey: options?.sessionPrivateKey,
			minBids: options?.minBids,
			pollIntervalMs: options?.pollIntervalMs,
			solver: options?.solver,
		})) {
			yield status
		}
	}

	/**
	 * Returns both the native token cost and the relayer fee for cancelling an
	 * order. Use `relayerFee` to approve the ERC-20 spend before submitting.
	 *
	 * Delegates to {@link OrderCanceller.quoteCancelOrder}.
	 *
	 * @param order - The order to quote cancellation for.
	 * @param fromDest - If `true`, quotes the destination-initiated cancellation fee.
	 * @returns `{ nativeValue }` — native token amount (wei) to send as `value`;
	 *   `{ relayerFee }` — relayer incentive denominated in the chain's fee token.
	 */
	async quoteCancelOrder(order: Order, fromDest: boolean = false): Promise<CancelQuote> {
		return this.orderCanceller.quoteCancelOrder(order, fromDest)
	}

	/**
	 * Async generator that cancels an order and streams status events until
	 * cancellation is complete.
	 *
	 * Delegates to {@link OrderCanceller.cancelOrder}.
	 *
	 * @param order - The order to cancel.
	 * @param indexerClient - Indexer client used for ISMP request status streaming.
	 * @param fromDest - If `true`, initiates cancellation from the destination chain.
	 *   Defaults to `false` (source-side cancellation).
	 * @yields {@link CancelEvent} objects describing each cancellation stage.
	 */
	async *cancelOrder(
		order: Order,
		indexerClient: IndexerClient,
		fromDest: boolean = false,
	): AsyncGenerator<CancelEvent> {
		yield* this.orderCanceller.cancelOrder(order, indexerClient, fromDest)
	}

	/**
	 * Constructs a signed `PackedUserOperation` for a solver to submit as a bid.
	 *
	 * Delegates to {@link BidManager.prepareSubmitBid}.
	 *
	 * @param options - Bid parameters including order, solver account, gas limits,
	 *   fee market values, and pre-built fill calldata.
	 * @returns A fully signed `PackedUserOperation` ready for submission.
	 */
	async prepareSubmitBid(options: SubmitBidOptions): Promise<PackedUserOperation> {
		return this.bidManager.prepareSubmitBid(options)
	}

	/**
	 * Selects the best available bid, simulates it, and submits the UserOperation
	 * to the bundler.
	 *
	 * Delegates to {@link BidManager.selectBid}.
	 *
	 * @param order - The placed order to fill.
	 * @param bids - Raw filler bids fetched from the coprocessor.
	 * @param sessionPrivateKey - Optional session key override; looked up from
	 *   storage if omitted.
	 * @returns A {@link SelectBidResult} with the submitted UserOperation, hashes,
	 *   and fill status.
	 */
	async selectBid(order: Order, bids: FillerBid[], sessionPrivateKey?: HexString): Promise<SelectBidResult> {
		return this.bidManager.selectBid(order, bids, sessionPrivateKey)
	}

	/**
	 * Estimates the gas cost for filling the given order, returning individual
	 * gas components and fee-token-denominated totals.
	 *
	 * Delegates to {@link GasEstimator.estimateFillOrder}.
	 *
	 * @param params - Estimation parameters including the order and optional
	 *   gas-price bump percentages.
	 * @returns A {@link FillOrderEstimate} with all gas components.
	 */
	async estimateFillOrder(params: EstimateFillOrderParams): Promise<FillOrderEstimate> {
		return this.gasEstimator.estimateFillOrder(params)
	}

	/**
	 * Encodes a list of calls into ERC-7821 `execute` calldata using
	 * single-batch mode.
	 *
	 * Delegates to {@link CryptoUtils.encodeERC7821Execute}.
	 *
	 * @param calls - Ordered list of calls to batch.
	 * @returns ABI-encoded calldata for the ERC-7821 `execute` function.
	 */
	encodeERC7821Execute(calls: ERC7821Call[]): HexString {
		return this._crypto.encodeERC7821Execute(calls)
	}

	/**
	 * Decodes ERC-7821 `execute` calldata back into its constituent calls.
	 *
	 * Delegates to {@link CryptoUtils.decodeERC7821Execute}.
	 *
	 * @param callData - Hex-encoded calldata to decode.
	 * @returns Array of decoded {@link ERC7821Call} objects, or `null` on failure.
	 */
	decodeERC7821Execute(callData: HexString): ERC7821Call[] | null {
		return this._crypto.decodeERC7821Execute(callData)
	}

	/**
	 * Checks whether an order has been filled on the destination chain.
	 *
	 * Delegates to {@link OrderStatusChecker.isOrderFilled}.
	 *
	 * @param order - The order to check.
	 * @returns `true` if the order's commitment slot on the destination chain is
	 *   non-zero (i.e. `fillOrder` has been called successfully).
	 */
	async isOrderFilled(order: Order): Promise<boolean> {
		return this.orderStatusChecker.isOrderFilled(order)
	}

	/**
	 * Checks whether all escrowed inputs for an order have been refunded on the
	 * source chain.
	 *
	 * Delegates to {@link OrderStatusChecker.isOrderRefunded}.
	 *
	 * @param order - The order to check.
	 * @returns `true` if every input token's escrowed amount has been zeroed out
	 *   in the `_orders` mapping on the source chain.
	 */
	async isOrderRefunded(order: Order): Promise<boolean> {
		return this.orderStatusChecker.isOrderRefunded(order)
	}
}
