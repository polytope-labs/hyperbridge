import { createSessionKeyStorage, createCancellationStorage, createUsedUserOpsStorage } from "@/storage"
import { Swap } from "@/utils/swap"
import { type ConsolaInstance, LogLevels, createConsola } from "consola"
import type { TransactionReceipt } from "viem"
import type {
	Order,
	HexString,
	CancelOrderOptions,
	CancelQuote,
	IndexerQueryClient,
	OrderStatus,
	OrderWithStatus,
	AvailableLiquiditySnapshot,
} from "@/types"
import type {
	PackedUserOperation,
	SubmitBidOptions,
	EstimateFillOrderParams,
	FillOrderEstimate,
	IntentOrderStatusUpdate,
	SelectBidResult,
	FillerBid,
	Bid,
} from "@/types"
import type { ResumeIntentOrderOptions } from "@/types"
import type { IEvmChain } from "@/chain"
import type { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { IsmpClient } from "@/client"
import { _queryOrderInternal } from "@/queryClient"
import type { IntentGatewayContext } from "./types"
import type { CancelEvent } from "./types"
import { CryptoUtils } from "./CryptoUtils"
import { OrderPlacer } from "./OrderPlacer"
import { OrderExecutor } from "./OrderExecutor"
import { OrderCanceller } from "./OrderCanceller"
import { BidManager } from "./BidManager"
import { GasEstimator, RELAYER_MESSAGE_GAS } from "./GasEstimator"
import { OrderStatusChecker } from "./OrderStatusChecker"
import { LiquidityEngine } from "./LiquidityEngine"
import {
	type IntentQuoteStrategyHandler,
	type QuoteIntentParams,
	type QuoteIntentResult,
	PhantomSnapshotIntentQuoteStrategy,
	UniswapV4IntentQuoteStrategy,
	UnsupportedIntentQuotePairError,
	UnsupportedIntentQuoteStrategyError,
} from "./quote"
import { PhantomSnapshotPairResolver } from "./quote/phantomSnapshot"
import type { ERC7821Call } from "@/types"
import { DEFAULT_GRAFFITI, DEFAULT_POLL_INTERVAL, ADDRESS_ZERO, sleep } from "@/utils"
import { convertGasToFeeToken } from "./utils"

/**
 * High-level facade for the IntentGatewayV2 protocol.
 *
 * `IntentGateway` orchestrates the complete lifecycle of an intent-based
 * cross-chain swap:
 * - **Quoting** — prices the order's input/output amounts via Phantom order
 *   snapshots by default, with Uniswap V4 available as an explicit strategy.
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
	/** Quote strategies for pricing orders before placement, keyed by strategy name. */
	private readonly quoteStrategies: Record<string, IntentQuoteStrategyHandler>
	/** Resolves order tokens to canonical Phantom snapshot market pairs. */
	private readonly phantomSnapshotPairResolver: PhantomSnapshotPairResolver

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
		this.orderExecutor = new OrderExecutor(this.ctx, bidManager)
		this.orderCanceller = new OrderCanceller(this.ctx)
		this.orderStatusChecker = new OrderStatusChecker(this.ctx)
		this.bidManager = bidManager
		this.gasEstimator = gasEstimator
		this._crypto = crypto
		this.phantomSnapshotPairResolver = new PhantomSnapshotPairResolver(dest.configService)
		this.quoteStrategies = {
			phantom_snapshot: new PhantomSnapshotIntentQuoteStrategy(
				dest.configService,
				() => this.requireIndexer().queryClient,
			),
			uniswap_v4: new UniswapV4IntentQuoteStrategy(dest.configService),
		}
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
	 * Quotes an intent between this gateway's source and destination chains.
	 *
	 * Uses the latest directional Phantom order price snapshot from the attached
	 * indexer by default. Pass `strategy: "uniswap_v4"` only when explicitly
	 * requesting a Uniswap quote. Provide exactly one of `amountIn` or `amountOut`.
	 *
	 * Both built-in strategies resolve their canonical market on Base,
	 * regardless of this gateway's destination chain. Returned
	 * `amountIn`/`amountOut` already account for the gateway's protocol fee
	 * (`quoteMetadata.protocolFeeBps`), which the gateway deducts from order
	 * inputs; use the returned amounts directly when placing the order.
	 *
	 * @param params - Token pair, amount, and optional strategy/pool overrides.
	 * @returns The quoted amounts plus strategy-specific metadata.
	 * @throws {UnsupportedIntentQuoteStrategyError} For unknown strategies.
	 * @throws {UnsupportedIntentQuotePairError} When the selected strategy does not support the pair.
	 * @throws {PhantomSnapshotUnavailableError} When an eligible cNGN pair has no snapshot.
	 */
	async quoteIntent(params: QuoteIntentParams): Promise<QuoteIntentResult> {
		const source = { stateMachineId: this.source.config.stateMachineId, client: this.source.client }
		const destination = { stateMachineId: this.dest.config.stateMachineId, client: this.dest.client }
		const strategy = params.strategy ?? "phantom_snapshot"
		const handler = this.quoteStrategies[strategy]
		if (!handler) throw new UnsupportedIntentQuoteStrategyError(strategy)

		return handler.quote({ ...params, strategy }, source, destination)
	}

	/**
	 * Returns the output-token liquidity measured in the latest directional
	 * Phantom snapshot for this gateway's source and destination.
	 *
	 * Pair resolution uses the same canonical Base market as {@link quoteIntent}.
	 * The snapshot itself determines the output token and chain to aggregate. The
	 * amount is in the token's smallest unit and reflects the indexer's
	 * `snapshotTime`; it is not a live reservation or fill guarantee.
	 *
	 * Requires a prior call to {@link withQueryClient}.
	 */
	async queryAvailableLiquidity(
		params: Pick<QuoteIntentParams, "tokenIn" | "tokenOut">,
	): Promise<AvailableLiquiditySnapshot | undefined> {
		const { queryClient } = this.requireIndexer()
		const sourceStateMachineId = this.source.config.stateMachineId
		const destinationStateMachineId = this.dest.config.stateMachineId
		const pair = this.phantomSnapshotPairResolver.resolve(params, sourceStateMachineId, destinationStateMachineId)
		if (!pair) {
			throw new UnsupportedIntentQuotePairError({
				source: sourceStateMachineId,
				destination: destinationStateMachineId,
				tokenIn: params.tokenIn,
				tokenOut: params.tokenOut,
				quoteSource: "Phantom snapshot pair",
			})
		}

		return new LiquidityEngine(queryClient, this.dest.configService).getAvailableLiquiditySnapshot({
			tokenIn: pair.tokenA,
			tokenOut: pair.tokenB,
		})
	}

	/**
	 * Bidirectional async generator that orchestrates the full order lifecycle:
	 * placement, fee estimation, bid collection, and execution.
	 *
	 * **Yield/receive protocol:**
	 * 1. If `order.fees` is unset or zero, estimates gas on an internal copy and
	 *    sets same-chain fees to twice the estimate. Cross-chain orders retain a
	 *    1% buffer and add a settlement-message uplift of `RELAYER_MESSAGE_GAS`
	 *    (the same gas budget the solver's relayer fee uses), while the wei cost
	 *    used for the `value` field receives a 2% buffer.
	 * 2. Yields `AWAITING_PLACE_ORDER` with `{ to, data, value, sessionPrivateKey }`.
	 *    The caller must sign the transaction and pass it back via `gen.next(signedTx)`.
	 * 3. Yields `ORDER_PLACED` with the finalised order and transaction hash once
	 *    the `OrderPlaced` event is confirmed.
	 * 4. Delegates to {@link OrderExecutor.executeOrder} and forwards all
	 *    subsequent status updates until the order is filled, exhausted, or fails.
	 *
	 * @param order - The order to place and execute. It is not mutated. `order.fees`
	 *   may be 0; fees are estimated automatically if so.
	 * @param graffiti - Optional bytes32 tag for orderflow attribution /
	 *   revenue share. Defaults to {@link DEFAULT_GRAFFITI}.
	 * @param options - Optional tuning parameters:
	 *   - `maxPriorityFeePerGasBumpPercent` — bump % for the priority fee estimate (default 8).
	 *   - `maxFeePerGasBumpPercent` — bump % for the max fee estimate (default 10).
	 *   - `auctionTimeMs` — duration in ms to collect bids before selecting the best one.
	 *   - `pollIntervalMs` — interval between bid-polling attempts.
	 * @yields {@link IntentOrderStatusUpdate} at each lifecycle stage.
	 * @throws If the `placeOrder` generator behaves unexpectedly, or if gas
	 *   estimation returns zero.
	 */
	async *execute(
		order: Order,
		graffiti: HexString = DEFAULT_GRAFFITI,
		options: {
			auctionTimeMs: number
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
			pollIntervalMs?: number
			solver?: { address: HexString; timeoutMs: number }
		},
	): AsyncGenerator<IntentOrderStatusUpdate, void, HexString | SelectBidResult | undefined> {
		const executionOrder: Order = { ...order }
		let value: bigint | undefined

		if (!executionOrder.fees || executionOrder.fees === 0n) {
			const estimate = await this.gasEstimator.estimateFillOrder({
				order: executionOrder,
				maxPriorityFeePerGasBumpPercent: options?.maxPriorityFeePerGasBumpPercent,
				maxFeePerGasBumpPercent: options?.maxFeePerGasBumpPercent,
			})

			if (estimate.totalGasCostWei === 0n || estimate.totalGasInFeeToken === 0n) {
				throw new Error("Gas estimation failed")
			}

			const isSameChain = this.source.config.stateMachineId === this.dest.config.stateMachineId
			value = estimate.totalGasCostWei + (estimate.totalGasCostWei * 2n) / 100n
			// Cover the cross-chain settlement (RedeemEscrow) message with the SAME
			// gas budget the solver's relayer fee is sized against, so a user placing
			// via the SDK attaches exactly what a solver requires to fill.
			const crossChainFeeBump = isSameChain
				? 0n
				: await convertGasToFeeToken(
						this.ctx,
						RELAYER_MESSAGE_GAS,
						"source",
						this.source.config.stateMachineId,
					)

			// Same-chain fills need a larger solver fee margin. Keep cross-chain cost estimation
			// unchanged for Simplex, and apply the user-order fee uplift only at placement.
			executionOrder.fees = isSameChain
				? estimate.totalGasInFeeToken * 2n
				: estimate.totalGasInFeeToken + (crossChainFeeBump + (estimate.totalGasInFeeToken * 1n) / 100n)
		}

		const placeOrderGen = this.orderPlacer.placeOrder(executionOrder, graffiti)
		const placeOrderFirst = await placeOrderGen.next()
		if (placeOrderFirst.done) {
			throw new Error("placeOrder generator completed without yielding")
		}
		const { to, data, sessionPrivateKey } = placeOrderFirst.value

		const signedTransaction = yield { status: "AWAITING_PLACE_ORDER", to, data, value, sessionPrivateKey }

		const placeOrderSecond = await placeOrderGen.next(signedTransaction as HexString)
		if (placeOrderSecond.done === false) {
			throw new Error("placeOrder generator yielded unexpectedly after signing")
		}
		const { order: finalizedOrder, receipt: placementReceipt } = placeOrderSecond.value as {
			order: Order
			receipt: TransactionReceipt
		}

		yield { status: "ORDER_PLACED", order: finalizedOrder, receipt: placementReceipt }

		yield* this.driveExecution(
			this.orderExecutor.executeOrder({
				order: finalizedOrder,
				sessionPrivateKey,
				auctionTimeMs: options.auctionTimeMs,
				pollIntervalMs: options.pollIntervalMs,
				solver: options.solver,
			}),
		)

		return
	}

	/**
	 * Forwards updates from the executor's bidirectional generator to the caller,
	 * threading the {@link SelectBidResult} the caller feeds back after a
	 * `BIDS_RECEIVED` yield into the executor's `.next()`. Other yields expect no
	 * feedback. This is what lets the consumer own `bid.execute()` while the
	 * executor keeps tracking fills and continuing the auction.
	 */
	private async *driveExecution(
		execGen: AsyncGenerator<IntentOrderStatusUpdate, void, SelectBidResult | undefined>,
	): AsyncGenerator<IntentOrderStatusUpdate, void, HexString | SelectBidResult | undefined> {
		try {
			let input: SelectBidResult | undefined
			while (true) {
				const { value, done } = await execGen.next(input)
				input = undefined
				if (done) break

				const fed = yield value
				if (value.status === "BIDS_RECEIVED") input = fed as SelectBidResult | undefined
			}
		} finally {
			// If the consumer stops early (e.g. breaks out / calls `.return()` after
			// BID_SELECTED on a cross-chain order), propagate the teardown so the
			// executor's own `finally` runs and stops its bid/deadline polling.
			await execGen.return()
		}
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
		options: ResumeIntentOrderOptions,
	): AsyncGenerator<IntentOrderStatusUpdate, void, SelectBidResult | undefined> {
		this.assertOrderCanResume(order)

		yield* this.driveExecution(
			this.orderExecutor.executeOrder({
				order,
				sessionPrivateKey: options.sessionPrivateKey,
				auctionTimeMs: options.auctionTimeMs,
				pollIntervalMs: options.pollIntervalMs,
				solver: options.solver,
			}),
		)
	}

	/**
	 * Batteries-included variant of {@link execute}: places the order and then
	 * auto-selects the best bid each round via {@link selectAndExecuteBest}, with
	 * no bid-selection input from the caller.
	 *
	 * The caller still signs the placement transaction: this generator yields
	 * `AWAITING_PLACE_ORDER` and the caller must hand the signed tx back via
	 * `gen.next(signedTx)` exactly as with {@link execute}. Every other stage
	 * (`BIDS_RECEIVED`, `BID_SELECTED`, `FILLED`, …) is handled automatically and
	 * surfaced for observation, so the rest of the loop needs no feedback.
	 *
	 * @param order - The order to place and execute.
	 * @param graffiti - Optional orderflow-attribution tag.
	 * @param options - Same tuning parameters as {@link execute}.
	 * @yields {@link IntentOrderStatusUpdate} at each lifecycle stage.
	 */
	async *executeBest(
		order: Order,
		graffiti: HexString = DEFAULT_GRAFFITI,
		options: {
			auctionTimeMs: number
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
			pollIntervalMs?: number
			solver?: { address: HexString; timeoutMs: number }
		},
	): AsyncGenerator<IntentOrderStatusUpdate, void, HexString> {
		const gen = this.execute(order, graffiti, options)
		try {
			let input: HexString | SelectBidResult | undefined
			let finalizedOrder: Order | undefined
			while (true) {
				const { value, done } = await gen.next(input)
				input = undefined
				if (done) break

				if (value.status === "ORDER_PLACED") {
					finalizedOrder = value.order
					yield value
				} else if (value.status === "BIDS_RECEIVED") {
					if (!finalizedOrder) {
						throw new Error("Received bids before the order was finalized")
					}
					yield value
					input = await this.autoSelect(finalizedOrder, value.bids)
				} else if (value.status === "AWAITING_PLACE_ORDER") {
					input = yield value
				} else {
					yield value
				}
			}
		} finally {
			// Propagate early teardown (consumer break / `.return()`) into the
			// underlying execute() generator so the executor stops polling.
			await gen.return()
		}
	}

	/**
	 * Batteries-included variant of {@link resume}: auto-selects the best bid each
	 * round via {@link selectAndExecuteBest}, with no bid-selection input from the
	 * caller. A plain `for await` loop is sufficient — there is no placement step.
	 *
	 * @param order - A previously placed order with a valid `id` and `session`.
	 * @param options - Optional tuning parameters for bid collection and execution.
	 * @yields {@link IntentOrderStatusUpdate} at each execution stage.
	 */
	async *resumeBest(order: Order, options: ResumeIntentOrderOptions): AsyncGenerator<IntentOrderStatusUpdate, void> {
		const gen = this.resume(order, options)
		try {
			let input: SelectBidResult | undefined
			while (true) {
				const { value, done } = await gen.next(input)
				input = undefined
				if (done) break

				yield value
				if (value.status === "BIDS_RECEIVED") {
					input = await this.autoSelect(order, value.bids)
				}
			}
		} finally {
			// Propagate early teardown (consumer break / `.return()`) into the
			// underlying resume() generator so the executor stops polling.
			await gen.return()
		}
	}

	/**
	 * Auto-select wrapper used by {@link executeBest} / {@link resumeBest}.
	 *
	 * Runs {@link selectAndExecuteBest} and returns the {@link SelectBidResult} to
	 * feed back to the executor. If selection fails this round — all bids fail
	 * simulation, no valid bids, or the bundler rejects the UserOp — it swallows
	 * the error and returns `undefined`, which tells the executor to keep polling
	 * for fresh bids until the deadline rather than aborting the order. Swallowing
	 * the error here (rather than letting it propagate) also keeps the executor's
	 * `finally` teardown intact, since nothing throws across the suspended
	 * generators.
	 */
	private async autoSelect(order: Order, bids: Bid[]): Promise<SelectBidResult | undefined> {
		try {
			return await this.selectAndExecuteBest(order, bids)
		} catch (err) {
			console.warn(
				`[IntentGateway] autoSelect: bid selection failed this round, continuing to poll: ${
					err instanceof Error ? err.message : String(err)
				}`,
			)
			return undefined
		}
	}

	/**
	 * Returns both the native token cost and the relayer fee for cancelling an
	 * order. Use `relayerFee` to approve the ERC-20 spend before submitting.
	 *
	 * Delegates to {@link OrderCanceller.quoteCancelOrder}.
	 *
	 * @param order - The order to quote cancellation for.
	 * @param options - Choose the initiation side. Defaults to source-side cancellation.
	 * @returns `{ nativeValue }` — native token amount (wei) to send as `value`;
	 *   `{ relayerFee }` — relayer incentive denominated in the chain's fee token.
	 */
	async quoteCancelOrder(order: Order, options: CancelOrderOptions = {}): Promise<CancelQuote> {
		return this.orderCanceller.quoteCancelOrder(order, options)
	}

	async quoteCancelOrderFromSource(order: Order): Promise<CancelQuote> {
		return this.orderCanceller.quoteCancelOrder(order, { from: "source" })
	}

	async quoteCancelOrderFromDest(order: Order): Promise<CancelQuote> {
		return this.orderCanceller.quoteCancelOrder(order, { from: "destination" })
	}

	/**
	 * Async generator that cancels an order and streams status events until
	 * cancellation is complete.
	 *
	 * Delegates to {@link OrderCanceller.cancelOrder}.
	 *
	 * @param order - The order to cancel.
	 * @param indexerClient - Indexer client used for ISMP request status streaming.
	 * @param options - Choose the initiation side. Defaults to source-side cancellation.
	 * @yields {@link CancelEvent} objects describing each cancellation stage.
	 */
	async *cancelOrder(
		order: Order,
		indexerClient: IsmpClient,
		options: CancelOrderOptions = {},
	): AsyncGenerator<CancelEvent> {
		yield* this.orderCanceller.cancelOrder(order, indexerClient, options)
	}

	async *cancelOrderFromSource(order: Order, indexerClient: IsmpClient): AsyncGenerator<CancelEvent> {
		yield* this.orderCanceller.cancelOrder(order, indexerClient, { from: "source" })
	}

	async *cancelOrderFromDest(order: Order, indexerClient: IsmpClient): AsyncGenerator<CancelEvent> {
		yield* this.orderCanceller.cancelOrder(order, indexerClient, { from: "destination" })
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
	 * Decodes raw filler bids into first-class {@link Bid} objects that can be
	 * ranked, simulated, and executed by the consumer.
	 *
	 * Delegates to {@link BidManager.buildBids}.
	 *
	 * @param order - The placed order the bids are competing to fill.
	 * @param bids - Raw filler bids fetched from the coprocessor.
	 * @param sessionPrivateKey - Optional session key override; looked up from
	 *   storage by `order.session` if omitted.
	 * @returns Array of executable {@link Bid} objects.
	 */
	buildBids(order: Order, bids: FillerBid[], sessionPrivateKey?: HexString): Bid[] {
		return this.bidManager.buildBids(order, bids, sessionPrivateKey)
	}

	/**
	 * Sorts bids by output value using the same strategy the autopilot uses.
	 *
	 * Delegates to {@link BidManager.sortBids}.
	 *
	 * @param order - The placed order whose output spec drives sorting.
	 * @param bids - Bids to sort (from {@link buildBids}).
	 * @returns Sorted array of {@link Bid} objects.
	 */
	async sortBids(order: Order, bids: Bid[]): Promise<Bid[]> {
		return this.bidManager.sortBids(order, bids)
	}

	/**
	 * Autopilot bid selection: sorts the given bids, simulates each until one
	 * passes, then executes it.
	 *
	 * Delegates to {@link BidManager.selectAndExecuteBest}.
	 *
	 * @param order - The placed order to fill.
	 * @param bids - Candidate bids (from {@link buildBids}).
	 * @returns A {@link SelectBidResult} with the submitted UserOperation, hashes,
	 *   and fill status.
	 */
	async selectAndExecuteBest(order: Order, bids: Bid[]): Promise<SelectBidResult> {
		return this.bidManager.selectAndExecuteBest(order, bids)
	}

	/**
	 * Decodes, sorts, simulates, signs, and submits the best of the given raw
	 * filler bids with no per-bid input from the caller.
	 *
	 * Delegates to {@link BidManager.selectBid}. Prefer {@link buildBids} +
	 * {@link selectAndExecuteBest} (or {@link executeBest}) for new code.
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

	// ── Indexer-backed order status tracking ────────────────────────────

	/**
	 * Optional indexer context for {@link queryOrder} / {@link orderStatusStream}.
	 * Configured via {@link withQueryClient}; unset by default since not every
	 * IntentGateway caller needs indexer access.
	 */
	private indexer?: {
		queryClient: IndexerQueryClient
		pollInterval: number
		logger: ConsolaInstance
	}

	/**
	 * Attaches an indexer GraphQL client to this IntentGateway so that
	 * {@link queryOrder} and {@link orderStatusStream} become available.
	 * Returns `this` for chaining.
	 *
	 * @example
	 * ```ts
	 * const gateway = (await IntentGateway.create(source, dest)).withQueryClient(queryClient)
	 * const order = await gateway.queryOrder("0x...")
	 * ```
	 */
	withQueryClient(queryClient: IndexerQueryClient, options: { pollInterval?: number; tracing?: boolean } = {}): this {
		const logger = createConsola({
			level: LogLevels[options.tracing ? "trace" : "info"],
			formatOptions: { columns: 80, colors: true, compact: true, date: false },
		})
		this.indexer = {
			queryClient,
			pollInterval: options.pollInterval ?? DEFAULT_POLL_INTERVAL,
			logger,
		}
		return this
	}

	private requireIndexer(): NonNullable<IntentGateway["indexer"]> {
		if (!this.indexer) {
			throw new Error(
				"IntentGateway: call withQueryClient(queryClient) before using indexer-backed methods or Phantom quotes",
			)
		}
		return this.indexer
	}

	/**
	 * Queries an order by its commitment hash.
	 *
	 * Requires a prior call to {@link withQueryClient}.
	 */
	async queryOrder(commitment: HexString): Promise<OrderWithStatus | undefined> {
		const { queryClient, logger } = this.requireIndexer()
		return _queryOrderInternal({ commitmentHash: commitment, queryClient, logger })
	}

	/**
	 * Streams status updates for an order until it reaches a terminal state
	 * (FILLED, REDEEMED, or REFUNDED).
	 *
	 * Requires a prior call to {@link withQueryClient}.
	 */
	async *orderStatusStream(commitment: HexString): AsyncGenerator<
		{
			status: OrderStatus
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp: bigint
				filler?: string
			}
		},
		void
	> {
		const { queryClient, pollInterval, logger } = this.requireIndexer()
		const streamLogger = logger.withTag("[orderStatusStream]")
		const TERMINAL = ["FILLED", "REDEEMED", "REFUNDED"] as const

		let order: OrderWithStatus | undefined
		while (!order) {
			await sleep(pollInterval)
			order = await _queryOrderInternal({ commitmentHash: commitment, queryClient, logger })
		}

		streamLogger.trace("`Order` found")
		const latestStatus = order.statuses[order.statuses.length - 1]
		yield { status: latestStatus.status, metadata: latestStatus.metadata }

		if ((TERMINAL as readonly string[]).includes(latestStatus.status)) return

		while (true) {
			await sleep(pollInterval)
			const updatedOrder = await _queryOrderInternal({ commitmentHash: commitment, queryClient, logger })
			if (!updatedOrder) continue

			const newLatestStatus = updatedOrder.statuses[updatedOrder.statuses.length - 1]
			if (newLatestStatus.status !== latestStatus.status) {
				yield { status: newLatestStatus.status, metadata: newLatestStatus.metadata }
				if ((TERMINAL as readonly string[]).includes(newLatestStatus.status)) return
			}
		}
	}
}
