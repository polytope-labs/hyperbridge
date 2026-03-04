import { FillerStrategy } from "@/strategies/base"
import {
	OrderV2,
	ExecutionResult,
	HexString,
	bytes32ToBytes20,
	FillOptionsV2,
	TokenInfoV2,
	IntentsCoprocessor,
	adjustDecimals,
	ADDRESS_ZERO,
} from "@hyperbridge/sdk"
import { privateKeyToAccount } from "viem/accounts"
import { ChainClientManager, ContractInteractionService, BidStorageService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { Decimal } from "decimal.js"
import { ERC20_ABI } from "@/config/abis/ERC20"

/**
 * Strategy for same-chain swaps between USD-pegged stablecoins (USDC/USDT)
 * and a single configurable exotic token priced via a `FillerPricePolicy`.
 *
 * The filler holds both the stablecoin(s) and the exotic token. When a user
 * places a same-chain order swapping between the two, this strategy:
 * 1. Evaluates profitability using the filler's price policy for the exotic token
 * 2. Calls fillOrder to deliver output tokens to the user
 * 3. Receives the user's escrowed input tokens from the contract
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
	private bidStorage?: BidStorageService
	/** Exotic token price policy in USD as a function of order USD value */
	private pricePolicy: FillerPricePolicy
	/** Maps chain identifier → exotic token address (e.g. cNGN on each supported chain) */
	private exoticTokenAddresses: Record<string, HexString>
	private maxOrderUsd: Decimal
	private account: ReturnType<typeof privateKeyToAccount>
	private logger = getLogger("fx-filler")

	/**
	 * @param privateKey             Filler's private key used to sign UserOps.
	 * @param configService          Network/config provider for addresses and decimals.
	 * @param clientManager          Used to get viem PublicClients for chains.
	 * @param contractService        Shared contract interaction service.
	 * @param pricePolicy            Exotic token price curve as a function of order USD value.
	 * @param maxOrderUsdStr         Maximum USD value this filler is willing to fill per order.
	 *                                Example: "5000" means, even if the order is for $10,000,
	 *                                the filler will only size its outputs as if the order were $5,000.
	 * @param exoticTokenAddresses   Map of chain identifier → exotic token address.
	 *                                Example: `{ "EVM-56": "0xabc..." }` for cNGN on BSC.
	 * @param bidStorage             Optional storage for submitted bids.
	 */
	constructor(
		privateKey: HexString,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		pricePolicy: FillerPricePolicy,
		maxOrderUsdStr: string,
		exoticTokenAddresses: Record<string, HexString>,
		bidStorage?: BidStorageService,
	) {
		this.privateKey = privateKey
		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.bidStorage = bidStorage
		this.pricePolicy = pricePolicy
		this.exoticTokenAddresses = exoticTokenAddresses
		this.maxOrderUsd = new Decimal(maxOrderUsdStr)
		if (this.maxOrderUsd.lte(0)) {
			throw new Error("FXFiller maxOrderUsd must be greater than 0")
		}
		this.account = privateKeyToAccount(privateKey)
	}

	async canFill(order: OrderV2): Promise<boolean> {
		try {
			if (order.source !== order.destination) {
				return false
			}

			if (order.inputs.length !== order.output.assets.length) {
				this.logger.debug(
					{ inputs: order.inputs.length, outputs: order.output.assets.length },
					"Order input/output length mismatch or empty",
				)
				return false
			}

			const chain = order.source

			for (let i = 0; i < order.inputs.length; i++) {
				const pair = this.classifyPair(order.inputs[i].token, order.output.assets[i].token, chain)
				if (!pair) {
					this.logger.debug({ index: i }, "Unsupported token pair for same-chain swap")
					return false
				}
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Error in canFill")
			return false
		}
	}

	/**
	 * Computes the USD value of both the input and output baskets of an order.
	 *
	 * - Stable tokens (USDC/USDT) are valued at $1 per unit.
	 * - Exotic tokens are valued using the minimum price from the FX price policy
	 *   (`getPrice(0)` clamps to the first configured point).
	 * - Unknown tokens are ignored (contribute $0).
	 *
	 * Returning both sides allows callers to size budgets from inputs and
	 * sanity-check that planned outputs never exceed the value received.
	 */
	async getOrderUsdValue(order: OrderV2): Promise<{ inputUsd: Decimal; outputUsd: Decimal } | null> {
		try {
			const chain = order.source
			const exoticAddress = this.exoticTokenAddresses[chain].toLowerCase()

			const exoticDecimals = await this.contractService.getTokenDecimals(exoticAddress, chain)

			const sourceUsdc = this.configService.getUsdcAsset(chain).toLowerCase()
			const sourceUsdt = this.configService.getUsdtAsset(chain).toLowerCase()

			const exoticMinPriceUsd = this.pricePolicy.getPrice(new Decimal(0))

			let inputUsd = new Decimal(0)
			let outputUsd = new Decimal(0)

			for (let i = 0; i < order.inputs.length; i++) {
				const inputAddress = bytes32ToBytes20(order.inputs[i].token).toLowerCase()
				const outputAddress = bytes32ToBytes20(order.output.assets[i].token).toLowerCase()

				// Value the input side
				if (inputAddress === sourceUsdc || inputAddress === sourceUsdt) {
					const decimals = await this.contractService.getTokenDecimals(inputAddress as HexString, chain)
					inputUsd = inputUsd.plus(new Decimal(formatUnits(order.inputs[i].amount, decimals)))
				} else if (inputAddress === exoticAddress) {
					const normalized = new Decimal(formatUnits(order.inputs[i].amount, exoticDecimals))
					inputUsd = inputUsd.plus(normalized.mul(exoticMinPriceUsd))
				}

				// Value the output side
				if (outputAddress === sourceUsdc || outputAddress === sourceUsdt) {
					const decimals = await this.contractService.getTokenDecimals(outputAddress as HexString, chain)
					outputUsd = outputUsd.plus(new Decimal(formatUnits(order.output.assets[i].amount, decimals)))
				} else if (outputAddress === exoticAddress) {
					const normalized = new Decimal(formatUnits(order.output.assets[i].amount, exoticDecimals))
					outputUsd = outputUsd.plus(normalized.mul(exoticMinPriceUsd))
				}
			}

			if (inputUsd.lte(0)) {
				this.logger.error({ orderId: order.id }, "Total input USD value is non-positive")
				return null
			}

			return { inputUsd, outputUsd }
		} catch (error) {
			this.logger.error({ err: error }, "Error computing order USD values")
			return null
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
	async calculateProfitability(order: OrderV2): Promise<number> {
		try {
			const chain = order.source
			const { decimals: feeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(chain)

			const destClient = this.clientManager.getPublicClient(chain)
			const walletAddress = this.account.address as HexString
			const balanceCache = new Map<string, bigint>()

			const orderUsd = await this.getOrderUsdValue(order)
			if (!orderUsd) {
				this.logger.info({ orderId: order.id }, "Skipping order: could not compute order USD values")
				return 0
			}

			const { inputUsd: totalInputUsd, outputUsd: totalOutputUsd } = orderUsd

			if (totalOutputUsd.gt(totalInputUsd)) {
				this.logger.info(
					{
						orderId: order.id,
						totalInputUsd: totalInputUsd.toString(),
						totalOutputUsd: totalOutputUsd.toString(),
					},
					"Skipping order: requested output USD value exceeds input USD value",
				)
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

			const exoticTokenPriceUsd = this.pricePolicy.getPrice(cappedOrderUsd)
			const fillerOutputs: TokenInfoV2[] = []
			let remainingUsd = cappedOrderUsd

			for (let i = 0; i < order.inputs.length; i++) {
				const input = order.inputs[i]
				const output = order.output.assets[i]
				const pair = this.classifyPair(input.token, output.token, chain)!

				const stableDecimals = await this.contractService.getTokenDecimals(
					bytes32ToBytes20(pair.stableToken) as HexString,
					chain,
				)

				const exoticTokenDecimals = await this.contractService.getTokenDecimals(
					this.exoticTokenAddresses[chain],
					chain,
				)

				const legResult = this.computeLegPolicyOutput(
					input.amount,
					pair.inputIsStable,
					stableDecimals,
					exoticTokenDecimals,
					remainingUsd,
					exoticTokenPriceUsd,
				)

				if (!legResult) {
					continue
				}

				const { usdUsed, policyMaxOutput } = legResult
				remainingUsd = remainingUsd.minus(usdUsed)

				// Cap by actual available balance for this token on the filler side.
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

			const { totalCostInSourceFeeToken } = await this.contractService.estimateGasFillPost(order)
			const feeProfit = order.fees > totalCostInSourceFeeToken ? order.fees - totalCostInSourceFeeToken : 0n

			this.logger.info(
				{
					orderId: order.id,
					orderValueUsdFull: totalInputUsd.toString(),
					orderValueUsdCapped: cappedOrderUsd.toString(),
					maxOrderUsd: this.maxOrderUsd.toString(),
					exoticTokenPriceUsd: exoticTokenPriceUsd.toString(),
					feeProfit: formatUnits(feeProfit, feeTokenDecimals),
					profitable: feeProfit > 0n,
				},
				"Same-chain swap profitability evaluation",
			)

			return parseFloat(formatUnits(feeProfit, feeTokenDecimals))
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
	async executeOrder(order: OrderV2, intentsCoprocessor?: IntentsCoprocessor): Promise<ExecutionResult> {
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
			this.logger.error({ err: error }, "Error executing same-chain swap order")
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
	 * `ContractInteractionService.prepareBidUserOp`. Bid metadata is persisted
	 * to `BidStorageService` when available.
	 */
	private async submitBid(
		order: OrderV2,
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
			this.bidStorage?.storeBid({
				commitment,
				extrinsicHash: bidResult.extrinsicHash!,
				blockHash: bidResult.blockHash!,
				success: true,
			})
			return {
				success: true,
				txHash: bidResult.extrinsicHash,
				strategyUsed: this.name,
				processingTimeMs: endTime - startTime,
			}
		}

		this.logger.error({ commitment, error: bidResult.error }, "Bid submission failed")
		this.bidStorage?.storeBid({ commitment, success: false, error: bidResult.error })
		return { success: false, error: bidResult.error }
	}

	// =========================================================================
	// Private — Helpers
	// =========================================================================

	/**
	 * Given a single (input, output) leg and the remaining capped USD budget,
	 * computes how much USD to allocate to this leg and the corresponding
	 * maximum output amount according to the price policy.
	 *
	 * Uses `exoticTokenPriceUsd` consistently for both directions:
	 * - Stable input → exotic output: USD allocation from stable amount, converted to exotic at policy price.
	 * - Exotic input → stable output: USD allocation from exotic amount priced at policy price.
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
		exoticTokenPriceUsd: Decimal,
	): { usdUsed: Decimal; policyMaxOutput: bigint } | null {
		let legMaxUsd: Decimal
		if (inputIsStable) {
			legMaxUsd = new Decimal(formatUnits(inputAmount, stableDecimals))
		} else {
			const normalizedExoticInput = new Decimal(formatUnits(inputAmount, exoticTokenDecimals))
			legMaxUsd = normalizedExoticInput.mul(exoticTokenPriceUsd)
		}

		const usdForLeg = Decimal.min(legMaxUsd, remainingUsd)
		if (usdForLeg.lte(0)) {
			return null
		}

		let policyMaxOutput: bigint
		if (inputIsStable) {
			// Output is exotic: convert USD allocation to exotic tokens at the policy price
			const exoticFromAlloc = usdForLeg.div(exoticTokenPriceUsd)
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

	private classifyPair(
		inputToken: string,
		outputToken: string,
		chain: string,
	): {
		inputIsStable: boolean
		stableToken: string
		exoticToken: string
	} | null {
		const exoticAddress = this.exoticTokenAddresses[chain]
		if (!exoticAddress) {
			throw new Error(`Exotic token address not configured for chain ${chain}`)
		}

		const normalizedInput = bytes32ToBytes20(inputToken).toLowerCase()
		const normalizedOutput = bytes32ToBytes20(outputToken).toLowerCase()
		const normalizedExotic = exoticAddress.toLowerCase()

		const inputStable = this.getStableType(normalizedInput, chain)
		const outputStable = this.getStableType(normalizedOutput, chain)

		if (inputStable && normalizedOutput === normalizedExotic) {
			return { inputIsStable: true, stableToken: inputToken, exoticToken: outputToken }
		}

		if (normalizedInput === normalizedExotic && outputStable) {
			return { inputIsStable: false, stableToken: outputToken, exoticToken: inputToken }
		}

		return null
	}

	private getStableType(normalizedAddress: string, chain: string): boolean {
		return (
			normalizedAddress === this.configService.getUsdcAsset(chain).toLowerCase() ||
			normalizedAddress === this.configService.getUsdtAsset(chain).toLowerCase()
		)
	}
}
