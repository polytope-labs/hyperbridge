import { FillerStrategy } from "@/strategies/base"
import {
	Order,
	ExecutionResult,
	HexString,
	bytes32ToBytes20,
	FillOptions,
	TokenInfo,
	IntentsCoprocessor,
	adjustDecimals,
	ADDRESS_ZERO,
} from "@hyperbridge/sdk"
import { privateKeyToAccount } from "viem/accounts"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { type CachedPairClassification } from "@/services/CacheService"
import { Decimal } from "decimal.js"
import { ERC20_ABI } from "@/config/abis/ERC20"

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
	private privateKey: HexString
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	/** Bid price policy: exotic tokens per USD when the filler is *buying* exotic from a user */
	private bidPricePolicy: FillerPricePolicy
	/** Ask price policy: exotic tokens per USD when the filler is *selling* exotic to a user */
	private askPricePolicy: FillerPricePolicy
	/** Maps chain identifier → exotic token address (e.g. cNGN on each supported chain) */
	private exoticTokenAddresses: Record<string, HexString>
	private maxOrderUsd: Decimal
	private account: ReturnType<typeof privateKeyToAccount>
	private logger = getLogger("fx-simplex")
	confirmationPolicy?: { getConfirmationBlocks: (chainId: number, amountUsd: number) => number }

	/**
	 * @param privateKey             Filler's private key used to sign UserOps.
	 * @param configService          Network/config provider for addresses and decimals.
	 * @param clientManager          Used to get viem PublicClients for chains.
	 * @param contractService        Shared contract interaction service.
	 * @param bidPricePolicy         Price curve used when the filler *buys* exotic from a user
	 *                                (exotic→stable leg). A higher exotic-per-USD rate here means
	 *                                the filler pays out fewer stables per exotic token received.
	 * @param askPricePolicy         Price curve used when the filler *sells* exotic to a user
	 *                                (stable→exotic leg). A lower exotic-per-USD rate here means
	 *                                the filler sends fewer exotic tokens per stable received.
	 * @param maxOrderUsdStr         Maximum USD value this filler is willing to fill per order.
	 *                                Example: "5000" means, even if the order is for $10,000,
	 *                                the filler will only size its outputs as if the order were $5,000.
	 * @param exoticTokenAddresses   Map of chain identifier → exotic token address.
	 *                                Example: `{ "EVM-56": "0xabc..." }` for cNGN on BSC.
	 * @param confirmationPolicy     Optional per-chain confirmation policy for cross-chain orders.
	 *                                If absent, no confirmation waiting is required.
	 */
	constructor(
		privateKey: HexString,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		bidPricePolicy: FillerPricePolicy,
		askPricePolicy: FillerPricePolicy,
		maxOrderUsdStr: string,
		exoticTokenAddresses: Record<string, HexString>,
		confirmationPolicy?: ConfirmationPolicy,
	) {
		this.privateKey = privateKey
		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.bidPricePolicy = bidPricePolicy
		this.askPricePolicy = askPricePolicy
		this.exoticTokenAddresses = exoticTokenAddresses
		this.maxOrderUsd = new Decimal(maxOrderUsdStr)
		if (this.maxOrderUsd.lte(0)) {
			throw new Error("FXFiller maxOrderUsd must be greater than 0")
		}
		this.account = privateKeyToAccount(privateKey)
		if (confirmationPolicy) {
			this.confirmationPolicy = {
				getConfirmationBlocks: (chainId: number, amountUsd: number) =>
					confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
			}
		}
	}

	async canFill(order: Order): Promise<boolean> {
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
		try {
			const sourceChain = order.source
			const destChain = order.destination
			const { decimals: feeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(sourceChain)

			const destClient = this.clientManager.getPublicClient(destChain)
			const walletAddress = this.account.address as HexString
			const balanceCache = new Map<string, bigint>()

			const pairs = this.classifyAllPairs(order)
			if (!pairs) {
				this.logger.info({ orderId: order.id }, "Skipping order: could not classify token pairs")
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
			const bidPrice = this.bidPricePolicy.getPrice(cappedOrderUsd)
			const askPrice = this.askPricePolicy.getPrice(cappedOrderUsd)
			const fillerOutputs: TokenInfo[] = []
			let remainingUsd = cappedOrderUsd

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

				const { usdUsed, policyMaxOutput } = legResult
				remainingUsd = remainingUsd.minus(usdUsed)

				// Cap by actual available balance for this token on the destination chain.
				const tokenAddress = bytes32ToBytes20(output.token).toLowerCase()
				const balance = await this.getAndCacheBalance(tokenAddress, walletAddress, destClient, balanceCache)

				const finalOutputAmount = balance > policyMaxOutput ? policyMaxOutput : balance

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

				if (sourceChain !== destChain && finalOutputAmount < output.amount) {
					this.logger.info(
						{
							orderId: order.id,
							token: output.token,
							fillerOutput: finalOutputAmount.toString(),
							userRequested: output.amount.toString(),
						},
						"Skipping order: filler output below user's requested minimum",
					)
					return 0
				}

				// Decrement remaining balance for this token so repeated outputs share the same pool.
				const remaining = balance - finalOutputAmount
				balanceCache.set(tokenAddress, remaining > 0n ? remaining : 0n)

				fillerOutputs.push({ token: output.token, amount: finalOutputAmount })

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

			// Spread profit (bid/ask): value received minus cost, using opposite side of spread for valuation.
			// - When filler sells exotic (stable→exotic): value exotic we give at acquisition cost (bid).
			// - When filler buys exotic (exotic→stable): value exotic we receive at resale (ask).
			let spreadProfitUsd = new Decimal(0)
			for (let i = 0; i < fillerOutputs.length; i++) {
				const input = order.inputs[i]
				const output = fillerOutputs[i]
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
				const exoticDecimalsLeg = pair.inputIsStable ? outputDecimals : inputDecimals
				if (pair.inputIsStable) {
					// Filler sells exotic (stable→exotic): receives stable, gives exotic. Value exotic at bid (acquisition cost).
					const inputUsd = new Decimal(formatUnits(input.amount, stableDecimals))
					const outputExotic = new Decimal(formatUnits(output.amount, exoticDecimalsLeg))
					// Cost = outputExotic / bidPrice; revenue = inputUsd → profit = inputUsd - outputExotic/bid
					spreadProfitUsd = spreadProfitUsd.plus(inputUsd.minus(outputExotic.div(bidPrice)))
				} else {
					// Filler buys exotic (exotic→stable): receives exotic, gives stable. Value exotic at ask (resale value).
					const inputExotic = new Decimal(formatUnits(input.amount, exoticDecimalsLeg))
					const outputUsd = new Decimal(formatUnits(output.amount, stableDecimals))
					// Revenue = inputExotic / askPrice; cost = outputUsd → profit = inputExotic/ask - outputUsd
					spreadProfitUsd = spreadProfitUsd.plus(inputExotic.div(askPrice).minus(outputUsd))
				}
			}

			const { totalCostInSourceFeeToken } = await this.contractService.estimateGasFillPost(order)
			const feeProfit = order.fees > totalCostInSourceFeeToken ? order.fees - totalCostInSourceFeeToken : 0n
			const feeProfitParsed = parseFloat(formatUnits(feeProfit, feeTokenDecimals))
			const totalProfit = feeProfitParsed + spreadProfitUsd.toNumber()

			this.logger.info(
				{
					orderId: order.id,
					sourceChain,
					destChain,
					crossChain: sourceChain !== destChain,
					orderValueUsdFull: totalInputUsd.toString(),
					orderValueUsdCapped: cappedOrderUsd.toString(),
					maxOrderUsd: this.maxOrderUsd.toString(),
					bidPrice: bidPrice.toString(),
					askPrice: askPrice.toString(),
					orderFees: formatUnits(order.fees, feeTokenDecimals),
					estimatedFees: formatUnits(totalCostInSourceFeeToken, feeTokenDecimals),
					feeProfit: formatUnits(feeProfit, feeTokenDecimals),
					spreadProfitUsd: spreadProfitUsd.toString(),
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

		const solverAccountAddress = this.account.address as HexString

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
		const sourceExotic = this.exoticTokenAddresses[sourceChain]
		const destExotic = this.exoticTokenAddresses[destChain]
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

		this.contractService.cacheService.setPairClassifications(order.id!, pairs)

		return pairs
	}

	private getStableType(normalizedAddress: string, chain: string): boolean {
		return (
			normalizedAddress === this.configService.getUsdcAsset(chain).toLowerCase() ||
			normalizedAddress === this.configService.getUsdtAsset(chain).toLowerCase()
		)
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
					this.exoticTokenAddresses[sourceChain],
					sourceChain,
				)
				const normalized = new Decimal(formatUnits(order.inputs[j].amount, exoticDecimals))
				totalInputUsd = totalInputUsd.plus(normalized.div(this.bidPricePolicy.getPrice(new Decimal(0))))
			}
		}

		if (totalInputUsd.lte(0)) return null
		return { inputUsd: totalInputUsd }
	}
}
