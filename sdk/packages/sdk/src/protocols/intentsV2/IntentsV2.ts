import { createSessionKeyStorage, createCancellationStorage } from "@/storage"
import { Swap } from "@/utils/swap"
import type { OrderV2, HexString } from "@/types"
import type {
	PackedUserOperation,
	SubmitBidOptions,
	EstimateFillOrderV2Params,
	FillOrderEstimateV2,
	IntentOrderStatusUpdate,
	ExecuteIntentOrderOptions,
	SelectBidResult,
	FillerBid,
} from "@/types"
import type { IEvmChain } from "@/chain"
import type { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { IndexerClient } from "@/client"
import type { IntentsV2Context } from "./types"
import type { CancelEvent } from "./types"
import { CryptoUtils } from "./CryptoUtils"
import { OrderPlacer } from "./OrderPlacer"
import { OrderExecutor } from "./OrderExecutor"
import { OrderCanceller } from "./OrderCanceller"
import { BidManager } from "./BidManager"
import { GasEstimator } from "./GasEstimator"
import { OrderStatusChecker } from "./OrderStatusChecker"
import type { ERC7821Call } from "@/types"
import { DEFAULT_GRAFFITI } from "@/utils"

/**
 * IntentsV2 utilities for placing orders, submitting bids, and managing the intent lifecycle.
 *
 * Use `IntentsV2.create()` to obtain an initialized instance.
 */
export class IntentsV2 {
	readonly source: IEvmChain
	readonly dest: IEvmChain
	readonly intentsCoprocessor?: IntentsCoprocessor
	readonly bundlerUrl?: string

	private readonly ctx: IntentsV2Context
	private readonly _crypto: CryptoUtils
	private readonly orderPlacer: OrderPlacer
	private readonly orderExecutor: OrderExecutor
	private readonly orderCanceller: OrderCanceller
	private readonly orderStatusChecker: OrderStatusChecker
	private readonly bidManager: BidManager
	private readonly gasEstimator: GasEstimator

	private constructor(
		source: IEvmChain,
		dest: IEvmChain,
		intentsCoprocessor?: IntentsCoprocessor,
		bundlerUrl?: string,
	) {
		this.source = source
		this.dest = dest
		this.intentsCoprocessor = intentsCoprocessor
		this.bundlerUrl = bundlerUrl

		const sessionKeyStorage = createSessionKeyStorage()
		const cancellationStorage = createCancellationStorage()
		const swap = new Swap()
		const feeTokenCache = new Map<string, { address: HexString; decimals: number; cachedAt: number }>()
		const solverCodeCache = new Map<string, string>()

		this.ctx = {
			source,
			dest,
			intentsCoprocessor,
			bundlerUrl,
			feeTokenCache,
			solverCodeCache,
			sessionKeyStorage,
			cancellationStorage,
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
	}

	/**
	 * Creates an initialized IntentsV2 instance.
	 *
	 * @param source - Source chain for order placement
	 * @param dest - Destination chain for order fulfillment
	 * @param intentsCoprocessor - Optional coprocessor for bid fetching and order execution
	 * @param bundlerUrl - Optional ERC-4337 bundler URL for gas estimation and UserOp submission
	 * @returns Initialized IntentsV2 instance
	 */
	static async create(
		source: IEvmChain,
		dest: IEvmChain,
		intentsCoprocessor?: IntentsCoprocessor,
		bundlerUrl?: string,
	): Promise<IntentsV2> {
		const instance = new IntentsV2(source, dest, intentsCoprocessor, bundlerUrl)
		await instance.init()
		return instance
	}

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

	async *execute(
		order: OrderV2,
		graffiti: HexString = DEFAULT_GRAFFITI,
		options?: {
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
			minBids?: number
			bidTimeoutMs?: number
			pollIntervalMs?: number
		},
	): AsyncGenerator<
		{ calldata: HexString; sessionPrivateKey: HexString } | IntentOrderStatusUpdate,
		void,
		HexString
	> {
		if (!order.fees || order.fees === 0n) {
			const estimate = await this.gasEstimator.estimateFillOrderV2({
				order,
				maxPriorityFeePerGasBumpPercent: options?.maxPriorityFeePerGasBumpPercent,
				maxFeePerGasBumpPercent: options?.maxFeePerGasBumpPercent,
			})
			order.fees = estimate.totalGasInFeeToken
		}

		const placeOrderGen = this.orderPlacer.placeOrder(order, graffiti)
		const placeOrderFirst = await placeOrderGen.next()
		if (placeOrderFirst.done) {
			throw new Error("placeOrder generator completed without yielding")
		}
		const { calldata, sessionPrivateKey } = placeOrderFirst.value

		const signedTransaction = yield { calldata, sessionPrivateKey }

		const placeOrderSecond = await placeOrderGen.next(signedTransaction)
		if (placeOrderSecond.done === false) {
			throw new Error("placeOrder generator yielded unexpectedly after signing")
		}
		const finalizedOrder = placeOrderSecond.value as OrderV2

		for await (const status of this.orderExecutor.executeIntentOrder({
			order: finalizedOrder,
			sessionPrivateKey,
			minBids: options?.minBids,
			bidTimeoutMs: options?.bidTimeoutMs,
			pollIntervalMs: options?.pollIntervalMs,
		})) {
			yield status
		}
	}

	async *placeOrder(
		order: OrderV2,
		graffiti: HexString = DEFAULT_GRAFFITI,
	): AsyncGenerator<{ calldata: HexString; sessionPrivateKey: HexString }, OrderV2, any> {
		return yield* this.orderPlacer.placeOrder(order, graffiti)
	}

	async *executeIntentOrder(options: ExecuteIntentOrderOptions): AsyncGenerator<IntentOrderStatusUpdate, void> {
		yield* this.orderExecutor.executeIntentOrder(options)
	}

	async quoteCancelNative(order: OrderV2, from: "source" | "dest" = "source"): Promise<bigint> {
		return this.orderCanceller.quoteCancelNative(order, from)
	}

	async *cancelOrder(
		order: OrderV2,
		indexerClient: IndexerClient,
		from: "source" | "dest" = "source",
	): AsyncGenerator<CancelEvent> {
		yield* this.orderCanceller.cancelOrder(order, indexerClient, from)
	}

	async prepareSubmitBid(options: SubmitBidOptions): Promise<PackedUserOperation> {
		return this.bidManager.prepareSubmitBid(options)
	}

	async selectBid(order: OrderV2, bids: FillerBid[], sessionPrivateKey?: HexString): Promise<SelectBidResult> {
		return this.bidManager.selectBid(order, bids, sessionPrivateKey)
	}

	async estimateFillOrderV2(params: EstimateFillOrderV2Params): Promise<FillOrderEstimateV2> {
		return this.gasEstimator.estimateFillOrderV2(params)
	}

	encodeERC7821Execute(calls: ERC7821Call[]): HexString {
		return this._crypto.encodeERC7821Execute(calls)
	}

	decodeERC7821Execute(callData: HexString): ERC7821Call[] | null {
		return this._crypto.decodeERC7821Execute(callData)
	}

	async isOrderFilled(order: OrderV2): Promise<boolean> {
		return this.orderStatusChecker.isOrderFilled(order)
	}

	async isOrderRefunded(order: OrderV2): Promise<boolean> {
		return this.orderStatusChecker.isOrderRefunded(order)
	}
}
