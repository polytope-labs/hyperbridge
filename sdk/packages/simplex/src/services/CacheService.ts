import type { ERC7821Call, TokenInfo } from "@hyperbridge/sdk"
import type { HexString } from "@hyperbridge/sdk"
import { defaultLoggerContext, type Logger, type LoggerContext } from "./Logger"

interface GasEstimateCache {
	totalCostInSourceFeeToken: string
	relayerFeeInSourceFeeToken: string
	dispatchFee: string
	callGasLimit: string
	verificationGasLimit: string
	preVerificationGas: string
	maxFeePerGas: string
	maxPriorityFeePerGas: string
	nonce: string
	totalGasCostWei: string
	timestamp: number
}

interface SwapCall {
	to: string
	data: string
	value: string
}

interface SwapOperationsCache {
	calls: SwapCall[]
	totalGasEstimate: string
	timestamp: number
}

interface FillerOutputCache {
	token: HexString
	amount: string
}

interface FillerOutputsCache {
	outputs: FillerOutputCache[]
	timestamp: number
}

export interface CachedPairClassification {
	inputIsStable: boolean
	stableToken: string
	exoticToken: string
}

interface PairClassificationsCache {
	pairs: CachedPairClassification[]
	timestamp: number
}

interface FundingCallCache {
	target: string
	value: string
	data: string
}

interface FundingPrependsCache {
	calls: FundingCallCache[]
	timestamp: number
}

/**
 * One bid: what a single limit order pays for an incoming order, and what funds it.
 *
 * An order that several limit orders can serve produces several of these, one per
 * order, each priced against the whole input at its own rate. They are never
 * summed: each bid is its own fill, and the gateway clamps whichever one lands
 * against what is still outstanding.
 */
export interface BidPlan {
	limitOrderId: string
	/** What this bid draws from that limit order, at 1e18. */
	payout: bigint
	/** The outputs the bid signs, in the output token's own units. */
	fillerOutputs: TokenInfo[]
	fundingCalls: ERC7821Call[]
	partialFill: boolean
	profit: number
}

interface BidPlanCache {
	limitOrderId: string
	payout: string
	outputs: FillerOutputCache[]
	calls: FundingCallCache[]
	partialFill: boolean
	profit: number
}

interface CacheData {
	gasEstimates: Record<string, GasEstimateCache>
	swapOperations: Record<string, SwapOperationsCache>
	fillerOutputs: Record<string, FillerOutputsCache>
	pairClassifications: Record<string, PairClassificationsCache>
	fundingPrepends: Record<string, FundingPrependsCache>
	/** The limit order an evaluation priced against, carried to the bid that draws on it. */
	matchedLimitOrders: Record<string, { holds: { limitOrderId: string; payout: string }[]; timestamp: number }>
	/** Which bid of the set is being built, so its op carries its own nonce sequence. */
	bidSequences: Record<string, { offset: number; timestamp: number }>
	/** The bids an evaluation decided on, one per limit order it matched. */
	bidPlans: Record<string, { plans: BidPlanCache[]; timestamp: number }>
	/** Orders whose evaluation concluded in a deliberate partial fill. */
	partialFills: Record<string, { partial: boolean; timestamp: number }>
	feeTokens: Record<string, { address: HexString; decimals: number }>
	tokenDecimals: Record<string, Record<HexString, number>>
	solverSelection: Record<string, boolean>
}

export class CacheService {
	private cacheData: CacheData
	private readonly CACHE_EXPIRY_MS = 1 * 60 * 1000 // 1 minute
	private logger: Logger

	constructor(loggers: LoggerContext = defaultLoggerContext()) {
		this.logger = loggers.get("cache-service")
		this.cacheData = {
			gasEstimates: {},
			swapOperations: {},
			fillerOutputs: {},
			pairClassifications: {},
			fundingPrepends: {},
			matchedLimitOrders: {},
			bidPlans: {},
			bidSequences: {},
			partialFills: {},
			feeTokens: {},
			tokenDecimals: {},
			solverSelection: {},
		}
	}

	private isCacheValid(timestamp: number): boolean {
		return Date.now() - timestamp < this.CACHE_EXPIRY_MS
	}

	private cleanupStaleData(): void {
		// Clean up gas estimates
		const staleGasEstimateIds = Object.entries(this.cacheData.gasEstimates)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleGasEstimateIds.forEach((orderId) => {
			delete this.cacheData.gasEstimates[orderId]
		})

		// Clean up matched limit orders
		for (const [orderId, data] of Object.entries(this.cacheData.bidPlans)) {
			if (!this.isCacheValid(data.timestamp)) delete this.cacheData.bidPlans[orderId]
		}
		for (const [orderId, data] of Object.entries(this.cacheData.bidSequences)) {
			if (!this.isCacheValid(data.timestamp)) delete this.cacheData.bidSequences[orderId]
		}
		for (const [orderId, data] of Object.entries(this.cacheData.matchedLimitOrders)) {
			if (!this.isCacheValid(data.timestamp)) delete this.cacheData.matchedLimitOrders[orderId]
		}

		// Clean up swap operations
		const staleSwapOperationIds = Object.entries(this.cacheData.swapOperations)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleSwapOperationIds.forEach((orderId) => {
			delete this.cacheData.swapOperations[orderId]
		})

		// Clean up filler outputs
		const staleFillerOutputIds = Object.entries(this.cacheData.fillerOutputs)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleFillerOutputIds.forEach((orderId) => {
			delete this.cacheData.fillerOutputs[orderId]
		})

		// Clean up pair classifications
		const stalePairIds = Object.entries(this.cacheData.pairClassifications)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		stalePairIds.forEach((orderId) => {
			delete this.cacheData.pairClassifications[orderId]
		})

		const staleFundingIds = Object.entries(this.cacheData.fundingPrepends)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		staleFundingIds.forEach((orderId) => {
			delete this.cacheData.fundingPrepends[orderId]
		})

		// Swept on its own timestamps: most orders never set fundingPrepends, so
		// piggybacking on that sweep left the common case growing without bound.
		const stalePartialIds = Object.entries(this.cacheData.partialFills)
			.filter(([_, data]) => !this.isCacheValid(data.timestamp))
			.map(([orderId]) => orderId)

		stalePartialIds.forEach((orderId) => {
			delete this.cacheData.partialFills[orderId]
		})
	}

	getGasEstimate(orderId: string): {
		totalCostInSourceFeeToken: bigint
		relayerFeeInSourceFeeToken: bigint
		dispatchFee: bigint
		callGasLimit: bigint
		verificationGasLimit: bigint
		preVerificationGas: bigint
		maxFeePerGas: bigint
		maxPriorityFeePerGas: bigint
		nonce: bigint
		totalGasCostWei: bigint
	} | null {
		try {
			const cache = this.cacheData.gasEstimates[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					totalCostInSourceFeeToken: BigInt(cache.totalCostInSourceFeeToken),
					relayerFeeInSourceFeeToken: BigInt(cache.relayerFeeInSourceFeeToken ?? "0"),
					dispatchFee: BigInt(cache.dispatchFee),
					callGasLimit: BigInt(cache.callGasLimit),
					verificationGasLimit: BigInt(cache.verificationGasLimit),
					preVerificationGas: BigInt(cache.preVerificationGas),
					maxFeePerGas: BigInt(cache.maxFeePerGas),
					maxPriorityFeePerGas: BigInt(cache.maxPriorityFeePerGas),
					nonce: BigInt(cache.nonce),
					totalGasCostWei: BigInt(cache.totalGasCostWei),
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting gas estimate")
			return null
		}
	}

	setGasEstimate(
		orderId: string,
		totalCostInSourceFeeToken: bigint,
		relayerFeeInSourceFeeToken: bigint,
		dispatchFee: bigint,
		callGasLimit: bigint,
		verificationGasLimit: bigint,
		preVerificationGas: bigint,
		maxFeePerGas: bigint,
		maxPriorityFeePerGas: bigint,
		nonce: bigint,
		totalGasCostWei: bigint,
	): void {
		if (totalCostInSourceFeeToken <= 0n) {
			throw new Error("Total cost in source fee token must be positive")
		}
		try {
			this.cleanupStaleData()
			this.cacheData.gasEstimates[orderId] = {
				totalCostInSourceFeeToken: totalCostInSourceFeeToken.toString(),
				relayerFeeInSourceFeeToken: relayerFeeInSourceFeeToken.toString(),
				dispatchFee: dispatchFee.toString(),
				callGasLimit: callGasLimit.toString(),
				verificationGasLimit: verificationGasLimit.toString(),
				preVerificationGas: preVerificationGas.toString(),
				maxFeePerGas: maxFeePerGas.toString(),
				maxPriorityFeePerGas: maxPriorityFeePerGas.toString(),
				nonce: nonce.toString(),
				totalGasCostWei: totalGasCostWei.toString(),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting gas estimate")
			throw error
		}
	}

	getSwapOperations(orderId: string): { calls: SwapCall[]; totalGasEstimate: bigint } | null {
		try {
			const cache = this.cacheData.swapOperations[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					calls: cache.calls,
					totalGasEstimate: BigInt(cache.totalGasEstimate),
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting swap operations")
			return null
		}
	}

	setSwapOperations(orderId: string, calls: SwapCall[], totalGasEstimate: bigint): void {
		try {
			this.cleanupStaleData()
			this.cacheData.swapOperations[orderId] = {
				calls,
				totalGasEstimate: totalGasEstimate.toString(),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting swap operations")
			throw error
		}
	}

	getFillerOutputs(orderId: string): { token: HexString; amount: bigint }[] | null {
		try {
			const cache = this.cacheData.fillerOutputs[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return cache.outputs.map((o) => ({
					token: o.token,
					amount: BigInt(o.amount),
				}))
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting filler outputs")
			return null
		}
	}

	setFillerOutputs(orderId: string, outputs: { token: HexString; amount: bigint }[]): void {
		try {
			this.cleanupStaleData()
			this.cacheData.fillerOutputs[orderId] = {
				outputs: outputs.map((o) => ({
					token: o.token,
					amount: o.amount.toString(),
				})),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting filler outputs")
			throw error
		}
	}

	/**
	 * The limit order this order was priced against, and the payout it allowed.
	 *
	 * Carried from `calculateProfitability` to the bid so the reservation lands on
	 * the same limit order the price came from. A fill later finds it through the
	 * bid row, not through here.
	 */
	getMatchedLimitOrder(orderId: string): { limitOrderId: string; payout: bigint }[] {
		try {
			const cache = this.cacheData.matchedLimitOrders[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return cache.holds.map((hold) => ({ limitOrderId: hold.limitOrderId, payout: BigInt(hold.payout) }))
			}
			return []
		} catch (error) {
			this.logger.error({ err: error }, "Error getting matched limit order")
			return []
		}
	}

	getBidPlans(orderId: string): BidPlan[] {
		try {
			const cache = this.cacheData.bidPlans[orderId]
			if (!cache || !this.isCacheValid(cache.timestamp)) return []
			return cache.plans.map((plan) => ({
				limitOrderId: plan.limitOrderId,
				payout: BigInt(plan.payout),
				fillerOutputs: plan.outputs.map((output) => ({ token: output.token, amount: BigInt(output.amount) })),
				fundingCalls: plan.calls.map((call) => ({
					target: call.target as HexString,
					value: BigInt(call.value),
					data: call.data as HexString,
				})),
				partialFill: plan.partialFill,
				profit: plan.profit,
			}))
		} catch (error) {
			this.logger.error({ err: error }, "Error getting bid plans")
			return []
		}
	}

	setBidPlans(orderId: string, plans: BidPlan[]): void {
		try {
			this.cleanupStaleData()
			this.cacheData.bidPlans[orderId] = {
				plans: plans.map((plan) => ({
					limitOrderId: plan.limitOrderId,
					payout: plan.payout.toString(),
					outputs: plan.fillerOutputs.map((output) => ({
						token: output.token as HexString,
						amount: output.amount.toString(),
					})),
					calls: plan.fundingCalls.map((call) => ({
						target: call.target.toLowerCase(),
						value: call.value.toString(),
						data: call.data,
					})),
					partialFill: plan.partialFill,
					profit: plan.profit,
				})),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting bid plans")
			throw error
		}
	}

	/** Which bid of the set is being built. Zero when only one bid is going out. */
	getBidSequence(orderId: string): number {
		const cache = this.cacheData.bidSequences[orderId]
		return cache && this.isCacheValid(cache.timestamp) ? cache.offset : 0
	}

	setBidSequence(orderId: string, offset: number): void {
		this.cacheData.bidSequences[orderId] = { offset, timestamp: Date.now() }
	}

	clearBidPlans(orderId: string): void {
		delete this.cacheData.bidPlans[orderId]
	}

	setMatchedLimitOrder(orderId: string, holds: { limitOrderId: string; payout: bigint }[]): void {
		try {
			this.cleanupStaleData()
			this.cacheData.matchedLimitOrders[orderId] = {
				holds: holds.map((hold) => ({ limitOrderId: hold.limitOrderId, payout: hold.payout.toString() })),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting matched limit order")
			throw error
		}
	}

	getPairClassifications(orderId: string): CachedPairClassification[] | null {
		try {
			const cache = this.cacheData.pairClassifications[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return cache.pairs
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting pair classifications")
			return null
		}
	}

	setPairClassifications(orderId: string, pairs: CachedPairClassification[]): void {
		try {
			this.cacheData.pairClassifications[orderId] = {
				pairs,
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting pair classifications")
			throw error
		}
	}

	getFundingPrepends(orderId: string): { calls: ERC7821Call[] } | null {
		try {
			const cache = this.cacheData.fundingPrepends[orderId]
			if (cache && this.isCacheValid(cache.timestamp)) {
				return {
					calls: cache.calls.map((c) => ({
						target: c.target as HexString,
						value: BigInt(c.value),
						data: c.data as HexString,
					})),
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting funding prepends")
			return null
		}
	}

	setFundingPrepends(orderId: string, calls: ERC7821Call[]): void {
		try {
			this.cleanupStaleData()
			this.cacheData.fundingPrepends[orderId] = {
				calls: calls.map((c) => ({
					target: c.target.toLowerCase(),
					value: c.value.toString(),
					data: c.data,
				})),
				timestamp: Date.now(),
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error setting funding prepends")
			throw error
		}
	}

	clearFundingPrepends(orderId: string): void {
		delete this.cacheData.fundingPrepends[orderId]
	}

	/**
	 * Whether the strategy's completed evaluation of this order chose a partial fill.
	 *
	 * Only ever written by an evaluation that went on to return a fillable answer,
	 * and cleared at the start of every evaluation — so a refusal can never leave a
	 * `true` behind for the caller to act on.
	 */
	isPartialFill(orderId: string): boolean {
		const cache = this.cacheData.partialFills[orderId]
		return cache !== undefined && this.isCacheValid(cache.timestamp) && cache.partial
	}

	setPartialFill(orderId: string, partial: boolean): void {
		this.cacheData.partialFills[orderId] = { partial, timestamp: Date.now() }
	}

	clearPartialFill(orderId: string): void {
		delete this.cacheData.partialFills[orderId]
	}

	getFeeTokenWithDecimals(chain: string): { address: HexString; decimals: number } | null {
		try {
			const cache = this.cacheData.feeTokens[chain]
			if (cache) {
				return {
					address: cache.address,
					decimals: cache.decimals,
				}
			}
			return null
		} catch (error) {
			this.logger.error({ err: error }, "Error getting fee token with decimals")
			return null
		}
	}

	setFeeTokenWithDecimals(chain: string, address: HexString, decimals: number): void {
		try {
			this.cleanupStaleData()

			this.cacheData.feeTokens[chain] = { address, decimals }
		} catch (error) {
			this.logger.error({ chain: chain, err: error }, "Error setting fee token with decimals")
			throw error
		}
	}

	getTokenDecimals(chain: string, tokenAddress: HexString): number | null {
		try {
			const chainCache = this.cacheData.tokenDecimals[chain]
			if (chainCache && chainCache[tokenAddress]) {
				return chainCache[tokenAddress]
			}
			return null
		} catch {
			return null
		}
	}

	setTokenDecimals(chain: string, tokenAddress: HexString, decimals: number): void {
		try {
			this.cleanupStaleData()
			// Ensure the chain object exists before setting the token decimals
			if (!this.cacheData.tokenDecimals[chain]) {
				this.cacheData.tokenDecimals[chain] = {}
			}
			this.cacheData.tokenDecimals[chain][tokenAddress] = decimals
		} catch (error) {
			this.logger.error({ chain: chain, tokenAddress: tokenAddress, err: error }, "Error setting token decimals")
			throw error
		}
	}

	getSolverSelection(chain: string): boolean | null {
		const cached = this.cacheData.solverSelection[chain]
		return cached !== undefined ? cached : null
	}

	setSolverSelection(chain: string, active: boolean): void {
		this.cacheData.solverSelection[chain] = active
	}
}
