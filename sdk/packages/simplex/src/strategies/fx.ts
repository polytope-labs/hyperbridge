import { FillerStrategy } from "@/strategies/base"
import {
	Order,
	ExecutionResult,
	HexString,
	bytes32ToBytes20,
	type ERC7821Call,
	FillOptions,
	TokenInfo,
	IntentsCoprocessor,
	ADDRESS_ZERO,
} from "@hyperbridge/sdk"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { type CachedPairClassification } from "@/services/CacheService"
import { Decimal } from "decimal.js"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { FundingVenue } from "@/funding/types"
import type { SigningAccount } from "@/services/wallet"

/**
 * Strategy for swaps between USD-pegged stablecoins (USDC/USDT) and a single
 * configurable exotic token priced via a `FillerPricePolicy`.
 * Supports both same-chain and cross-chain orders.
 *
 * The filler holds both the stablecoin(s) and the exotic token. When a user
 * places an order swapping between the two (on the same chain or across
 * different chains), this strategy:
 * 1. Evaluates profitability using the filler's price policy for the exotic token
 * 2. Calls fillOrder to deliver output tokens to the user on the destination chain
 * 3. Receives the user's escrowed input tokens from the source chain contract
 *
 * For cross-chain orders, input tokens are resolved against the source chain's
 * stable/exotic addresses, and output tokens against the destination chain's.
 * The filler's output balance is checked on the destination chain.
 *
 * The filler manages their own internal rebalancing/swaps outside of order execution.
 *
 * This implementation also enforces a per-order USD cap for risk management:
 * - A maximum order USD value is configured on the constructor.
 * - The price policy is always evaluated on the capped USD amount.
 * - The capped USD budget is then allocated across legs in order to determine
 *   how much the filler is willing to output.
 * - Actual outputs are further limited by the filler's real token balances.
 *
 * Because the IntentGateway releases inputs proportionally to the fraction of
 * outputs provided, this allows safe partial fills (and even overfills relative
 * to the user's requested outputs) without additional on-chain logic here.
 */
export class FXFiller implements FillerStrategy {
	name = "FXFiller"
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	/** Bid price policy: exotic tokens per USD when the filler is *buying* exotic from a user */
	private bidPricePolicy: FillerPricePolicy
	/** Ask price policy: exotic tokens per USD when the filler is *selling* exotic to a user */
	private askPricePolicy: FillerPricePolicy
	/** Maps chain identifier → exotic token address (e.g. cNGN on each supported chain) */
	private token1: Record<string, HexString>
	private maxOrderUsd: Decimal
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
	private spreadBps: number
	/**
	 * Optional Uniswap price guard, keyed by chain. When a chain has an entry, a
	 * venue (pool) quote is only trusted if it stays within `maxDeviationBps` of the
	 * static `reference` price (exotic per USD); a quote outside the band rejects the
	 * order — defence against a manipulated, stale, or thin pool. Sourced from the
	 * per-position config under `[strategies.vault.uniswapV4]`.
	 */
	private priceGuard?: Map<string, { reference: Decimal; maxDeviationBps: number }>
	/**
	 * Whether the filler buys exotic from users (exotic-in/stable-out legs), priced
	 * with the bid curve. Disabled by omitting the bid curve — the basis for one-sided
	 * LP: drop a side and the filler skips orders in that direction.
	 */
	private bidEnabled: boolean
	/** Whether the filler sells exotic to users (stable-in/exotic-out legs), priced with the ask curve. */
	private askEnabled: boolean

	/**
	 * @param signer                 Filler's signing account for UserOp signatures.
	 * @param configService          Network/config provider for addresses and decimals.
	 * @param clientManager          Used to get viem PublicClients for chains.
	 * @param contractService        Shared contract interaction service.
	 * @param maxOrderUsd             Maximum USD value this filler is willing to fill per order.
	 * @param token1   Map of chain identifier → exotic token address.
	 * @param options.bidPricePolicy Optional price curve for buying exotic. Required if no fundingVenues.
	 * @param options.askPricePolicy Optional price curve for selling exotic. Required if no fundingVenues.
	 * @param options.confirmationPolicy Optional per-chain confirmation policy for cross-chain orders.
	 * @param options.fundingVenues  Optional funding venues for on-chain liquidity sourcing and live pricing.
	 * @param options.spreadBps      Spread in basis points applied when redeeming from the pool (default 50).
	 *
	 * One-sided LP, two ways depending on the pricing mode:
	 * - Static curves: omit one of `bidPricePolicy`/`askPricePolicy` to fill only the other
	 *   side. Providing both keeps both directions open.
	 * - Venue (pool) pricing with no curves: set `side` to restrict to one direction.
	 *   Omitting `side` keeps both directions open.
	 * @param options.side Pool-pricing one-sided switch ("bid" buys exotic, "ask" sells exotic).
	 *   Only valid with venue pricing and no static curves.
	 */
	constructor(
		signer: SigningAccount,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		maxOrderUsd: number,
		token1: Record<string, HexString>,
		options?: {
			bidPricePolicy?: FillerPricePolicy
			askPricePolicy?: FillerPricePolicy
			confirmationPolicy?: ConfirmationPolicy
			fundingVenues?: FundingVenue[]
			spreadBps?: number
			priceGuard?: Record<string, { referencePrice: string; maxDeviationBps: number }>
			side?: "bid" | "ask"
		},
	) {
		const {
			bidPricePolicy,
			askPricePolicy,
			confirmationPolicy,
			fundingVenues = [],
			spreadBps = 50,
			priceGuard,
			side,
		} = options ?? {}

		const hasAnyPolicy = !!(bidPricePolicy || askPricePolicy)
		const hasVenues = fundingVenues.length > 0

		if (!hasAnyPolicy && !hasVenues) {
			throw new Error("FXFiller requires a bid and/or ask price policy, or funding venues")
		}
		if (side && hasAnyPolicy) {
			throw new Error("FXFiller 'side' only applies to venue (pool) pricing; omit bid/ask price policies")
		}

		// Direction enablement. With static curves, only the side(s) with a curve are filled
		// (one-sided LP). With venue pricing (no curves), `side` optionally restricts to one
		// direction; without it both sides are open.
		this.bidEnabled = hasAnyPolicy ? !!bidPricePolicy : side ? side === "bid" : true
		this.askEnabled = hasAnyPolicy ? !!askPricePolicy : side ? side === "ask" : true

		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.token1 = token1
		this.fundingVenues = fundingVenues
		this.spreadBps = spreadBps
		if (priceGuard && Object.keys(priceGuard).length > 0) {
			this.priceGuard = new Map()
			for (const [chain, guard] of Object.entries(priceGuard)) {
				this.priceGuard.set(chain, {
					reference: new Decimal(guard.referencePrice),
					maxDeviationBps: guard.maxDeviationBps,
				})
			}
		}

		// Absent policies get a placeholder flat curve. A side without a curve is either
		// disabled (so never priced) or venue-priced at runtime, so the placeholder is unused.
		this.bidPricePolicy = bidPricePolicy ?? new FillerPricePolicy({ points: [{ amount: "0", price: "1" }] })
		this.askPricePolicy = askPricePolicy ?? new FillerPricePolicy({ points: [{ amount: "0", price: "1" }] })

		this.maxOrderUsd = new Decimal(maxOrderUsd)
		if (this.maxOrderUsd.lte(0)) {
			throw new Error("FXFiller maxOrderUsd must be greater than 0")
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
	 * Hydrates all funding venue state and derives initial bid/ask prices from venue data.
	 */
	async initialise(): Promise<void> {
		const solver = this.signer.account.address as HexString
		await Promise.all(this.fundingVenues.map((v) => v.initialise(solver)))
	}

	/**
	 * Queries funding venues for the exotic token's USD price on a given chain.
	 * Uniswap V4 is preferred; falls back to other venues. Returns the raw
	 * pool price as both bid and ask, or null if no venue has a price.
	 */
	private async getVenuePrice(chain: string): Promise<{ bid: Decimal; ask: Decimal } | null> {
		const exoticAddr = this.token1[chain]
		if (!exoticAddr) return null

		// Prefer V4, fall back to others
		const v4 = this.fundingVenues.filter((v) => v.name === "UniswapV4")
		const venues = v4.length > 0 ? v4 : this.fundingVenues

		for (const venue of venues) {
			const usdPrice = await venue.getExoticTokenPrice(chain, exoticAddr)
			if (usdPrice && usdPrice.isPositive()) {
				const exoticPerUsd = new Decimal(1).div(usdPrice)
				return {
					bid: exoticPerUsd,
					ask: exoticPerUsd,
				}
			}
		}
		return null
	}

	/**
	 * Validates a live venue quote against the static reference price for the chain.
	 * Returns true (pass) when no guard is configured, or no reference exists for the
	 * chain. Returns false when the quote (exotic per USD) deviates from the reference
	 * by more than `maxDeviationBps`, in which case the order must not be filled.
	 */
	private checkPriceGuard(orderId: string | undefined, chain: string, venueExoticPerUsd: Decimal): boolean {
		const guard = this.priceGuard?.get(chain)
		if (!guard || guard.reference.lte(0)) return true

		const deviationBps = venueExoticPerUsd.minus(guard.reference).abs().div(guard.reference).mul(10000)
		if (deviationBps.gt(guard.maxDeviationBps)) {
			this.logger.warn(
				{
					orderId,
					chain,
					venuePrice: venueExoticPerUsd.toString(),
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

			const pairs = this.classifyAllPairs(order)
			if (!pairs) {
				this.logger.debug({ sourceChain: order.source, destChain: order.destination }, "Unsupported token pair")
				return false
			}

			if (!this.isOrderDirectionEnabled(pairs, order.id)) {
				return false
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Error in canFill")
			return false
		}
	}

	/**
	 * Evaluates whether an order is profitable to fill under the configured
	 * per-order USD cap and the filler's current token balances.
	 *
	 * High-level flow:
	 * - Compute the total USD value of the order based on the input side,
	 *   pricing exotic inputs at the policy's minimum price.
	 * - Cap this at `maxOrderUsd` to get a capped USD budget.
	 * - Ask the price policy for an exotic token price at that capped USD.
	 * - Walk each (input, output) leg in order, allocating from the capped USD
	 *   budget and computing how much the filler is willing to output.
	 * - Further cap each leg by the filler's current token balance.
	 * - Cache the resulting outputs for later use in `executeOrder`.
	 *
	 * Note: we may intentionally overfill relative to the user's requested
	 * outputs if the price policy makes that attractive. This is how we stay competitive.
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

			const pairs = this.classifyAllPairs(order)
			if (!pairs) {
				this.logger.info({ orderId: order.id }, "Skipping order: could not classify token pairs")
				return 0
			}

			if (!this.isOrderDirectionEnabled(pairs, order.id)) {
				return 0
			}

			const usdResult = await this.getOrderUsdValue(order)
			const totalInputUsd = usdResult?.inputUsd

			if (!totalInputUsd || totalInputUsd.lte(0)) {
				this.logger.info({ orderId: order.id }, "Skipping order: could not compute input USD value")
				return 0
			}

			const cappedOrderUsd = Decimal.min(totalInputUsd, this.maxOrderUsd)
			if (cappedOrderUsd.lte(0)) {
				this.logger.info(
					{
						orderId: order.id,
						orderValueUsdFull: totalInputUsd.toString(),
						orderValueUsdCapped: cappedOrderUsd.toString(),
						maxOrderUsd: this.maxOrderUsd.toString(),
					},
					"Skipping order: capped USD value is non-positive",
				)
				return 0
			}

			// Compute bid and ask prices at the capped order size once, then pick per leg.
			// - askPrice: used when filler sells exotic (stable->exotic). Lower rate = fewer exotic sent.
			// - bidPrice: used when filler buys exotic (exotic->stable). Higher rate = fewer USD paid out.
			const policyBidPrice = this.bidPricePolicy.getPrice(cappedOrderUsd)
			const policyAskPrice = this.askPricePolicy.getPrice(cappedOrderUsd)
			const fillerOutputs: TokenInfo[] = []
			// Original leg index for each entry in `fillerOutputs`. Legs can be skipped
			// (insufficient balance, exhausted budget), so `fillerOutputs[k]` is the k-th
			// *surviving* leg, not the k-th leg. The valuation pass below realigns to the
			// original input/pair via this array rather than by position.
			const fillerOutputLegs: number[] = []
			let remainingUsd = cappedOrderUsd

			const fundingCalls: ERC7821Call[] = []

			// Fetch venue prices once per chain (avoids redundant RPC per leg)
			const sourceVenuePrice = this.fundingVenues.length > 0 ? await this.getVenuePrice(sourceChain) : null
			const destVenuePrice = sourceChain !== destChain && this.fundingVenues.length > 0
				? await this.getVenuePrice(destChain) : sourceVenuePrice

			// Price guard: reject the whole order if a venue quote on any involved chain
			// has drifted beyond the configured band from its static reference price.
			if (sourceVenuePrice && !this.checkPriceGuard(order.id, sourceChain, sourceVenuePrice.bid)) {
				return 0
			}
			if (sourceChain !== destChain && destVenuePrice && !this.checkPriceGuard(order.id, destChain, destVenuePrice.bid)) {
				return 0
			}

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
				const pair = pairs[i]

				const inputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(input.token) as HexString,
					sourceChain,
				)
				const outputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(output.token) as HexString,
					destChain,
				)

				const stableDecimals = pair.inputIsStable ? inputDecimals : outputDecimals
				const exoticTokenDecimals = pair.inputIsStable ? outputDecimals : inputDecimals

				const venuePrice = pair.inputIsStable ? destVenuePrice : sourceVenuePrice
				const bidPrice = venuePrice?.bid ?? policyBidPrice
				const askPrice = venuePrice?.ask ?? policyAskPrice

				const legResult = this.computeLegPolicyOutput(
					input.amount,
					pair.inputIsStable,
					stableDecimals,
					exoticTokenDecimals,
					remainingUsd,
					pair.inputIsStable ? askPrice : bidPrice,
				)

				if (!legResult) {
					continue
				}

				const { usdUsed, policyMaxOutput: rawPolicyMaxOutput } = legResult
				remainingUsd = remainingUsd.minus(usdUsed)

				// Overfill detection is warn-only: the clamp is DISABLED, so the filler
				// fills the full computed amount even when it exceeds
				// (1 + maxOverfillBps) × user-requested — including venue-priced legs
				// (e.g. Uniswap V4). NOTE: this removes the per-leg loss bound that
				// previously protected against a bug / stale cache / manipulated venue
				// price. Output is no longer capped; we only emit a warning.
				const overfillCeiling = (output.amount * (10000n + this.maxOverfillBps)) / 10000n
				const policyMaxOutput = rawPolicyMaxOutput
				if (rawPolicyMaxOutput > overfillCeiling) {
					const priceSource = venuePrice ? "venue" : "policy"
					this.logger.warn(
						{
							orderId: order.id,
							leg: i,
							token: output.token,
							userRequested: output.amount.toString(),
							unclamped: rawPolicyMaxOutput.toString(),
							ceiling: overfillCeiling.toString(),
							maxOverfillBps: this.maxOverfillBps.toString(),
							priceSource,
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
					continue
				}

				if (policyMaxOutput < output.amount) {
					this.logger.info(
						{
							orderId: order.id,
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

				if (remainingUsd.lte(0)) {
					break
				}
			}

			if (fillerOutputs.length === 0) {
				this.logger.info(
					{
						orderId: order.id,
						orderValueUsdFull: totalInputUsd.toString(),
						orderValueUsdCapped: cappedOrderUsd.toString(),
						maxOrderUsd: this.maxOrderUsd.toString(),
					},
					"Skipping order: no outputs after applying USD cap and balance constraints",
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

			// Realized FX margin, report-only — never rejects an order. A single fill is half a
			// round-trip, so the open leg is marked at the opposite side of the spread:
			// - sells exotic (stable→exotic): value the exotic given at bid (rebuy cost).
			// - buys exotic (exotic→stable): value the exotic received at ask (resale value).
			// Positive by construction when bid ≥ ask. `fillerOutputs[i]` is the i-th *surviving*
			// leg; realign to its original input/pair via `fillerOutputLegs`.
			let fxMarginUsd = new Decimal(0)
			for (let i = 0; i < fillerOutputs.length; i++) {
				const legIndex = fillerOutputLegs[i]
				const input = order.inputs[legIndex]
				const output = fillerOutputs[i]
				const pair = pairs[legIndex]

				const inputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(input.token) as HexString,
					sourceChain,
				)
				const outputDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(output.token) as HexString,
					destChain,
				)
				const stableDecimals = pair.inputIsStable ? inputDecimals : outputDecimals
				const exoticDecimalsLeg = pair.inputIsStable ? outputDecimals : inputDecimals

				const venuePriceProfit = pair.inputIsStable ? destVenuePrice : sourceVenuePrice
				const bidPrice = venuePriceProfit?.bid ?? policyBidPrice
				const askPrice = venuePriceProfit?.ask ?? policyAskPrice

				if (pair.inputIsStable) {
					// Sells exotic: receives stable, gives exotic valued at bid (rebuy cost).
					const inputUsd = new Decimal(formatUnits(input.amount, stableDecimals))
					const outputExotic = new Decimal(formatUnits(output.amount, exoticDecimalsLeg))
					fxMarginUsd = fxMarginUsd.plus(inputUsd.minus(outputExotic.div(bidPrice)))
				} else {
					// Buys exotic: gives stable, receives exotic valued at ask (resale value).
					const inputExotic = new Decimal(formatUnits(input.amount, exoticDecimalsLeg))
					const outputUsd = new Decimal(formatUnits(output.amount, stableDecimals))
					fxMarginUsd = fxMarginUsd.plus(inputExotic.div(askPrice).minus(outputUsd))
				}
			}

			// Clamp is disabled, so a leg can never be clamped — the halt subsystem is
			// left in place but dormant (always recorded as a clean, unclamped outcome).
			this.recordOrderOutcome(false, order.id)

			const { totalCostInSourceFeeToken } = await this.contractService.estimateGasFillPost(order)
			// Reject only when the user's attached fees can't cover what we expect to spend on the fill.
			if (order.fees < totalCostInSourceFeeToken) {
				this.logger.info(
					{
						orderId: order.id,
						orderFees: formatUnits(order.fees, feeTokenDecimals),
						estimatedCost: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
					},
					"Skipping order: attached fees do not cover estimated execution cost",
				)
				return 0
			}
			const feeProfit = order.fees - totalCostInSourceFeeToken
			// FX bids are gated on fee profit only. fxMarginUsd is a theoretical mark-to-model
			// value (open leg priced at the opposite curve) and is reported separately, never summed in.
			const totalProfit = parseFloat(formatUnits(feeProfit, feeTokenDecimals))

			this.logger.info(
				{
					orderId: order.id,
					sourceChain,
					destChain,
					crossChain: sourceChain !== destChain,
					orderValueUsdFull: totalInputUsd.toString(),
					orderValueUsdCapped: cappedOrderUsd.toString(),
					maxOrderUsd: this.maxOrderUsd.toString(),
					bidPrice: policyBidPrice.toString(),
					askPrice: policyAskPrice.toString(),
					orderFees: formatUnits(order.fees, feeTokenDecimals),
					estimatedFees: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
					feeProfit: formatUnits(feeProfit, feeTokenDecimals),
					fxMarginUsd: fxMarginUsd.toString(),
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
	 * Given a single (input, output) leg and the remaining capped USD budget,
	 * computes how much USD to allocate to this leg and the corresponding
	 * maximum output amount according to the price policy.
	 *
	 * Uses `exoticPerUsd` (exotic tokens per 1 USD) consistently for both directions:
	 * - Stable input → exotic output: USD × exoticPerUsd → exotic amount.
	 * - Exotic input → stable output: exoticAmount / exoticPerUsd → USD.
	 *
	 * Returns `null` when this leg cannot consume any of the remaining USD
	 * budget (e.g. the cap has already been exhausted).
	 */

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

	private computeLegPolicyOutput(
		inputAmount: bigint,
		inputIsStable: boolean,
		stableDecimals: number,
		exoticTokenDecimals: number,
		remainingUsd: Decimal,
		exoticPerUsd: Decimal,
	): { usdUsed: Decimal; policyMaxOutput: bigint } | null {
		let legMaxUsd: Decimal
		if (inputIsStable) {
			legMaxUsd = new Decimal(formatUnits(inputAmount, stableDecimals))
		} else {
			const normalizedExoticInput = new Decimal(formatUnits(inputAmount, exoticTokenDecimals))
			legMaxUsd = normalizedExoticInput.div(exoticPerUsd)
		}

		const usdForLeg = Decimal.min(legMaxUsd, remainingUsd)
		if (usdForLeg.lte(0)) {
			return null
		}

		let policyMaxOutput: bigint
		if (inputIsStable) {
			// Output is exotic: convert USD allocation to exotic tokens at the policy price
			const exoticFromAlloc = usdForLeg.mul(exoticPerUsd)
			policyMaxOutput = BigInt(exoticFromAlloc.mul(new Decimal(10).pow(exoticTokenDecimals)).floor().toFixed(0))
		} else {
			// Output is stable: the filler pays out the USD value of the exotic input
			policyMaxOutput = BigInt(usdForLeg.mul(new Decimal(10).pow(stableDecimals)).floor().toFixed(0))
		}

		return { usdUsed: usdForLeg, policyMaxOutput }
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
	 * Classifies all (input, output) legs of an order in one pass.
	 * Returns null if any leg has an unsupported pair.
	 */
	private classifyAllPairs(order: Order): CachedPairClassification[] | null {
		if (order.id) {
			const cached = this.contractService.cacheService.getPairClassifications(order.id)
			if (cached) return cached
		}

		const sourceChain = order.source
		const destChain = order.destination
		const sourceExotic = this.token1[sourceChain]
		const destExotic = this.token1[destChain]
		if (!sourceExotic && !destExotic) {
			throw new Error(`Exotic token address not configured for chains ${sourceChain} / ${destChain}`)
		}

		const pairs: CachedPairClassification[] = []
		for (let i = 0; i < order.inputs.length; i++) {
			const normalizedInput = bytes32ToBytes20(order.inputs[i].token).toLowerCase()
			const normalizedOutput = bytes32ToBytes20(order.output.assets[i].token).toLowerCase()

			const inputStable = this.getStableType(normalizedInput, sourceChain)
			const outputStable = this.getStableType(normalizedOutput, destChain)

			if (inputStable && destExotic && normalizedOutput === destExotic.toLowerCase()) {
				pairs.push({
					inputIsStable: true,
					stableToken: order.inputs[i].token,
					exoticToken: order.output.assets[i].token,
				})
			} else if (sourceExotic && normalizedInput === sourceExotic.toLowerCase() && outputStable) {
				pairs.push({
					inputIsStable: false,
					stableToken: order.output.assets[i].token,
					exoticToken: order.inputs[i].token,
				})
			} else {
				return null
			}
		}

		if (order.id) {
			this.contractService.cacheService.setPairClassifications(order.id, pairs)
		}

		return pairs
	}

	/**
	 * One-sided LP gate: returns false if any leg runs in a disabled direction
	 * (curve omitted, or excluded by the venue `side`). A stable-in leg sells exotic
	 * and needs the ask side; an exotic-in leg buys exotic and needs the bid side.
	 * The IntentGateway settles all legs atomically, so a mixed-direction order can't
	 * be partially honoured.
	 *
	 * Kept separate from `classifyAllPairs`: that result is cached and shared across
	 * strategies, whereas enablement is per-strategy, so this must run on every
	 * evaluation regardless of cache state.
	 */
	private isOrderDirectionEnabled(pairs: CachedPairClassification[], orderId: string | undefined): boolean {
		for (let i = 0; i < pairs.length; i++) {
			const leg = pairs[i]
			if ((leg.inputIsStable && !this.askEnabled) || (!leg.inputIsStable && !this.bidEnabled)) {
				this.logger.debug(
					{
						orderId,
						leg: i,
						inputIsStable: leg.inputIsStable,
						bidEnabled: this.bidEnabled,
						askEnabled: this.askEnabled,
					},
					"Rejecting order: leg direction disabled for one-sided LP",
				)
				return false
			}
		}
		return true
	}

	private getStableType(normalizedAddress: string, chain: string): boolean {
		return (
			normalizedAddress === this.configService.getUsdcAsset(chain).toLowerCase() ||
			normalizedAddress === this.configService.getUsdtAsset(chain).toLowerCase()
		)
	}

	/**
	 * Returns the filler's proposed output amounts for a phantom order without
	 * checking on-chain balance or estimating gas. Phantom orders are probes that
	 * never execute; we only need the price signal.
	 *
	 * Returns `null` when the pair is not supported or the USD value cannot be
	 * computed (e.g. venue price unavailable and no fallback).
	 */
	async quotePhantomFill(order: Order): Promise<TokenInfo[] | null> {
		if (!(await this.canFill(order))) return null

		const pairs = this.classifyAllPairs(order)
		if (!pairs) return null

		const usdResult = await this.getOrderUsdValue(order)
		if (!usdResult || usdResult.inputUsd.lte(0)) return null

		const cappedOrderUsd = Decimal.min(usdResult.inputUsd, this.maxOrderUsd)
		if (cappedOrderUsd.lte(0)) return null

		const chain = order.source
		const venuePrice = this.fundingVenues.length > 0 ? await this.getVenuePrice(chain) : null
		const policyBidPrice = this.bidPricePolicy.getPrice(cappedOrderUsd)
		const policyAskPrice = this.askPricePolicy.getPrice(cappedOrderUsd)

		const outputs: TokenInfo[] = []
		let remainingUsd = cappedOrderUsd

		for (let i = 0; i < order.inputs.length; i++) {
			const input = order.inputs[i]
			const output = order.output.assets[i]
			const pair = pairs[i]

			const inputDecimals = await this.contractService.getTokenDecimals(
				bytes32ToBytes20(input.token) as HexString,
				chain,
			)
			const outputDecimals = await this.contractService.getTokenDecimals(
				bytes32ToBytes20(output.token) as HexString,
				chain,
			)

			const stableDecimals = pair.inputIsStable ? inputDecimals : outputDecimals
			const exoticDecimals = pair.inputIsStable ? outputDecimals : inputDecimals
			const bidPrice = venuePrice?.bid ?? policyBidPrice
			const askPrice = venuePrice?.ask ?? policyAskPrice

			const legResult = this.computeLegPolicyOutput(
				input.amount,
				pair.inputIsStable,
				stableDecimals,
				exoticDecimals,
				remainingUsd,
				pair.inputIsStable ? askPrice : bidPrice,
			)

			if (!legResult) continue

			remainingUsd = remainingUsd.minus(legResult.usdUsed)

			// Phantom orders request a zero output (they only probe price), so the user-requested
			// overfill ceiling is zero — capping to it would always quote 0. Skip the cap in that
			// case and quote the full policy output; otherwise keep the (1 + maxOverfillBps) ceiling.
			const overfillCeiling = (output.amount * (10000n + this.maxOverfillBps)) / 10000n
			const amount =
				output.amount > 0n && legResult.policyMaxOutput > overfillCeiling
					? overfillCeiling
					: legResult.policyMaxOutput
			outputs.push({ token: output.token, amount })

			if (remainingUsd.lte(0)) break
		}

		if (outputs.length === 0) return null

		if (order.id) {
			this.contractService.cacheService.setFillerOutputs(order.id, outputs)
		}

		return outputs
	}

	/**
	 * Returns the USD value of the order's full input basket.
	 * Stablecoin inputs are priced at face value; exotic inputs are converted
	 * via the bid price policy at the minimum price point.
	 * Returns `null` only when pair classification fails (genuine "can't price").
	 */
	async getOrderUsdValue(order: Order): Promise<{ inputUsd: Decimal } | null> {
		const pairs = this.classifyAllPairs(order)
		if (!pairs) return null

		const sourceChain = order.source
		let totalInputUsd = new Decimal(0)

		for (let j = 0; j < order.inputs.length; j++) {
			if (pairs[j].inputIsStable) {
				const decimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(order.inputs[j].token) as HexString,
					sourceChain,
				)
				totalInputUsd = totalInputUsd.plus(new Decimal(formatUnits(order.inputs[j].amount, decimals)))
			} else {
				const exoticDecimals = await this.contractService.getTokenDecimals(
					this.token1[sourceChain],
					sourceChain,
				)
				const normalized = new Decimal(formatUnits(order.inputs[j].amount, exoticDecimals))
				const vp = this.fundingVenues.length > 0 ? await this.getVenuePrice(sourceChain) : null
				const bidPriceForChain = vp?.bid ?? this.bidPricePolicy.getPrice(new Decimal(0))
				totalInputUsd = totalInputUsd.plus(normalized.div(bidPriceForChain))
			}
		}

		if (totalInputUsd.lte(0)) return null
		return { inputUsd: totalInputUsd }
	}
}
