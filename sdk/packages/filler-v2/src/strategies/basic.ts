import { FillerStrategy } from "@/strategies/base"
import {
	OrderV2,
	ExecutionResult,
	HexString,
	bytes32ToBytes20,
	FillOptionsV2,
	ADDRESS_ZERO,
	TokenInfoV2,
	adjustDecimals,
	IntentsCoprocessor,
	transformOrderForContract,
} from "@hyperbridge/sdk"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { privateKeyToAccount } from "viem/accounts"
import { ChainClientManager, ContractInteractionService, BidStorageService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"

/** Supported token types for same-token execution */
type SupportedTokenType = "USDT" | "USDC"

export class BasicFiller implements FillerStrategy {
	name = "BasicFiller"
	private privateKey: HexString
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	private bidStorage?: BidStorageService
	private fillerBps: bigint
	private logger = getLogger("basic-filler")

	constructor(
		privateKey: HexString,
		configService: FillerConfigService,
		clientManager: ChainClientManager,
		contractService: ContractInteractionService,
		fillerBps: number,
		bidStorage?: BidStorageService,
	) {
		this.privateKey = privateKey
		this.configService = configService
		this.clientManager = clientManager
		this.contractService = contractService
		this.bidStorage = bidStorage
		this.fillerBps = BigInt(fillerBps)
	}

	/**
	 * Determines if this strategy can fill the given order.
	 * Validates that the order has supported token pairs (same-token swaps only).
	 * @param order The order to check
	 * @returns True if the strategy can fill the order
	 */
	async canFill(order: OrderV2): Promise<boolean> {
		try {
			// Validate basic structure
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
	 * based on the configured bps (basis points).
	 * @param order The order to calculate the USD value for
	 * @returns The profit in USD (Number), or 0 if not profitable or output amounts don't meet minimum
	 */
	async calculateProfitability(order: OrderV2): Promise<number> {
		try {
			const { decimals: destFeeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(
				order.destination,
			)

			// Validate that order outputs meet filler's minimum bps requirements
			// and calculate profit from slippage (normalized to dest fee token decimals)
			const { isValid, profitFromSlippage } = await this.calculateSlippageProfit(order, destFeeTokenDecimals)
			if (!isValid) {
				this.logger.info(
					{ orderId: order.id, fillerBps: this.fillerBps.toString() },
					"User expects more output than filler can provide based on bps",
				)
				return 0
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
					orderFeesUSD: formatUnits(order.fees, destFeeTokenDecimals),
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
	 * @returns Object with isValid boolean and profitFromSlippage (normalized to specified decimals)
	 */
	private async calculateSlippageProfit(
		order: OrderV2,
		normalizeToDecimals: number,
	): Promise<{ isValid: boolean; profitFromSlippage: bigint }> {
		const basisPoints = 10000n
		let totalProfitNormalized = 0n
		const fillerOutputs: TokenInfoV2[] = []

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
			const fillerMaxOutput = (convertedInputAmount * (basisPoints - this.fillerBps)) / basisPoints

			// Reject if user expects more than filler can provide
			if (output.amount > fillerMaxOutput) {
				this.logger.debug(
					{
						index: i,
						inputAmount: input.amount.toString(),
						inputDecimals,
						userExpects: output.amount.toString(),
						fillerWillProvide: fillerMaxOutput.toString(),
						outputDecimals,
						fillerBps: this.fillerBps.toString(),
					},
					"User expects more than filler can provide based on bps",
				)
				return { isValid: false, profitFromSlippage: 0n }
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
	 * Executes the order fill.
	 * If hyperbridge is provided, submits a bid (solver selection mode).
	 * Otherwise, fills the order directly via contract call.
	 *
	 * @param order The order to fill
	 * @param hyperbridge HyperbridgeService for bid submission (provided when solver selection is active)
	 * @returns The execution result
	 */
	async executeOrder(order: OrderV2, intentsCoprocessor?: IntentsCoprocessor): Promise<ExecutionResult> {
		const startTime = Date.now()

		// Ensure tokens are approved before submitting bid or direct fill
		await this.contractService.approveTokensIfNeeded(order)

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
		order: OrderV2,
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
		const solverAccountAddress = privateKeyToAccount(this.privateKey).address as HexString

		this.logger.info({ orderId: order.id, destination: order.destination }, "Submitting bid to Hyperbridge")

		// Prepare the signed UserOp for bid submission
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

			// Store successful bid for later cleanup/fund recovery
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
				processingTimeMs,
			}
		}
		this.logger.error({ commitment, error: bidResult.error }, "Failed to submit bid to Hyperbridge")

		// Store failed bid for debugging/analysis
		this.bidStorage?.storeBid({
			commitment,
			success: false,
			error: bidResult.error,
		})

		return {
			success: false,
			error: bidResult.error,
		}
	}

	/**
	 * Fills the order directly via contract call (non-solver selection mode)
	 * Uses cached filler outputs (calculated based on bps) instead of order.output.assets
	 * @private
	 */
	private async fillOrder(order: OrderV2, startTime: number): Promise<ExecutionResult> {
		const { destClient, walletClient } = this.clientManager.getClientsForOrder(order)

		const { dispatchFee, nativeDispatchFee, callGasLimit } = await this.contractService.estimateGasFillPost(order)

		// Use cached filler outputs (calculated based on bps) for competitive filling
		const cachedFillerOutputs = this.contractService.cacheService.getFillerOutputs(order.id!)
		if (!cachedFillerOutputs) {
			throw new Error(`No cached filler outputs found for order ${order.id}. Call calculateProfitability first.`)
		}

		const fillOptions: FillOptionsV2 = {
			relayerFee: dispatchFee,
			nativeDispatchFee: nativeDispatchFee,
			outputs: cachedFillerOutputs,
		}

		// Add all eth values from the filler's calculated outputs
		const ethValue = cachedFillerOutputs.reduce((acc: bigint, output: TokenInfoV2) => {
			if (bytes32ToBytes20(output.token) === ADDRESS_ZERO) {
				return acc + output.amount
			}
			return acc
		}, 0n)

		const tx = await walletClient
			.writeContract({
				abi: INTENT_GATEWAY_V2_ABI,
				address: this.configService.getIntentGatewayV2Address(order.destination),
				functionName: "fillOrder",
				args: [transformOrderForContract(order) as any, fillOptions as any],
				account: privateKeyToAccount(this.privateKey),
				value: nativeDispatchFee !== 0n ? ethValue + nativeDispatchFee : ethValue,
				chain: walletClient.chain,
				gas: callGasLimit + (callGasLimit * 2500n) / 10000n,
			})
			.catch(async () => {
				return await walletClient.writeContract({
					abi: INTENT_GATEWAY_V2_ABI,
					address: this.configService.getIntentGatewayV2Address(order.destination),
					functionName: "fillOrder",
					args: [transformOrderForContract(order) as any, fillOptions as any],
					account: privateKeyToAccount(this.privateKey),
					value: nativeDispatchFee !== 0n ? ethValue + nativeDispatchFee : ethValue,
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
