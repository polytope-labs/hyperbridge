import { FillerStrategy } from "@/strategies/base"
import {
	Order,
	ExecutionResult,
	HexString,
	adjustDecimals,
	bytes32ToBytes20,
	type ERC7821Call,
	TokenInfo,
	IntentsCoprocessor,
	ADDRESS_ZERO,
} from "@hyperbridge/sdk"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { AssetRegistry, normalizeSymbol, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import { unanchoredToken0Symbols } from "@/config/pairs"
import { type CachedPairClassification } from "@/services/CacheService"
import { Decimal } from "decimal.js"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { FundingVenue } from "@/funding/types"
import type { SigningAccount } from "@/services/wallet"

/**
 * A trading pair the engine serves. `token0` and `token1` are registry symbols
 * (see `AssetRegistry`); the price policies quote **token1 per 1 token0**,
 * keyed by the order's token0 notional.
 *
 * The bid policy prices the filler *buying* token1 (user sends token1, receives
 * token0); the ask policy prices the filler *selling* token1. A missing policy
 * disables that direction for this pair (one-sided LP). A pair with neither
 * policy is priced from a Uniswap V4 venue (USD-stable `token0` only).
 *
 * A **same-token pair** (`token0 == token1`, e.g. USDC/USDC) is the same-asset
 * cross-chain market: ask-only, with the ask price at or below par — the gap
 * to 1 is the filler's spread, realized in-kind on every fill.
 *
 * `maxOrderSize` caps the pair's exposure per order, denominated in token0 —
 * the curve amount axis shares that unit, so trade pricing never consults an
 * external feed. Confirmation sizing alone converts token0 notionals to USD,
 * derived from the declared curves (USD stables at $1, curve mids as FX edges).
 */
export interface TradingPair {
	token0: string
	token1: string
	/** Maximum token0 notional this pair fills per order. */
	maxOrderSize: Decimal
	bidPricePolicy?: FillerPricePolicy
	askPricePolicy?: FillerPricePolicy
}

/** Whether a pair quotes the same asset on both sides (same-asset cross-chain market). */
function isSameTokenPair(pair: TradingPair): boolean {
	return normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1)
}

/** Zero-notional mid of a pair's curves (token1 per token0), or null when curve-less. */
function pairMidRate(pair: TradingPair): Decimal | null {
	const rates: Decimal[] = []
	for (const policy of [pair.bidPricePolicy, pair.askPricePolicy]) {
		const rate = policy?.getPrice(new Decimal(0))
		if (rate?.gt(0)) rates.push(rate)
	}
	if (rates.length === 0) return null
	return rates.reduce((a, b) => a.plus(b)).div(rates.length)
}

/** A leg matched to a configured pair, with everything needed to price it. */
interface ResolvedLeg {
	pair: TradingPair
	/** True when the leg's input is the pair's token0 (filler sells token1). */
	inputIsToken0: boolean
	/** token1 address on the chain where the exotic side of this leg settles. */
	token1Address: string
	/** Chain (state machine id) where the token1 side of this leg lives. */
	token1Chain: string
}

/** Rate context resolved for a leg: pricing rate plus the opposite side for margin marking. */
interface LegRates {
	/** token1 per token0 used to price this leg's output. */
	rate: Decimal
	/** The opposite side's rate (bid for ask-legs, ask for bid-legs), when available. */
	oppositeRate: Decimal | null
	priceSource: "venue" | "policy"
}

/**
 * Strategy for swaps across a configurable set of trading pairs, each priced
 * and sized by its own bid/ask curves (or a Uniswap V4 venue). Supports both
 * same-chain and cross-chain orders.
 *
 * Pairs are declared as `token0`/`token1` registry symbols — e.g. USDC/CNGN,
 * USDT/CNGN, ZARP/CNGN — and any number of pairs can run in one engine. Curves
 * are quoted in **token1 per token0**; nothing assumes the quote side is a USD
 * stablecoin and no external price feed is consulted. Trade sizing is
 * pair-local in token0 units (the per-order `maxOrderSize` cap and the curve
 * amount axis); only confirmation sizing converts to USD, using the curves
 * themselves as the price feed (see `usdFactors`).
 *
 * For each (input, output) leg the engine finds the configured pair matching
 * the leg's direction:
 *  - input = token0, output = token1 → the filler *sells* token1 at the ask.
 *  - input = token1, output = token0 → the filler *buys* token1 at the bid.
 *
 * The filler holds inventory on both sides of its pairs. Profitability
 * evaluation caps each pair's legs at the pair's `maxOrderSize`, prices each
 * leg with its pair's curve (or venue), and bounds outputs by the filler's
 * real balances plus funding-venue withdrawals. Because the IntentGateway
 * releases inputs proportionally to the fraction of outputs provided, partial
 * fills (and overfills) need no extra on-chain logic.
 */
export class FXFiller implements FillerStrategy {
	name = "FXFiller"
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	/** Trading pairs served by this engine; each with its own bid/ask policies and cap. */
	private pairs: TradingPair[]
	/** Symbol → per-chain address resolution (built-ins + curated + user `[assets]`). */
	private registry: AssetRegistry
	private signer: SigningAccount
	private logger = getLogger("fx-simplex")
	/** Consecutive orders where overfill clamp activated. */
	private consecutiveClamps = 0
	/** Once set, the filler refuses all orders until restart — systemic pricing error suspected. */
	private halted = false
	/** Ceiling bps above user-requested output. Sourced from filler config. */
	private readonly maxOverfillBps: bigint
	/** Consecutive clamped evaluations before halting. Sourced from filler config. */
	private readonly maxConsecutiveClamps: number
	confirmationPolicy?: { getConfirmationBlocks: (chainId: number, amountUsd: number) => number }
	private fundingVenues: FundingVenue[]
	/**
	 * Optional Uniswap price guard, keyed by chain. When a chain has an entry, a
	 * venue (pool) quote is only trusted if it stays within `maxDeviationBps` of the
	 * static `reference` price (token1 per USD); a quote outside the band rejects the
	 * order — defence against a manipulated, stale, or thin pool. Sourced from the
	 * per-position config under `[strategies.vault.uniswapV4]`.
	 */
	private priceGuard?: Map<string, { reference: Decimal; maxDeviationBps: number }>
	/**
	 * One-sided switch for venue-priced pairs (no static curves): "bid" only buys
	 * token1, "ask" only sells it. Curve-priced pairs express one-sidedness by
	 * omitting a curve instead.
	 */
	private side?: "bid" | "ask"

	/**
	 * @param signer          Filler's signing account for UserOp signatures.
	 * @param configService   Network/config provider for addresses and decimals.
	 * @param clientManager   Used to get viem PublicClients for chains.
	 * @param contractService Shared contract interaction service.
	 * @param pairs           Trading pairs with their bid/ask price policies and per-order caps.
	 * @param registry        Asset symbol registry resolving pair symbols per chain.
	 * @param options.confirmationPolicy Optional per-chain confirmation policy for cross-chain orders.
	 * @param options.fundingVenues  Optional funding venues for on-chain liquidity sourcing and live pricing.
	 * @param options.side    Venue-pricing one-sided switch ("bid" buys token1, "ask" sells token1).
	 *   Only valid when no pair has static curves; curve-priced pairs go one-sided by omitting a curve.
	 */
	constructor(
		signer: SigningAccount,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		pairs: TradingPair[],
		registry: AssetRegistry,
		options?: {
			confirmationPolicy?: ConfirmationPolicy
			fundingVenues?: FundingVenue[]
			priceGuard?: Record<string, { referencePrice: string; maxDeviationBps: number }>
			side?: "bid" | "ask"
		},
	) {
		const { confirmationPolicy, fundingVenues = [], priceGuard, side } = options ?? {}

		if (pairs.length === 0) {
			throw new Error("FXFiller requires at least one trading pair")
		}
		const hasAnyPolicy = pairs.some((p) => p.bidPricePolicy || p.askPricePolicy)
		const hasVenues = fundingVenues.length > 0

		if (!hasAnyPolicy && !hasVenues) {
			throw new Error("FXFiller requires price curves on its pairs, or funding venues for pool pricing")
		}
		if (side && hasAnyPolicy) {
			throw new Error("FXFiller 'side' only applies to venue (pool) pricing; omit pair price curves")
		}
		const seenPairs = new Set<string>()
		for (const pair of pairs) {
			const label = `${normalizeSymbol(pair.token0)}/${normalizeSymbol(pair.token1)}`
			const reversed = `${normalizeSymbol(pair.token1)}/${normalizeSymbol(pair.token0)}`
			if (seenPairs.has(label) || seenPairs.has(reversed)) {
				throw new Error(
					`FXFiller pair ${pair.token0}/${pair.token1}: duplicate market (a pair and its reverse are the same market)`,
				)
			}
			seenPairs.add(label)
			if (!pair.maxOrderSize.isFinite() || pair.maxOrderSize.lte(0)) {
				throw new Error(
					`FXFiller pair ${pair.token0}/${pair.token1}: maxOrderSize must be a positive token0 amount`,
				)
			}
			if (isSameTokenPair(pair)) {
				// Same-asset market: ask-only, priced at or below par — above par
				// pays out more than it receives on every fill.
				if (pair.bidPricePolicy || !pair.askPricePolicy) {
					throw new Error(
						`FXFiller pair ${pair.token0}/${pair.token1}: same-token pairs need exactly an ask policy (they are ask-only)`,
					)
				}
				const abovePar = pair.askPricePolicy.getPoints().some((p) => new Decimal(p.price).gt(1))
				if (abovePar) {
					throw new Error(
						`FXFiller pair ${pair.token0}/${pair.token1}: same-token ask prices must not exceed 1`,
					)
				}
				continue
			}
			if (!pair.bidPricePolicy && !pair.askPricePolicy) {
				if (!hasVenues) {
					throw new Error(
						`FXFiller pair ${pair.token0}/${pair.token1}: needs a bid and/or ask policy, or funding venues`,
					)
				}
				if (!USD_STABLE_SYMBOLS.has(normalizeSymbol(pair.token0))) {
					throw new Error(
						`FXFiller pair ${pair.token0}/${pair.token1}: venue (pool) pricing requires a USD-stable token0 — add price curves instead`,
					)
				}
			}
		}

		// Mirrors validatePairConfigs for direct SDK construction: confirmation
		// depth prices token0 notionals in USD through the curve graph, so every
		// token0 must be reachable from a USD stable.
		const unanchored = unanchoredToken0Symbols(
			pairs.map((p) => ({
				token0: p.token0,
				token1: p.token1,
				hasCurve: Boolean(p.bidPricePolicy || p.askPricePolicy),
			})),
		)
		if (unanchored.length > 0) {
			throw new Error(
				`FXFiller: no USD anchor for ${unanchored.join(", ")} — add a curve-priced pair against a USD stable (e.g. USDC/${unanchored[0]}), directly or through an already-anchored asset`,
			)
		}

		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.pairs = pairs
		this.registry = registry
		this.fundingVenues = fundingVenues
		this.side = side
		if (priceGuard && Object.keys(priceGuard).length > 0) {
			this.priceGuard = new Map()
			for (const [chain, guard] of Object.entries(priceGuard)) {
				this.priceGuard.set(chain, {
					reference: new Decimal(guard.referencePrice),
					maxDeviationBps: guard.maxDeviationBps,
				})
			}
		}

		this.signer = signer
		this.maxOverfillBps = configService.getMaxOverfillBps()
		this.maxConsecutiveClamps = configService.getMaxConsecutiveClamps()
		if (confirmationPolicy) {
			this.confirmationPolicy = {
				getConfirmationBlocks: (chainId: number, amountUsd: number) =>
					confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
			}
		}
	}

	// =========================================================================
	// Lifecycle
	// =========================================================================

	/**
	 * Call once at startup after construction.
	 * Hydrates all funding venue state so venue-priced pairs quote from live pool data.
	 */
	async initialise(): Promise<void> {
		const solver = this.signer.account.address as HexString
		await Promise.all(this.fundingVenues.map((v) => v.initialise(solver)))
	}

	/**
	 * Queries funding venues for `token1Address`'s USD price on a chain.
	 * Uniswap V4 is preferred; falls back to other venues. Returns null when no
	 * venue can price the token there.
	 */
	private async getVenueUsdPrice(chain: string, token1Address: string): Promise<Decimal | null> {
		if (this.fundingVenues.length === 0) return null

		// Prefer V4, fall back to others
		const v4 = this.fundingVenues.filter((v) => v.name === "UniswapV4")
		const venues = v4.length > 0 ? v4 : this.fundingVenues

		for (const venue of venues) {
			const usdPrice = await venue.getExoticTokenPrice(chain, token1Address)
			if (usdPrice?.isPositive()) return usdPrice
		}
		return null
	}

	/** Per-evaluation memo over `getVenueUsdPrice`, keyed by (chain, token1). */
	private venuePriceMemo(): (chain: string, token1Address: string) => Promise<Decimal | null> {
		const cache = new Map<string, Decimal | null>()
		return async (chain: string, token1Address: string) => {
			const key = `${chain}:${token1Address}`
			const cached = cache.get(key)
			if (cached !== undefined) return cached
			const price = await this.getVenueUsdPrice(chain, token1Address)
			cache.set(key, price)
			return price
		}
	}

	/**
	 * Validates a live venue quote against the static reference price for the chain.
	 * Returns true (pass) when no guard is configured, or no reference exists for the
	 * chain. Returns false when the quote (token1 per USD) deviates from the reference
	 * by more than `maxDeviationBps`, in which case the order must not be filled.
	 */
	private checkPriceGuard(orderId: string | undefined, chain: string, venueToken1PerUsd: Decimal): boolean {
		const guard = this.priceGuard?.get(chain)
		if (!guard || guard.reference.lte(0)) return true

		const deviationBps = venueToken1PerUsd.minus(guard.reference).abs().div(guard.reference).mul(10000)
		if (deviationBps.gt(guard.maxDeviationBps)) {
			this.logger.warn(
				{
					orderId,
					chain,
					venuePrice: venueToken1PerUsd.toString(),
					referencePrice: guard.reference.toString(),
					deviationBps: deviationBps.toFixed(2),
					maxDeviationBps: guard.maxDeviationBps,
				},
				"Rejecting order: Uniswap venue quote outside price-guard band",
			)
			return false
		}
		return true
	}

	async canFill(order: Order): Promise<boolean> {
		if (this.halted) {
			this.logger.warn({ orderId: order.id }, "FXFiller halted — rejecting order")
			return false
		}
		try {
			if (order.inputs.length !== order.output.assets.length) {
				this.logger.debug(
					{ inputs: order.inputs.length, outputs: order.output.assets.length },
					"Order input/output length mismatch or empty",
				)
				return false
			}

			const legs = this.resolveOrderLegs(order)
			if (!legs) {
				this.logger.debug(
					{ sourceChain: order.source, destChain: order.destination },
					"No configured pair matches the order's token legs",
				)
				return false
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Error in canFill")
			return false
		}
	}

	/**
	 * Evaluates whether an order is profitable to fill under the per-pair
	 * `maxOrderSize` caps and the filler's current token balances.
	 *
	 * High-level flow:
	 * - Resolve each (input, output) leg to a configured pair and direction.
	 * - Estimate each pair's total token0 notional in the order and cap it at
	 *   the pair's `maxOrderSize`; pair curves are evaluated at that capped
	 *   notional.
	 * - Walk the legs, allocating from each pair's capped token0 budget and
	 *   pricing outputs at the pair's rate.
	 * - Further cap each leg by the filler's current token balance plus
	 *   funding-venue withdrawals.
	 * - Cache the resulting outputs for later use in `executeOrder`.
	 *
	 * Note: we may intentionally overfill relative to the user's requested
	 * outputs if the pair pricing makes that attractive. This is how we stay competitive.
	 */
	async calculateProfitability(order: Order): Promise<number> {
		if (this.halted) {
			this.logger.warn({ orderId: order.id }, "FXFiller halted — rejecting order")
			return 0
		}
		try {
			const sourceChain = order.source
			const destChain = order.destination
			const { decimals: feeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(sourceChain)

			const destClient = this.clientManager.getPublicClient(destChain)
			const walletAddress = this.signer.account.address as HexString
			const balanceCache = new Map<string, bigint>()

			const legs = this.resolveOrderLegs(order)
			if (!legs) {
				this.logger.info({ orderId: order.id }, "Skipping order: no configured pair matches its legs")
				return 0
			}

			// A zero requested output is degenerate on the fill path: the gateway
			// releases no escrow for it (remaining == 0), yet the leg would still be
			// sized and could feed the profit gate. Never bid on such an order.
			if (order.output.assets.some((a) => a.amount === 0n)) {
				this.logger.info({ orderId: order.id }, "Skipping order: contains a zero-amount requested output")
				return 0
			}

			const venueUsdPrice = this.venuePriceMemo()

			// Per-pair token0 notionals, capped at each pair's maxOrderSize. The
			// capped notional is both the curve evaluation point and the budget
			// legs of that pair draw from.
			const sized = await this.sizeOrder(order, legs, venueUsdPrice)
			if (!sized) {
				this.logger.info({ orderId: order.id }, "Skipping order: could not size the order's legs")
				return 0
			}
			const { legNotionals, cappedByPair, totalNotional } = sized

			const remainingByPair = new Map(cappedByPair)

			const fillerOutputs: TokenInfo[] = []
			// Original leg index for each entry in `fillerOutputs`. Legs can be skipped
			// (insufficient balance, exhausted budget), so `fillerOutputs[k]` is the k-th
			// *surviving* leg, not the k-th leg. The valuation pass below realigns to the
			// original input/leg via this array rather than by position.
			const fillerOutputLegs: number[] = []
			// Rate context per original leg index, captured during the leg loop so the
			// margin pass below prices with the same numbers.
			const legRatesByIndex = new Map<number, LegRates>()

			const fundingCalls: ERC7821Call[] = []

			let deadlineTimestamp: bigint | undefined
			try {
				const latestBlock = await destClient.getBlock()
				const blockTimeMs = destClient.chain?.blockTime
				const blockTimeSec = blockTimeMs ? blockTimeMs / 1000 : 2
				const remainingBlocks = order.deadline > latestBlock.number ? Number(order.deadline - latestBlock.number) : 0
				deadlineTimestamp = BigInt(Math.floor(Number(latestBlock.timestamp) + remainingBlocks * blockTimeSec))
			} catch (err) {
				this.logger.warn({ err, destChain }, "Failed to estimate deadline timestamp, using fallback")
			}

			for (let i = 0; i < order.inputs.length; i++) {
				const input = order.inputs[i]
				const output = order.output.assets[i]
				const leg = legs[i]

				const inputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(input.token) as HexString,
					sourceChain,
				)
				const outputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(output.token) as HexString,
					destChain,
				)

				const token0Decimals = leg.inputIsToken0 ? inputDecimals : outputDecimals
				const token1Decimals = leg.inputIsToken0 ? outputDecimals : inputDecimals

				const cappedNotional = cappedByPair.get(leg.pair) ?? leg.pair.maxOrderSize
				const rates = await this.resolveLegRates(order.id, leg, cappedNotional, venueUsdPrice)
				if (!rates) return 0
				legRatesByIndex.set(i, rates)

				const remaining = remainingByPair.get(leg.pair) ?? new Decimal(0)
				const legResult = this.computeLegPolicyOutput(
					input.amount,
					leg.inputIsToken0,
					token0Decimals,
					token1Decimals,
					remaining,
					rates.rate,
				)

				if (!legResult) {
					// Budget exhausted for this pair. Emit a zero output so the
					// on-chain outputs array stays index-aligned with
					// order.output.assets (the gateway skips solverAmount == 0 legs);
					// a compacted array would mismatch and revert fillOrder.
					fillerOutputs.push({ token: output.token, amount: 0n })
					fillerOutputLegs.push(i)
					continue
				}

				const { token0Used, policyMaxOutput: rawPolicyMaxOutput } = legResult
				remainingByPair.set(leg.pair, remaining.minus(token0Used))

				// Overfill detection is warn-only: the clamp is DISABLED, so the filler
				// fills the full computed amount even when it exceeds
				// (1 + maxOverfillBps) × user-requested — including venue-priced legs
				// (e.g. Uniswap V4). NOTE: this removes the per-leg loss bound that
				// previously protected against a bug / stale cache / manipulated venue
				// price. Output is no longer capped; we only emit a warning.
				const overfillCeiling = (output.amount * (10000n + this.maxOverfillBps)) / 10000n
				const policyMaxOutput = rawPolicyMaxOutput
				if (rawPolicyMaxOutput > overfillCeiling) {
					this.logger.warn(
						{
							orderId: order.id,
							leg: i,
							pair: `${leg.pair.token0}/${leg.pair.token1}`,
							token: output.token,
							userRequested: output.amount.toString(),
							unclamped: rawPolicyMaxOutput.toString(),
							ceiling: overfillCeiling.toString(),
							maxOverfillBps: this.maxOverfillBps.toString(),
							priceSource: rates.priceSource,
						},
						"Overfill ceiling exceeded — clamp disabled, filling unclamped amount",
					)
				}

				// Spend the free wallet balance first, down to the configured minBalance
				// reserve — kept liquid for the gas/paymaster pull during
				// validatePaymasterUserOp — then source any remaining shortfall from the
				// funding venues (the vault).
				const tokenAddress = bytes32ToBytes20(output.token).toLowerCase()
				const balance = await this.getAndCacheBalance(tokenAddress, walletAddress, destClient, balanceCache)

				let reserve = 0n
				for (const venue of this.fundingVenues) {
					reserve += venue.walletReserveForToken(destChain, tokenAddress)
				}
				const usableWallet = balance > reserve ? balance - reserve : 0n

				const walletContribution = policyMaxOutput < usableWallet ? policyMaxOutput : usableWallet

				let credited = 0n
				let needed = policyMaxOutput - walletContribution
				for (const venue of this.fundingVenues) {
					if (needed <= 0n) break
					const planned = await venue.planWithdrawalForToken(destChain, walletAddress, tokenAddress, needed, deadlineTimestamp)
					if (planned.calls.length > 0) {
						fundingCalls.push(...planned.calls)
						credited += planned.credited
						needed -= planned.credited
					}
				}

				const effectiveBalance = walletContribution + credited

				const finalOutputAmount = effectiveBalance > policyMaxOutput ? policyMaxOutput : effectiveBalance

				if (finalOutputAmount === 0n) {
					this.logger.info(
						{
							orderId: order.id,
							token: output.token,
							fillerBalance: balance.toString(),
						},
						"Skipping leg: no available balance for required output token",
					)
					// Aligned zero output (see budget-exhausted case above).
					fillerOutputs.push({ token: output.token, amount: 0n })
					fillerOutputLegs.push(i)
					continue
				}

				if (policyMaxOutput < output.amount) {
					this.logger.info(
						{
							orderId: order.id,
							pair: `${leg.pair.token0}/${leg.pair.token1}`,
							token: output.token,
							policyOutput: policyMaxOutput.toString(),
							userRequested: output.amount.toString(),
						},
						"Skipping order: filler price yields less than user's requested amount",
					)
					return 0
				}

				if (sourceChain !== destChain && finalOutputAmount < output.amount) {
					this.logger.info(
						{
							orderId: order.id,
							token: output.token,
							fillerBalance: balance.toString(),
							userRequested: output.amount.toString(),
						},
						"Skipping cross-chain order: insufficient balance for full fill",
					)
					return 0
				}

				// Decrement the wallet pool by what this leg drew from it (vault-sourced
				// tokens are tracked by the venue's own reservations) so repeated outputs
				// of the same token share one wallet balance.
				const walletRemaining = balance - walletContribution
				balanceCache.set(tokenAddress, walletRemaining > 0n ? walletRemaining : 0n)

				fillerOutputs.push({ token: output.token, amount: finalOutputAmount })
				fillerOutputLegs.push(i)
			}

			if (fillerOutputs.every((o) => o.amount === 0n)) {
				this.logger.info(
					{
						orderId: order.id,
						orderNotional: totalNotional.toString(),
					},
					"Skipping order: no fillable outputs after applying pair caps and balance constraints",
				)
				return 0
			}

			this.contractService.cacheService.setFillerOutputs(order.id!, fillerOutputs)

			if (order.id) {
				if (fundingCalls.length > 0) {
					this.contractService.cacheService.setFundingPrepends(order.id, fundingCalls)
				} else {
					this.contractService.cacheService.clearFundingPrepends(order.id)
				}
			}

			// Per-leg P&L accounting over the surviving legs, feeding the SPREAD gate
			// (the filler's margin on the swap itself, independent of order.fees):
			//  - Same-token legs realize their spread in-kind (input − output of the
			//    SAME asset), in fee-token (USD) units — deterministic.
			//  - Cross-asset legs are half a round-trip: the open side is marked at
			//    the opposite curve (sells token1 → rebuy at bid; buys token1 →
			//    resale at ask), in token0 units. Only two-sided legs contribute; a
			//    one-sided (directional) leg has no opposite curve to mark against.
			let realizedSpreadProfit = 0n
			let fxMarginQuote = new Decimal(0)
			let hasSameTokenSpread = false
			// Every same-token leg must individually come out ahead in its OWN asset.
			// Checked per leg (not on the summed total) because same-token legs of
			// different assets aren't in a common unit.
			let sameTokenAllProfitable = true
			let hasFxMargin = false
			for (let i = 0; i < fillerOutputs.length; i++) {
				const legIndex = fillerOutputLegs[i]
				const input = order.inputs[legIndex]
				const output = fillerOutputs[i]
				const leg = legs[legIndex]

				// Zero-amount legs are placeholders keeping the outputs array aligned;
				// they release no escrow on-chain, so they contribute nothing to P&L.
				if (output.amount === 0n) continue

				const inputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(input.token) as HexString,
					sourceChain,
				)
				const outputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(output.token) as HexString,
					destChain,
				)

				if (isSameTokenPair(leg.pair)) {
					// Spread in the asset's OWN units: escrow released (full input) minus
					// output paid. Positive iff the filler nets the asset — a sign check
					// that is valid for any asset (USD-stable or not), since it never
					// crosses into another unit.
					const convertedInput = adjustDecimals(input.amount, inputDecimals, outputDecimals)
					const spread = convertedInput - output.amount
					if (spread <= 0n) sameTokenAllProfitable = false
					realizedSpreadProfit += adjustDecimals(spread, outputDecimals, feeTokenDecimals)
					hasSameTokenSpread = true
					continue
				}

				const rates = legRatesByIndex.get(legIndex)
				if (!rates?.oppositeRate) continue
				hasFxMargin = true

				const token0Decimals = leg.inputIsToken0 ? inputDecimals : outputDecimals
				const token1Decimals = leg.inputIsToken0 ? outputDecimals : inputDecimals

				if (leg.inputIsToken0) {
					// Sells token1: receives token0, gives token1 valued at bid (rebuy cost).
					const inputToken0 = new Decimal(formatUnits(input.amount, token0Decimals))
					const outputToken1 = new Decimal(formatUnits(output.amount, token1Decimals))
					fxMarginQuote = fxMarginQuote.plus(inputToken0.minus(outputToken1.div(rates.oppositeRate)))
				} else {
					// Buys token1: gives token0, receives token1 valued at ask (resale value).
					const inputToken1 = new Decimal(formatUnits(input.amount, token1Decimals))
					const outputToken0 = new Decimal(formatUnits(output.amount, token0Decimals))
					fxMarginQuote = fxMarginQuote.plus(inputToken1.div(rates.oppositeRate).minus(outputToken0))
				}
			}

			// Clamp is disabled, so a leg can never be clamped — the halt subsystem is
			// left in place but dormant (always recorded as a clean, unclamped outcome).
			this.recordOrderOutcome(false, order.id)

			const { totalCostInSourceFeeToken, relayerFeeInSourceFeeToken } =
				await this.contractService.estimateGasFillPost(order)

			// GATE 1 — execution cost (independent). order.fees exist solely to pay
			// for execution: the fill gas plus, for cross-chain orders, the relayer
			// fee for delivering the escrow-release message back to the source chain
			// (RELAYER_MESSAGE_GAS priced on the source chain; 0 for same-chain). The
			// swap spread is NOT credited here — fees must cover cost on their own.
			const executionCost = totalCostInSourceFeeToken + relayerFeeInSourceFeeToken
			if (order.fees < executionCost) {
				this.logger.info(
					{
						orderId: order.id,
						orderFees: formatUnits(order.fees, feeTokenDecimals),
						fillGas: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
						relayerFee: formatUnits(relayerFeeInSourceFeeToken, feeTokenDecimals),
						executionCost: formatUnits(executionCost, feeTokenDecimals),
					},
					"Skipping order: attached fees do not cover execution cost (fill gas + relayer fee)",
				)
				return 0
			}

			// GATE 2 — swap profit (independent). The fill must make the filler money
			// on the swap itself, measured per category present. One-sided (directional)
			// legs produce no spread signal and are not gated here — the operator opted
			// into that position by configuring one-sided pricing.
			if (hasSameTokenSpread && !sameTokenAllProfitable) {
				this.logger.info(
					{ orderId: order.id, realizedSpreadProfit: formatUnits(realizedSpreadProfit, feeTokenDecimals) },
					"Skipping order: a same-token leg does not net a positive spread",
				)
				return 0
			}
			if (hasFxMargin && fxMarginQuote.lte(0)) {
				this.logger.info(
					{ orderId: order.id, fxMarginQuote: fxMarginQuote.toString() },
					"Skipping order: FX spread margin is not positive",
				)
				return 0
			}

			const feeProfit = order.fees - executionCost
			// Both gates passed → the order is profitable. This number is only the
			// ranking / >0 execute signal, never a funds gate (the two gates above
			// already decided). It sums fee surplus (USD) with the realized same-token
			// spread and the cross-asset FX margin — for a non-USD same-token asset
			// the spread term is in that asset's units, so the magnitude is a rough
			// signal rather than a true dollar figure; its sign is always correct.
			const totalProfit =
				parseFloat(formatUnits(feeProfit + realizedSpreadProfit, feeTokenDecimals)) + fxMarginQuote.toNumber()

			this.logger.info(
				{
					orderId: order.id,
					sourceChain,
					destChain,
					crossChain: sourceChain !== destChain,
					pairs: legs.map((leg) => `${leg.pair.token0}/${leg.pair.token1}`),
					orderNotional: totalNotional.toString(),
					legNotionals: legNotionals.map((n) => n.toString()),
					pairCaps: [...cappedByPair].map(
						([pair, cap]) => `${pair.token0}/${pair.token1}=${cap.toString()}`,
					),
					orderFees: formatUnits(order.fees, feeTokenDecimals),
					fillGas: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
					relayerFee: formatUnits(relayerFeeInSourceFeeToken, feeTokenDecimals),
					executionCost: formatUnits(executionCost, feeTokenDecimals),
					feeProfit: formatUnits(feeProfit, feeTokenDecimals),
					realizedSpreadProfit: formatUnits(realizedSpreadProfit, feeTokenDecimals),
					fxMarginQuote: fxMarginQuote.toString(),
					totalProfit,
					profitable: totalProfit > 0,
				},
				"FX swap profitability evaluation",
			)

			return totalProfit
		} catch (error) {
			this.logger.error({ err: error }, "Error calculating profitability")
			return 0
		}
	}

	/**
	 * Executes an order by submitting a bid via the IntentsCoprocessor.
	 *
	 * Assumes that `calculateProfitability` has already been called for the
	 * given order so that filler outputs are cached in `contractService`.
	 * This method only orchestrates the bid construction and submission; the
	 * actual token movements are handled on-chain by the IntentGateway.
	 */
	async executeOrder(order: Order, intentsCoprocessor?: IntentsCoprocessor): Promise<ExecutionResult> {
		const startTime = Date.now()

		try {
			if (!intentsCoprocessor) {
				return {
					success: false,
					error: "FXFiller requires the UserOp/Hyperbridge path (intentsCoprocessor must be provided)",
				}
			}

			return await this.submitBid(order, startTime, intentsCoprocessor)
		} catch (error) {
			this.logger.error({ err: error }, "Error executing FX swap order")
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	// =========================================================================
	// Private — Execution
	// =========================================================================

	/**
	 * Prepares and submits a bid UserOp to Hyperbridge for the given order.
	 *
	 * Uses the filler outputs previously cached by `calculateProfitability`.
	 * Approval bundling and UserOp construction are handled by
	 * `ContractInteractionService.prepareBidUserOp`.
	 */
	private async submitBid(
		order: Order,
		startTime: number,
		intentsCoprocessor: IntentsCoprocessor,
	): Promise<ExecutionResult> {
		const entryPointAddress = this.configService.getEntryPointAddress(order.destination)
		if (!entryPointAddress) {
			return {
				success: false,
				error: `EntryPoint not configured for chain ${order.destination}`,
			}
		}

		const solverAccountAddress = this.signer.account.address as HexString

		// Prepare the signed UserOp for bid submission (bundles approvals + fillOrder internally)
		const { commitment, userOp } = await this.contractService.prepareBidUserOp(
			order,
			entryPointAddress,
			solverAccountAddress,
		)

		const bidResult = await intentsCoprocessor.submitBid(commitment, userOp)

		const endTime = Date.now()
		if (bidResult.success) {
			this.logger.info({ commitment }, "Bid submitted successfully")
			return {
				success: true,
				txHash: bidResult.extrinsicHash,
				strategyUsed: this.name,
				processingTimeMs: endTime - startTime,
				commitment,
			}
		}

		this.logger.error({ commitment, error: bidResult.error }, "Bid submission failed")
		return { success: false, error: bidResult.error, commitment }
	}

	// =========================================================================
	// Private — Helpers
	// =========================================================================

	/**
	 * Update consecutive-clamp counter after a successful order evaluation.
	 * Only venue-priced legs feed this counter (see clamp site) — a streak of those
	 * is the signal that a live market source has gone off (stale pool, manipulated
	 * venue) and the filler should stop until an operator investigates. Offline
	 * price-curve clamps warn but never reach here.
	 */
	private recordOrderOutcome(clamped: boolean, orderId: string | undefined) {
		if (clamped) {
			this.consecutiveClamps += 1
			if (this.consecutiveClamps >= this.maxConsecutiveClamps) {
				this.halted = true
				this.logger.error(
					{ orderId, consecutiveClamps: this.consecutiveClamps, maxConsecutiveClamps: this.maxConsecutiveClamps },
					"FXFiller HALTED — venue-priced overfill clamp triggered consecutively; restart required after investigation",
				)
			}
		} else {
			this.consecutiveClamps = 0
		}
	}

	/**
	 * Estimates each leg's token0 notional and derives per-pair budgets.
	 *
	 * The estimate uses the leg's own price source at minimum size (curve at 0,
	 * or the venue quote) to convert token1-input legs into token0 terms. Each
	 * pair's total is capped at its `maxOrderSize` — that capped notional is
	 * the point pair curves are evaluated at and the budget its legs draw from.
	 *
	 * Returns null when a leg cannot be estimated (no usable rate).
	 */
	private async sizeOrder(
		order: Order,
		legs: ResolvedLeg[],
		venueUsdPrice: (chain: string, token1Address: string) => Promise<Decimal | null>,
	): Promise<{ legNotionals: Decimal[]; cappedByPair: Map<TradingPair, Decimal>; totalNotional: Decimal } | null> {
		const sourceChain = order.source
		const legNotionals: Decimal[] = []
		const totals = new Map<TradingPair, Decimal>()

		for (let i = 0; i < order.inputs.length; i++) {
			const leg = legs[i]
			const decimals = await this.contractService.getTokenDecimals(
				bytes32ToBytes20(order.inputs[i].token) as HexString,
				sourceChain,
			)
			const amount = new Decimal(formatUnits(order.inputs[i].amount, decimals))

			let notional: Decimal
			if (leg.inputIsToken0) {
				notional = amount
			} else {
				const rate = await this.referenceRate(leg, venueUsdPrice)
				if (!rate) return null
				notional = amount.div(rate)
			}
			legNotionals.push(notional)
			totals.set(leg.pair, (totals.get(leg.pair) ?? new Decimal(0)).plus(notional))
		}

		const cappedByPair = new Map<TradingPair, Decimal>()
		// totalNotional is log-only: for an order whose legs span pairs with
		// different token0s it mixes units — anything decision-making must use
		// legNotionals (per-pair token0) or getOrderUsdValue (USD).
		let totalNotional = new Decimal(0)
		for (const [pair, total] of totals) {
			cappedByPair.set(pair, Decimal.min(total, pair.maxOrderSize))
			totalNotional = totalNotional.plus(total)
		}

		return { legNotionals, cappedByPair, totalNotional }
	}

	/**
	 * Minimum-size reference rate (token1 per token0) for a leg's pair: the
	 * bid curve at 0 (the side token1-input legs trade at), falling back to the
	 * ask curve, then the live venue quote for venue-priced pairs.
	 */
	private async referenceRate(
		leg: ResolvedLeg,
		venueUsdPrice: (chain: string, token1Address: string) => Promise<Decimal | null>,
	): Promise<Decimal | null> {
		const policy = leg.pair.bidPricePolicy ?? leg.pair.askPricePolicy
		if (policy) {
			const rate = policy.getPrice(new Decimal(0))
			return rate.gt(0) ? rate : null
		}
		// Venue-priced pair: token0 is USD-stable (constructor invariant), so the
		// venue's USD-per-token1 quote inverts straight into token1-per-token0.
		const venueUsd = await venueUsdPrice(leg.token1Chain, leg.token1Address)
		return venueUsd ? new Decimal(1).div(venueUsd) : null
	}

	/**
	 * Given a single (input, output) leg and the remaining token0 budget of its
	 * pair, computes how much token0 notional to allocate to this leg and the
	 * corresponding maximum output amount at the pair's rate.
	 *
	 * `rate` is **token1 per 1 token0**:
	 * - token0 input → token1 output: token0 × rate → token1 amount.
	 * - token1 input → token0 output: token1 ÷ rate → token0 amount.
	 *
	 * Returns `null` when this leg cannot consume any of the pair's remaining
	 * budget (e.g. the cap has already been exhausted).
	 */
	private computeLegPolicyOutput(
		inputAmount: bigint,
		inputIsToken0: boolean,
		token0Decimals: number,
		token1Decimals: number,
		remainingToken0: Decimal,
		rate: Decimal,
	): { token0Used: Decimal; policyMaxOutput: bigint } | null {
		let legMaxToken0: Decimal
		if (inputIsToken0) {
			legMaxToken0 = new Decimal(formatUnits(inputAmount, token0Decimals))
		} else {
			legMaxToken0 = new Decimal(formatUnits(inputAmount, token1Decimals)).div(rate)
		}

		const token0ForLeg = Decimal.min(legMaxToken0, remainingToken0)
		if (token0ForLeg.lte(0)) {
			return null
		}

		let policyMaxOutput: bigint
		if (inputIsToken0) {
			// Output is token1: convert the token0 allocation at the pair rate.
			policyMaxOutput = BigInt(
				token0ForLeg.mul(rate).mul(new Decimal(10).pow(token1Decimals)).floor().toFixed(0),
			)
		} else {
			// Output is token0: pay out the token0 equivalent of the token1 input.
			policyMaxOutput = BigInt(token0ForLeg.mul(new Decimal(10).pow(token0Decimals)).floor().toFixed(0))
		}

		return { token0Used: token0ForLeg, policyMaxOutput }
	}

	/**
	 * Resolves the pricing rate (token1 per token0) for a leg: the venue quote
	 * when available (validated against the price guard; USD-stable token0
	 * pairs only), otherwise the pair's curve for the leg's direction at the
	 * pair's capped token0 notional. Returns null when the leg cannot be priced
	 * (guard tripped, or direction disabled).
	 */
	private async resolveLegRates(
		orderId: string | undefined,
		leg: ResolvedLeg,
		cappedPairNotional: Decimal,
		venueUsdPrice: (chain: string, token1Address: string) => Promise<Decimal | null>,
	): Promise<LegRates | null> {
		// Explicitly configured curves always win — the venue only prices pairs
		// with no curves at all (and never same-token pairs, where a venue quote
		// would just be the asset's own USD price, not a spread).
		const curveless = !leg.pair.bidPricePolicy && !leg.pair.askPricePolicy
		if (curveless && !isSameTokenPair(leg.pair) && USD_STABLE_SYMBOLS.has(normalizeSymbol(leg.pair.token0))) {
			const venueUsd = await venueUsdPrice(leg.token1Chain, leg.token1Address)
			if (venueUsd) {
				// Guard compares the venue's token1-per-USD quote against the static reference.
				if (!this.checkPriceGuard(orderId, leg.token1Chain, new Decimal(1).div(venueUsd))) {
					return null
				}
				// The pool mid is used for both directions, mirroring the previous
				// venue behaviour.
				const venueRate = new Decimal(1).div(venueUsd)
				return { rate: venueRate, oppositeRate: venueRate, priceSource: "venue" }
			}
		}

		const askRate = leg.pair.askPricePolicy?.getPrice(cappedPairNotional) ?? null
		const bidRate = leg.pair.bidPricePolicy?.getPrice(cappedPairNotional) ?? null

		const rate = leg.inputIsToken0 ? askRate : bidRate
		if (!rate) {
			this.logger.debug(
				{ orderId, pair: `${leg.pair.token0}/${leg.pair.token1}`, inputIsToken0: leg.inputIsToken0 },
				"Rejecting leg: direction disabled for one-sided LP",
			)
			return null
		}
		return { rate, oppositeRate: leg.inputIsToken0 ? bidRate : askRate, priceSource: "policy" }
	}

	/**
	 * Reads and caches the filler's balance for a token on the destination chain.
	 *
	 * Normalizes the token address, checks an in-memory cache, and only hits
	 * the chain (native `getBalance` or ERC20 `balanceOf`) on a cache miss.
	 * This allows multiple legs within a single profitability evaluation to
	 * share the same balance pool.
	 */
	private async getAndCacheBalance(
		tokenAddressLower: string,
		walletAddress: HexString,
		// biome-ignore lint/suspicious/noExplicitAny: viem public client type varies per chain
		destClient: any,
		balanceCache: Map<string, bigint>,
	): Promise<bigint> {
		const key = tokenAddressLower.toLowerCase()
		const cached = balanceCache.get(key)
		if (cached !== undefined) {
			return cached
		}

		let balance: bigint
		if (key === ADDRESS_ZERO.toLowerCase()) {
			balance = await destClient.getBalance({ address: walletAddress })
		} else {
			balance = await destClient.readContract({
				abi: ERC20_ABI,
				address: key as HexString,
				functionName: "balanceOf",
				args: [walletAddress],
			})
		}

		balanceCache.set(key, balance)
		return balance
	}

	/**
	 * Matches an (input, output) leg to a configured pair on the order's chains.
	 *
	 * A leg matches when input = token0 on source and output = token1 on dest
	 * (the filler sells token1 — ask direction), or input = token1 on source and
	 * output = token0 on dest (the filler buys token1 — bid direction). The
	 * direction must also be enabled for the pair (one-sided LP).
	 */
	private matchLeg(
		sourceChain: string,
		destChain: string,
		inputAddress: string,
		outputAddress: string,
	): ResolvedLeg | null {
		for (const pair of this.pairs) {
			// Same-token pairs are the same-asset CROSS-chain market only. A
			// same-chain leg (source == dest) would be a pay-more-get-less self
			// swap on one chain — never fill it.
			if (isSameTokenPair(pair) && sourceChain === destChain) continue

			const token0Source = this.registry.getAddress(pair.token0, sourceChain)?.toLowerCase()
			const token1Dest = this.registry.getAddress(pair.token1, destChain)?.toLowerCase()
			if (token0Source && token1Dest && inputAddress === token0Source && outputAddress === token1Dest) {
				if (!this.directionEnabled(pair, true)) continue
				return {
					pair,
					inputIsToken0: true,
					token1Address: token1Dest,
					token1Chain: destChain,
				}
			}

			// Same-token pairs have identical addresses on both branches — every
			// matching leg is the ask direction, so the bid branch never applies.
			if (isSameTokenPair(pair)) continue

			const token1Source = this.registry.getAddress(pair.token1, sourceChain)?.toLowerCase()
			const token0Dest = this.registry.getAddress(pair.token0, destChain)?.toLowerCase()
			if (token1Source && token0Dest && inputAddress === token1Source && outputAddress === token0Dest) {
				if (!this.directionEnabled(pair, false)) continue
				return {
					pair,
					inputIsToken0: false,
					token1Address: token1Source,
					token1Chain: sourceChain,
				}
			}
		}
		return null
	}

	/**
	 * Whether a pair fills legs in the given direction. Curve-priced pairs are
	 * gated by the presence of the direction's curve; venue-priced pairs (no
	 * curves) by the global `side` switch. The IntentGateway settles all legs
	 * atomically, so a mixed-direction order is rejected as a whole when any
	 * leg's direction is disabled.
	 */
	private directionEnabled(pair: TradingPair, inputIsToken0: boolean): boolean {
		const hasCurves = !!(pair.bidPricePolicy || pair.askPricePolicy)
		if (hasCurves) {
			// input token0 → filler sells token1 → needs the ask curve; and vice versa.
			return inputIsToken0 ? !!pair.askPricePolicy : !!pair.bidPricePolicy
		}
		if (this.side) {
			return inputIsToken0 ? this.side === "ask" : this.side === "bid"
		}
		return true
	}

	/**
	 * Resolves every (input, output) leg of an order to a configured pair in one
	 * pass. Returns null if any leg matches no pair (or a disabled direction).
	 *
	 * The address-level classification is cached per order id in the shared
	 * cache (as `CachedPairClassification`, one entry per leg) so repeated
	 * evaluations skip re-derivation; pair resolution itself is re-run per
	 * strategy since pair sets differ between engine instances.
	 */
	private resolveOrderLegs(order: Order): ResolvedLeg[] | null {
		const sourceChain = order.source
		const destChain = order.destination

		const cached = order.id ? this.contractService.cacheService.getPairClassifications(order.id) : null
		if (cached) {
			const legs: ResolvedLeg[] = []
			for (const entry of cached) {
				// Re-derive the leg's input/output addresses from the cached
				// classification, then re-match against THIS engine's pairs.
				const token0Address = bytes32ToBytes20(entry.stableToken).toLowerCase()
				const token1Address = bytes32ToBytes20(entry.exoticToken).toLowerCase()
				const inputAddress = entry.inputIsStable ? token0Address : token1Address
				const outputAddress = entry.inputIsStable ? token1Address : token0Address
				const leg = this.matchLeg(sourceChain, destChain, inputAddress, outputAddress)
				if (!leg) return null
				legs.push(leg)
			}
			return legs
		}

		const legs: ResolvedLeg[] = []
		const classifications: CachedPairClassification[] = []
		for (let i = 0; i < order.inputs.length; i++) {
			const inputAddress = bytes32ToBytes20(order.inputs[i].token).toLowerCase()
			const outputAddress = bytes32ToBytes20(order.output.assets[i].token).toLowerCase()

			const leg = this.matchLeg(sourceChain, destChain, inputAddress, outputAddress)
			if (!leg) return null
			legs.push(leg)
			classifications.push({
				inputIsStable: leg.inputIsToken0,
				stableToken: leg.inputIsToken0 ? order.inputs[i].token : order.output.assets[i].token,
				exoticToken: leg.inputIsToken0 ? order.output.assets[i].token : order.inputs[i].token,
			})
		}

		if (order.id) {
			this.contractService.cacheService.setPairClassifications(order.id, classifications)
		}

		return legs
	}

	/**
	 * Returns the filler's proposed output amounts for a phantom order without
	 * checking on-chain balance or estimating gas. Phantom orders are probes that
	 * never execute; we only need the price signal.
	 *
	 * Returns `null` when no pair matches or the legs cannot be sized (e.g.
	 * venue price unavailable and no fallback).
	 */
	async quotePhantomFill(order: Order): Promise<TokenInfo[] | null> {
		if (!(await this.canFill(order))) return null

		const legs = this.resolveOrderLegs(order)
		if (!legs) return null

		const chain = order.source
		const venueUsdPrice = this.venuePriceMemo()

		const sized = await this.sizeOrder(order, legs, venueUsdPrice)
		if (!sized) return null
		const remainingByPair = new Map(sized.cappedByPair)

		const outputs: TokenInfo[] = []

		for (let i = 0; i < order.inputs.length; i++) {
			const input = order.inputs[i]
			const output = order.output.assets[i]
			const leg = legs[i]

			const inputDecimals = await this.contractService.getTokenDecimals(
				bytes32ToBytes20(input.token) as HexString,
				chain,
			)
			// Phantom orders are same-chain probes today, but resolve the output on
			// the destination anyway — decimals differ per chain for some assets.
			const outputDecimals = await this.contractService.getTokenDecimals(
				bytes32ToBytes20(output.token) as HexString,
				order.destination,
			)

			const token0Decimals = leg.inputIsToken0 ? inputDecimals : outputDecimals
			const token1Decimals = leg.inputIsToken0 ? outputDecimals : inputDecimals

			const cappedNotional = sized.cappedByPair.get(leg.pair) ?? leg.pair.maxOrderSize
			const rates = await this.resolveLegRates(order.id, leg, cappedNotional, venueUsdPrice)
			if (!rates) return null

			const remaining = remainingByPair.get(leg.pair) ?? new Decimal(0)
			const legResult = this.computeLegPolicyOutput(
				input.amount,
				leg.inputIsToken0,
				token0Decimals,
				token1Decimals,
				remaining,
				rates.rate,
			)

			if (!legResult) continue

			remainingByPair.set(leg.pair, remaining.minus(legResult.token0Used))

			// Phantom orders only probe price (they request a zero output), so there is no
			// user-requested amount to cap against — quote the full policy output.
			outputs.push({ token: output.token, amount: legResult.policyMaxOutput })
		}

		if (outputs.length === 0) return null

		if (order.id) {
			this.contractService.cacheService.setFillerOutputs(order.id, outputs)
		}

		return outputs
	}

	/**
	 * Returns the order's input basket in **USD** — each leg is sized to its
	 * pair's token0 notional via the pair's own reference rate (curve at
	 * minimum size, or the venue quote), converted to dollars through the
	 * curve-derived anchor factor for that token0, and summed across legs.
	 *
	 * The core filler feeds this to the per-chain confirmation curves, whose
	 * `amount` axis is USD — honest for non-USD pairs too, which is the whole
	 * point of the anchor graph. Returns `null` when a leg matches no pair or
	 * cannot be sized (genuine "can't price").
	 */
	async getOrderUsdValue(order: Order): Promise<{ inputUsd: Decimal } | null> {
		const legs = this.resolveOrderLegs(order)
		if (!legs) return null

		const sized = await this.sizeOrder(order, legs, this.venuePriceMemo())
		if (!sized) return null

		// Leg notionals are in each pair's own token0. Convert to USD through
		// the curve graph before summing — the confirmation curve's amount axis
		// is USD, and a raw token quantity would over-wait for sub-dollar
		// assets and, worse, under-wait for anything above a dollar.
		const factors = this.usdFactors()
		let inputUsd = new Decimal(0)
		for (let i = 0; i < legs.length; i++) {
			const factor = factors.get(normalizeSymbol(legs[i].pair.token0))
			// Unreachable after the constructor's anchor check; refuse to size
			// rather than mislabel a token quantity as dollars.
			if (!factor) return null
			inputUsd = inputUsd.plus(sized.legNotionals[i].mul(factor))
		}
		if (inputUsd.lte(0)) return null
		return { inputUsd }
	}

	/**
	 * USD per unit of every priceable symbol, derived from the operator's own
	 * curves: USD stables are $1 anchors and each curve-priced cross-asset
	 * pair's zero-notional mid is an FX edge (token1 per token0). Recomputed
	 * per call so live curve edits through the admin server take effect
	 * immediately; the graph is a handful of pairs, so this is trivial.
	 *
	 * When several pairs could price the same symbol, the **first anchoring
	 * pair in declaration order wins** and later edges are ignored — factors
	 * are never re-derived, which keeps the walk terminating and deterministic
	 * (no averaging across inconsistent routes, no divergence on curve cycles).
	 * USD stables are pinned at $1 and never re-priced through a curve, so a
	 * mis-set stable/stable curve cannot contaminate the anchors. Declare the
	 * pair you want as the reference first if an asset has multiple routes.
	 */
	private usdFactors(): Map<string, Decimal> {
		const factors = new Map<string, Decimal>()
		for (const symbol of USD_STABLE_SYMBOLS) factors.set(symbol, new Decimal(1))

		let grew = true
		while (grew) {
			grew = false
			for (const pair of this.pairs) {
				if (isSameTokenPair(pair)) continue
				const mid = pairMidRate(pair)
				if (!mid) continue
				const token0 = normalizeSymbol(pair.token0)
				const token1 = normalizeSymbol(pair.token1)
				const usd0 = factors.get(token0)
				const usd1 = factors.get(token1)
				if (usd0 && !usd1) {
					// 1 token0 = mid token1 ⇒ usd(token1) = usd(token0) / mid.
					factors.set(token1, usd0.div(mid))
					grew = true
				} else if (usd1 && !usd0) {
					factors.set(token0, usd1.mul(mid))
					grew = true
				}
			}
		}
		return factors
	}
}
