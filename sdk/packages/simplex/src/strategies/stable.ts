import { Decimal } from "decimal.js"
import { FillerStrategy } from "@/strategies/base"
import {
	Order,
	ExecutionResult,
	HexString,
	bytes32ToBytes20,
	FillOptions,
	ADDRESS_ZERO,
	TokenInfo,
	adjustDecimals,
	IntentsCoprocessor,
	type ERC7821Call,
} from "@hyperbridge/sdk"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"
import { FillerBpsPolicy, ConfirmationPolicy } from "@/config/interpolated-curve"
import { SupportedTokenType } from "@/strategies/base"
import type { FundingVenue } from "@/funding/types"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { SigningAccount } from "@/services/wallet"

export class StableFiller implements FillerStrategy {
	name = "StableFiller"
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	private bpsPolicy: FillerBpsPolicy
	private signer: SigningAccount
	private logger = getLogger("stable-simplex")
	/** Ceiling bps above user-requested output. Sourced from filler config. */
	private readonly maxOverfillBps: bigint
	/** On-chain liquidity sources for topping up output-token shortfalls (e.g. ERC-4626 vaults). */
	private fundingVenues: FundingVenue[]
	confirmationPolicy: { getConfirmationBlocks: (chainId: number, amountUsd: number) => number }

	constructor(
		signer: SigningAccount,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		bpsPolicy: FillerBpsPolicy,
		confirmationPolicy: ConfirmationPolicy,
		fundingVenues: FundingVenue[] = [],
	) {
		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.bpsPolicy = bpsPolicy
		this.confirmationPolicy = {
			getConfirmationBlocks: (chainId: number, amountUsd: number) =>
				confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
		}
		this.signer = signer
		this.maxOverfillBps = configService.getMaxOverfillBps()
		this.fundingVenues = fundingVenues
	}

	/**
	 * Call once at startup after construction. Hydrates funding venue state so
	 * output-token shortfalls can be sourced on-chain during fills.
	 */
	async initialise(): Promise<void> {
		const solver = this.signer.account.address as HexString
		await Promise.all(this.fundingVenues.map((v) => v.initialise(solver)))
	}

	/**
	 * Determines if this strategy can fill the given order.
	 * Validates that the order has supported token pairs (same-token swaps only).
	 * @param order The order to check
	 * @returns True if the strategy can fill the order
	 */
	async canFill(order: Order): Promise<boolean> {
		try {
			// Validate order structure
			if (order.inputs.length === 0 || order.inputs.length !== order.output.assets.length) {
				this.logger.debug(
					{ inputs: order.inputs.length, outputs: order.output.assets.length },
					"Order input/output length mismatch or empty",
				)
				return false
			}

			// Validate all token pairs are supported (same-token swaps: USDC→USDC, USDT→USDT)
			for (let i = 0; i < order.inputs.length; i++) {
				const inputType = this.getTokenType(order.inputs[i].token, order.source)
				const outputType = this.getTokenType(order.output.assets[i].token, order.destination)

				if (!inputType) {
					this.logger.debug({ index: i, token: order.inputs[i].token }, "Unsupported input token")
					return false
				}

				if (!outputType) {
					this.logger.debug({ index: i, token: order.output.assets[i].token }, "Unsupported output token")
					return false
				}

				if (inputType !== outputType) {
					this.logger.debug(
						{ index: i, inputType, outputType },
						"Token type mismatch (must be same-token swap)",
					)
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
	 * Gets the supported token type for a given token address on a chain.
	 * @param tokenAddress The token address (bytes32 format)
	 * @param chain The chain identifier
	 * @returns The token type or null if unsupported
	 */
	private getTokenType(tokenAddress: string, chain: string): SupportedTokenType | null {
		const normalizedAddress = bytes32ToBytes20(tokenAddress).toLowerCase()
		const supportedAssets: Record<SupportedTokenType, string> = {
			USDT: this.configService.getUsdtAsset(chain).toLowerCase(),
			USDC: this.configService.getUsdcAsset(chain).toLowerCase(),
		}

		for (const [tokenType, address] of Object.entries(supportedAssets)) {
			if (address === normalizedAddress) {
				return tokenType as SupportedTokenType
			}
		}

		return null
	}

	/**
	 * Calculates the USD value of the order's inputs, outputs, fees and compares
	 * what will the filler receive and what will the filler pay.
	 * Also validates that the order output amounts meet the filler's minimum requirements
	 * based on the configured bps (basis points) curve.
	 * @param order The order to calculate the USD value for
	 * @returns The profit in USD (Number), or 0 if not profitable or output amounts don't meet minimum
	 */
	async calculateProfitability(order: Order): Promise<number> {
		try {
			const { decimals: destFeeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(
				order.destination,
			)

			// Get order value and determine dynamic BPS from policy
			const inputUsdValue = await this.contractService.getInputUsdValue(order)
			const fillerBps = this.bpsPolicy.getBps(inputUsdValue)

			// Validate that order outputs meet filler's minimum bps requirements
			// and calculate profit from slippage (normalized to dest fee token decimals)
			const { isValid, profitFromSlippage } = await this.calculateSlippageProfit(
				order,
				destFeeTokenDecimals,
				fillerBps,
			)
			if (!isValid) {
				this.logger.info(
					{ orderId: order.id, orderValueUsd: inputUsdValue.toString(), fillerBps: fillerBps.toString() },
					"User expects more output than filler can provide based on bps",
				)
				return 0
			}

			// Source output-token shortfalls from funding venues (e.g. ERC-4626 vaults).
			// Runs before gas estimation so the prepend gas bump is accounted for.
			const fillerOutputs = this.contractService.cacheService.getFillerOutputs(order.id!)
			if (fillerOutputs && this.fundingVenues.length > 0) {
				const fundable = await this.planFunding(order, fillerOutputs)
				if (!fundable) return 0
				this.contractService.cacheService.setFillerOutputs(order.id!, fillerOutputs)
			}

			const { totalCostInSourceFeeToken } = await this.contractService.estimateGasFillPost(order)

			const { decimals: sourceFeeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(
				order.source,
			)

			// Profit from fees: order.fees - gas costs
			const feeProfit = order.fees > totalCostInSourceFeeToken ? order.fees - totalCostInSourceFeeToken : 0n

			// Total profit = fee profit + profit from slippage (both normalized to dest fee token decimals)
			const feeProfitInDestDecimals = adjustDecimals(feeProfit, sourceFeeTokenDecimals, destFeeTokenDecimals)
			const totalProfit = feeProfitInDestDecimals + profitFromSlippage

			this.logger.info(
				{
					orderFeesUSD: formatUnits(order.fees, sourceFeeTokenDecimals),
					totalCostInSourceFeeTokenUSD: formatUnits(totalCostInSourceFeeToken, sourceFeeTokenDecimals),
					feeProfitUSD: formatUnits(feeProfitInDestDecimals, destFeeTokenDecimals),
					slippageProfitUSD: formatUnits(profitFromSlippage, destFeeTokenDecimals),
					totalProfitUSD: formatUnits(totalProfit, destFeeTokenDecimals),
					profitable: totalProfit > 0n,
				},
				"Profitability evaluation",
			)
			return parseFloat(formatUnits(totalProfit, destFeeTokenDecimals))
		} catch (error) {
			this.logger.error({ err: error }, "Error calculating profitability")
			return 0
		}
	}

	/**
	 * Validates that the filler can meet the user's minimum output requirements
	 * based on the configured bps (basis points), and calculates the profit from slippage.
	 * Also caches the filler's calculated outputs for use during order execution.
	 *
	 * The logic:
	 * - User sends X tokens and expects minimum Y tokens (order.output.amount)
	 * - Filler calculates max they will provide: X * (10000 - fillerBps) / 10000
	 * - If filler can provide >= user's minimum → valid, proceed
	 * - Filler pays out their calculated max (to be competitive)
	 * - Profit = X - fillerMaxOutput (filler keeps their bps as profit)
	 *
	 * Example: User sends 100 USDC, expects minimum 99.4 USDC, filler has 50 bps (0.5%)
	 * - Filler will provide: 100 * (10000 - 50) / 10000 = 99.5 USDC
	 * - User expects 99.4 USDC, filler provides 99.5 → valid (99.5 >= 99.4)
	 * - Profit = 100 - 99.5 = 0.5 USDC (filler receives 100, pays out 99.5)
	 *
	 * @param order The order to validate (assumed to have passed canFill validation)
	 * @param normalizeToDecimals The decimal precision to normalize the profit to (e.g., dest fee token decimals)
	 * @param fillerBps The basis points to use for this order (determined by order value)
	 * @returns Object with isValid boolean and profitFromSlippage (normalized to specified decimals)
	 */
	private async calculateSlippageProfit(
		order: Order,
		normalizeToDecimals: number,
		fillerBps: bigint,
	): Promise<{ isValid: boolean; profitFromSlippage: bigint }> {
		const basisPoints = 10000n
		let totalProfitNormalized = 0n
		const fillerOutputs: TokenInfo[] = []

		for (let i = 0; i < order.inputs.length; i++) {
			const input = order.inputs[i]
			const output = order.output.assets[i]

			// Get token decimals for both chains
			const [inputDecimals, outputDecimals] = await Promise.all([
				this.contractService.getTokenDecimals(input.token, order.source),
				this.contractService.getTokenDecimals(output.token, order.destination),
			])

			// Convert input amount to output decimals
			const convertedInputAmount = adjustDecimals(input.amount, inputDecimals, outputDecimals)

			// Calculate max output filler will provide based on their bps
			// Formula: inputAmount * (10000 - fillerBps) / 10000
			const bpsOutput = (convertedInputAmount * (basisPoints - fillerBps)) / basisPoints

			// Reject if user expects more than filler can provide
			if (output.amount > bpsOutput) {
				this.logger.debug(
					{
						index: i,
						inputAmount: input.amount.toString(),
						inputDecimals,
						userExpects: output.amount.toString(),
						fillerWillProvide: bpsOutput.toString(),
						outputDecimals,
						fillerBps: fillerBps.toString(),
					},
					"User expects more than filler can provide based on bps",
				)
				return { isValid: false, profitFromSlippage: 0n }
			}

			// Clamp to at most (1 + maxOverfillBps) × user-requested to bound loss on pricing errors.
			const overfillCeiling = (output.amount * (10000n + this.maxOverfillBps)) / 10000n
			let fillerMaxOutput = bpsOutput
			if (bpsOutput > overfillCeiling) {
				this.logger.warn(
					{
						orderId: order.id,
						index: i,
						userRequested: output.amount.toString(),
						unclamped: bpsOutput.toString(),
						clamped: overfillCeiling.toString(),
						maxOverfillBps: this.maxOverfillBps.toString(),
					},
					"Overfill clamp activated",
				)
				fillerMaxOutput = overfillCeiling
			}

			// Store the filler's calculated output for this token
			fillerOutputs.push({
				token: output.token,
				amount: fillerMaxOutput,
			})

			// Calculate profit: filler receives input, pays out their max (to be competitive)
			// Profit = input - fillerMaxOutput (filler keeps their bps as profit)
			const profitInOutputDecimals = convertedInputAmount - fillerMaxOutput

			// Normalize profit to the target decimals for summing across different tokens
			const profitNormalized = adjustDecimals(profitInOutputDecimals, outputDecimals, normalizeToDecimals)
			totalProfitNormalized += profitNormalized
		}

		// Cache filler outputs for use during order execution
		this.contractService.cacheService.setFillerOutputs(order.id!, fillerOutputs)
		this.logger.debug(
			{
				orderId: order.id,
				fillerOutputs: fillerOutputs.map((o) => ({ token: o.token, amount: o.amount.toString() })),
			},
			"Cached filler outputs for order",
		)

		return { isValid: true, profitFromSlippage: totalProfitNormalized }
	}

	/**
	 * Quotes fill outputs for a phantom order using the bps policy, without gas estimation.
	 * Returns null when this strategy does not support the order's token pair.
	 */
	async quotePhantomFill(order: Order): Promise<TokenInfo[] | null> {
		if (!(await this.canFill(order))) return null

		const basisPoints = 10000n
		const inputUsdValue = await this.contractService.getInputUsdValue(order)
		const fillerBps = this.bpsPolicy.getBps(inputUsdValue)
		const outputs: TokenInfo[] = []

		for (let i = 0; i < order.inputs.length; i++) {
			const input = order.inputs[i]
			const output = order.output.assets[i]

			const [inputDecimals, outputDecimals] = await Promise.all([
				this.contractService.getTokenDecimals(input.token, order.source),
				this.contractService.getTokenDecimals(output.token, order.destination),
			])

			const convertedInput = adjustDecimals(input.amount, inputDecimals, outputDecimals)
			const bpsOutput = (convertedInput * (basisPoints - fillerBps)) / basisPoints
			const overfillCeiling = (output.amount * (10000n + this.maxOverfillBps)) / 10000n

			outputs.push({ token: output.token, amount: bpsOutput > overfillCeiling ? overfillCeiling : bpsOutput })
		}

		this.contractService.cacheService.setFillerOutputs(order.id!, outputs)
		return outputs
	}

	/**
	 * Sources each output token from the solver's wallet, topping up shortfalls
	 * via funding venues (e.g. an ERC-4626 vault `withdraw`). When a venue can only
	 * partially cover the deficit, the competitive output is reduced to the
	 * coverable amount — never below the user's requested minimum. The venue
	 * withdrawal calls are recorded as ERC-7821 prepends so they execute atomically
	 * before `fillOrder` in the same batch.
	 *
	 * Mutates `fillerOutputs` in place. Returns false when an output cannot be
	 * sourced down to the user's minimum, signalling the order should be skipped.
	 */
	private async planFunding(order: Order, fillerOutputs: TokenInfo[]): Promise<boolean> {
		const destClient = this.clientManager.getPublicClient(order.destination)
		const solver = this.signer.account.address as HexString
		const balanceCache = new Map<string, bigint>()
		const fundingCalls: ERC7821Call[] = []

		for (let i = 0; i < fillerOutputs.length; i++) {
			const out = fillerOutputs[i]
			const userMin = order.output.assets[i].amount
			const tokenLower = bytes32ToBytes20(out.token).toLowerCase()

			// Native outputs can't be sourced from token venues.
			if (tokenLower === ADDRESS_ZERO.toLowerCase()) continue

			const walletBalance = await this.getAndCacheBalance(tokenLower, solver, destClient, balanceCache)

			// Wallet floor reserved for gas/paymaster (the vault minBalance) — never filled from.
			let reserve = 0n
			for (const venue of this.fundingVenues) {
				reserve += venue.walletReserveForToken(order.destination, tokenLower)
			}
			const usableWallet = walletBalance > reserve ? walletBalance - reserve : 0n

			// Spend the free wallet balance (above the reserve) first, then source any
			// remaining shortfall from the funding venues (the vault).
			const walletContribution = out.amount < usableWallet ? out.amount : usableWallet

			let credited = 0n
			let needed = out.amount - walletContribution
			for (const venue of this.fundingVenues) {
				if (needed <= 0n) break
				const planned = await venue.planWithdrawalForToken(order.destination, solver, tokenLower, needed)
				if (planned.calls.length > 0) {
					fundingCalls.push(...planned.calls)
					credited += planned.credited
					needed -= planned.credited
				}
			}

			const available = walletContribution + credited

			const effectiveOutput = out.amount < available ? out.amount : available
			if (effectiveOutput < userMin) {
				this.logger.info(
					{
						orderId: order.id,
						token: out.token,
						userMin: userMin.toString(),
						sourceable: available.toString(),
					},
					"Skipping order: cannot source output token down to user minimum",
				)
				this.contractService.cacheService.clearFundingPrepends(order.id!)
				return false
			}

			if (effectiveOutput < out.amount) {
				this.logger.info(
					{
						orderId: order.id,
						token: out.token,
						competitive: out.amount.toString(),
						coverable: effectiveOutput.toString(),
					},
					"Reducing output to coverable amount (partial funding)",
				)
				out.amount = effectiveOutput
			}
			// Decrement the wallet pool by what this leg drew from it (vault-sourced tokens
			// are tracked by the venue's own reservations) so repeated outputs share one pool.
			balanceCache.set(tokenLower, walletBalance - walletContribution)
		}

		if (fundingCalls.length > 0) {
			this.contractService.cacheService.setFundingPrepends(order.id!, fundingCalls)
		} else {
			this.contractService.cacheService.clearFundingPrepends(order.id!)
		}
		return true
	}

	/**
	 * Reads (and memoises) the solver's balance of a token on the destination
	 * chain. Lets multiple output legs in one evaluation share a balance pool.
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
		if (cached !== undefined) return cached

		const balance = (await destClient.readContract({
			abi: ERC20_ABI,
			address: key as HexString,
			functionName: "balanceOf",
			args: [walletAddress],
		})) as bigint

		balanceCache.set(key, balance)
		return balance
	}

	/**
	 * Executes the order fill.
	 * If hyperbridge is provided, submits a bid (solver selection mode).
	 * Otherwise, fills the order directly via contract call.
	 *
	 * @param order The order to fill
	 * @param hyperbridge HyperbridgeService for bid submission (provided when solver selection is active)
	 * @returns The execution result
	 */
	async executeOrder(order: Order, intentsCoprocessor?: IntentsCoprocessor): Promise<ExecutionResult> {
		const startTime = Date.now()

		try {
			if (intentsCoprocessor) {
				return await this.submitBidToHyperbridge(order, startTime, intentsCoprocessor)
			}
			return await this.fillOrder(order, startTime)
		} catch (error) {
			this.logger.error({ err: error }, "Error executing order")

			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Submits a bid to Hyperbridge for solver selection mode
	 * @private
	 */
	private async submitBidToHyperbridge(
		order: Order,
		startTime: number,
		intentsCoprocessor: IntentsCoprocessor,
	): Promise<ExecutionResult> {
		const entryPointAddress = this.configService.getEntryPointAddress(order.destination)

		if (!entryPointAddress) {
			const errorMsg = `Solver selection is active but entryPointAddress is not configured for chain ${order.destination}.`
			this.logger.error(errorMsg)
			return {
				success: false,
				error: errorMsg,
			}
		}

		// With EIP-7702 delegation, the filler's EOA address IS the solver account
		const solverAccountAddress = this.signer.account.address as HexString

		this.logger.info({ orderId: order.id, destination: order.destination }, "Submitting bid to Hyperbridge")

		// Prepare the signed UserOp for bid submission (bundles approvals + fillOrder internally)
		const { commitment, userOp } = await this.contractService.prepareBidUserOp(
			order,
			entryPointAddress,
			solverAccountAddress,
		)

		// Submit the bid to Hyperbridge
		const bidResult = await intentsCoprocessor.submitBid(commitment, userOp)

		const endTime = Date.now()
		const processingTimeMs = endTime - startTime

		if (bidResult.success) {
			this.logger.info(
				{
					commitment,
					blockHash: bidResult.blockHash,
					extrinsicHash: bidResult.extrinsicHash,
				},
				"Bid submitted to Hyperbridge successfully",
			)

			return {
				success: true,
				txHash: bidResult.extrinsicHash,
				strategyUsed: this.name,
				processingTimeMs,
				commitment,
			}
		}
		this.logger.error({ commitment, error: bidResult.error }, "Failed to submit bid to Hyperbridge")

		return {
			success: false,
			error: bidResult.error,
			commitment,
		}
	}

	/**
	 * Fills the order directly via contract call (non-solver selection mode)
	 * Uses cached filler outputs (calculated based on bps) instead of order.output.assets
	 * @private
	 */
	private async fillOrder(order: Order, startTime: number): Promise<ExecutionResult> {
		const { destClient, walletClient } = this.clientManager.getClientsForOrder(order)

		const { dispatchFee, nativeDispatchFee, callGasLimit } = await this.contractService.estimateGasFillPost(order)

		// Use cached filler outputs (calculated based on bps) for competitive filling
		const cachedFillerOutputs = this.contractService.cacheService.getFillerOutputs(order.id!)
		if (!cachedFillerOutputs) {
			throw new Error(`No cached filler outputs found for order ${order.id}. Call calculateProfitability first.`)
		}

		const fillOptions: FillOptions = {
			relayerFee: dispatchFee,
			nativeDispatchFee,
			outputs: cachedFillerOutputs,
		}

		// Bundle any required ERC20 approvals + fillOrder into a single batch tx via ERC-7821 execute
		const callData = await this.contractService.buildApprovalAndFillCalldata(
			order,
			cachedFillerOutputs,
			fillOptions,
			dispatchFee,
		)

		// Total ETH to forward: native outputs + dispatch fee
		const nativeValue =
			cachedFillerOutputs.reduce((acc: bigint, output: TokenInfo) => {
				if (bytes32ToBytes20(output.token) === ADDRESS_ZERO) {
					return acc + output.amount
				}
				return acc
			}, 0n) + nativeDispatchFee

		const fillerAddress = this.signer.account.address
		const tx = await walletClient
			.sendTransaction({
				to: fillerAddress,
				data: callData,
				value: nativeValue,
				chain: walletClient.chain,
				gas: callGasLimit + (callGasLimit * 2500n) / 10000n,
			})
			.catch(async (err) => {
				this.logger.error({ err }, "Could not send transaction")
				return await walletClient.sendTransaction({
					to: fillerAddress,
					data: callData,
					value: nativeValue,
					chain: walletClient.chain,
				})
			})

		const endTime = Date.now()
		const processingTimeMs = endTime - startTime

		const receipt = await destClient.waitForTransactionReceipt({ hash: tx, confirmations: 1 })

		if (receipt.status !== "success") {
			this.logger.error({ txHash: receipt.transactionHash, status: receipt.status }, "Could not fill order")
			return {
				success: false,
				txHash: tx,
			}
		}

		return {
			success: true,
			txHash: receipt.transactionHash,
			gasUsed: receipt.gasUsed.toString(),
			gasPrice: receipt.effectiveGasPrice.toString(),
			confirmedAtBlock: Number(receipt.blockNumber),
			confirmedAt: new Date(),
			strategyUsed: this.name,
			processingTimeMs,
		}
	}
}
