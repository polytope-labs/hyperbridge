import { toHex, formatUnits, encodeFunctionData, maxUint256, formatEther } from "viem"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import {
	ADDRESS_ZERO,
	HexString,
	bytes32ToBytes20,
	retryPromise,
	OrderV2,
	IntentsV2,
	EvmChain,
	getChainId,
	orderV2Commitment,
	encodeUserOpScale,
	type PackedUserOperation,
	type FillOptionsV2,
	encodeERC7821ExecuteBatch,
	type ERC7821Call,
	transformOrderForContract,
	TokenInfoV2,
} from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { CacheService } from "./CacheService"
import { getLogger } from "@/services/Logger"
import { Decimal } from "decimal.js"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"

// Configure for financial precision
Decimal.config({ precision: 28, rounding: 4 })
/**
 * Handles contract interactions for tokens and other contracts
 */
export class ContractInteractionService {
	private configService: FillerConfigService
	public cacheService: CacheService
	private logger = getLogger("contract-service")
	private sdkHelperCache: Map<string, IntentsV2> = new Map()
	private solverAccountAddress: HexString
	private account: ReturnType<typeof privateKeyToAccount>

	constructor(
		private clientManager: ChainClientManager,
		private privateKey: HexString,
		configService: FillerConfigService,
		sharedCacheService?: CacheService,
	) {
		this.configService = configService
		this.cacheService = sharedCacheService || new CacheService()
		this.solverAccountAddress = privateKeyToAddress(this.privateKey)
		this.account = privateKeyToAccount(this.privateKey)
		this.initCache()
	}

	/**
	 * Gets the SDK helper for a given source and destination chain.
	 * Instances are cached and reused to avoid redundant RPC calls.
	 */
	async getIntentsV2(source: string, destination: string): Promise<IntentsV2> {
		const cacheKey = `${source}:${destination}`

		const cached = this.sdkHelperCache.get(cacheKey)
		if (cached) {
			return cached
		}

		const sourceClient = this.clientManager.getPublicClient(source)
		const destinationClient = this.clientManager.getPublicClient(destination)
		const sourceEvmChain = new EvmChain({
			chainId: getChainId(source)!,
			host: this.configService.getHostAddress(source),
			rpcUrl: sourceClient.transport.url,
		})
		const destinationEvmChain = new EvmChain({
			chainId: getChainId(destination)!,
			host: this.configService.getHostAddress(destination),
			rpcUrl: destinationClient.transport.url,
		})

		// Pass bundlerUrl to IntentGatewayV2 for accurate gas estimation via eth_estimateUserOperationGas
		const bundlerUrl = this.configService.getBundlerUrl(source)
		const helper = await IntentsV2.create(sourceEvmChain, destinationEvmChain, undefined, bundlerUrl)
		this.sdkHelperCache.set(cacheKey, helper)

		this.logger.debug(
			{ source, destination, bundlerUrl: bundlerUrl ? "[configured]" : undefined },
			"Created and cached new IntentGatewayV2 instance",
		)

		return helper
	}

	async initCache(): Promise<void> {
		const chainIds = this.configService.getConfiguredChainIds()
		const chainNames = chainIds.map((id) => `EVM-${id}`)
		for (const chainName of chainNames) {
			await this.getFeeTokenWithDecimals(chainName)
		}

		for (const destChain of chainNames) {
			const destClient = this.clientManager.getPublicClient(destChain)
			const usdc = this.configService.getUsdcAsset(destChain)
			const usdt = this.configService.getUsdtAsset(destChain)
			await this.getTokenDecimals(usdc, destChain)
			await this.getTokenDecimals(usdt, destChain)
			for (const sourceChain of chainNames) {
				// Same-chain intents don't dispatch cross-chain messages, so perByteFee is not needed.
				// The SDK's estimateFillOrderV2 skips quoteNative for same-chain orders.
				if (sourceChain === destChain) continue
				// Check cache before making RPC call to avoid duplicate requests when cache is shared
				const cachedPerByteFee = this.cacheService.getPerByteFee(destChain, sourceChain)
				if (cachedPerByteFee === null) {
					const perByteFee = await retryPromise(
						() =>
							destClient.readContract({
								address: this.configService.getHostAddress(destChain),
								abi: EVM_HOST,
								functionName: "perByteFee",
								args: [toHex(sourceChain)],
							}),
						{
							maxRetries: 3,
							backoffMs: 250,
							logMessage: "Failed to load perByteFee for cache initialization",
						},
					)
					this.cacheService.setPerByteFee(destChain, sourceChain, perByteFee)
				}
			}
		}
	}

	getCache(): CacheService {
		return this.cacheService
	}

	/**
	 * Gets the decimals for a token
	 */
	async getTokenDecimals(tokenAddress: string, chain: string): Promise<number> {
		const bytes20Address = tokenAddress.length === 66 ? bytes32ToBytes20(tokenAddress) : tokenAddress

		if (bytes20Address === ADDRESS_ZERO) {
			return 18 // Native token (ETH, MATIC, etc.)
		}

		const cachedTokenDecimals = this.cacheService.getTokenDecimals(chain, bytes20Address as HexString)
		if (cachedTokenDecimals) {
			return cachedTokenDecimals
		}

		const client = this.clientManager.getPublicClient(chain)

		try {
			const decimals = await retryPromise(
				() =>
					client.readContract({
						address: bytes20Address as HexString,
						abi: ERC20_ABI,
						functionName: "decimals",
					}),
				{
					maxRetries: 3,
					backoffMs: 250,
					logMessage: "Failed to get token decimals",
				},
			)

			this.cacheService.setTokenDecimals(chain, bytes20Address as HexString, decimals)
			return decimals
		} catch (error) {
			this.logger.warn({ err: error }, "Error getting token decimals, defaulting to 18")
			return 18 // Default to 18 if we can't determine
		}
	}

	/**
	 * Estimates gas for filling an order and caches the full estimate for bid preparation
	 */
	async estimateGasFillPost(order: OrderV2): Promise<{
		totalCostInSourceFeeToken: bigint
		dispatchFee: bigint
		nativeDispatchFee: bigint
		callGasLimit: bigint
	}> {
		try {
			const client = this.clientManager.getPublicClient(order.destination)
			const cachedEstimate = this.cacheService.getGasEstimate(order.id!)
			if (cachedEstimate) {
				return {
					totalCostInSourceFeeToken: cachedEstimate.totalCostInSourceFeeToken,
					dispatchFee: cachedEstimate.dispatchFee,
					nativeDispatchFee: cachedEstimate.nativeDispatchFee,
					callGasLimit: cachedEstimate.callGasLimit,
				}
			}

			const sdkHelper = await this.getIntentsV2(order.source, order.destination)
			const gasFeeBumpConfig = this.configService.getGasFeeBumpConfig()
			const estimate = await sdkHelper.estimateFillOrderV2({
				order,
				maxPriorityFeePerGasBumpPercent: gasFeeBumpConfig?.maxPriorityFeePerGasBumpPercent,
				maxFeePerGasBumpPercent: gasFeeBumpConfig?.maxFeePerGasBumpPercent,
			})

			const nonce = await client.readContract({
				address: this.configService.getEntryPointAddress(order.destination)!,
				abi: ENTRYPOINT_ABI,
				functionName: "getNonce",
				args: [this.solverAccountAddress, BigInt(orderV2Commitment(order)) & ((1n << 192n) - 1n)],
			})

			this.logger.info({ orderId: order.id }, "Caching gas estimate")
			this.logger.info({ estimate }, "Estimate")
			this.cacheService.setGasEstimate(
				order.id!,
				estimate.totalGasInFeeToken,
				estimate.fillOptions.relayerFee,
				estimate.fillOptions.nativeDispatchFee,
				estimate.callGasLimit,
				estimate.verificationGasLimit,
				estimate.preVerificationGas,
				estimate.maxFeePerGas,
				estimate.maxPriorityFeePerGas,
				nonce,
				estimate.totalGasCostWei,
			)
			return {
				totalCostInSourceFeeToken: estimate.totalGasInFeeToken,
				dispatchFee: estimate.fillOptions.relayerFee,
				nativeDispatchFee: estimate.fillOptions.nativeDispatchFee,
				callGasLimit: estimate.callGasLimit,
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error estimating gas, using generous fallback values")
			throw new Error(`Failed to estimate gas: ${error instanceof Error ? error.message : "Unknown error"}`)
		}
	}

	/**
	 * Gets the fee token address and decimals for a given chain.
	 *
	 * @param chain - The chain identifier to get fee token info for
	 * @returns An object containing the fee token address and its decimal places
	 */
	async getFeeTokenWithDecimals(chain: string): Promise<{ address: HexString; decimals: number }> {
		const cachedFeeToken = this.cacheService.getFeeTokenWithDecimals(chain)
		if (cachedFeeToken) {
			return cachedFeeToken
		}
		const client = this.clientManager.getPublicClient(chain)
		const feeTokenAddress = await retryPromise(
			() =>
				client.readContract({
					abi: EVM_HOST,
					address: this.configService.getHostAddress(chain),
					functionName: "feeToken",
				}),
			{
				maxRetries: 3,
				backoffMs: 250,
				logMessage: "Failed to get fee token address",
			},
		)
		const feeTokenDecimals = await retryPromise(
			() =>
				client.readContract({
					address: feeTokenAddress,
					abi: ERC20_ABI,
					functionName: "decimals",
				}),
			{
				maxRetries: 3,
				backoffMs: 250,
				logMessage: "Failed to get fee token decimals",
			},
		)
		this.cacheService.setFeeTokenWithDecimals(chain, feeTokenAddress, feeTokenDecimals)
		return { address: feeTokenAddress, decimals: feeTokenDecimals }
	}

	/**
	 * Ensures the solver's EntryPoint deposit has enough native token to cover
	 * the estimated gas cost for a given order.
	 *
	 * Uses cached gas estimates (from estimateGasFillPost) and, if the current
	 * deposit is insufficient, tops up by depositing 10% of the solver's EOA
	 * native balance on the destination chain.
	 */
	async ensureEntryPointDeposit(order: OrderV2): Promise<void> {
		if (!order.id) {
			this.logger.warn({ destination: order.destination }, "Order has no ID, skipping EntryPoint deposit check")
			return
		}

		const gasEstimate = this.cacheService.getGasEstimate(order.id)
		if (!gasEstimate) {
			this.logger.warn(
				{ orderId: order.id, destination: order.destination },
				"No cached gas estimate found, skipping EntryPoint deposit check",
			)
			return
		}

		const requiredNative = 3n * gasEstimate.totalGasCostWei

		const currentDeposit = await this.getSolverEntryPointBalance(order.destination)

		this.logger.debug(
			{
				orderId: order.id,
				destination: order.destination,
				currentDeposit: formatEther(currentDeposit),
				requiredNative: formatEther(requiredNative),
			},
			"EntryPoint deposit gas coverage check",
		)

		if (currentDeposit >= requiredNative) {
			return
		}

		const publicClient = this.clientManager.getPublicClient(order.destination)
		const solverBalance = await publicClient.getBalance({ address: this.solverAccountAddress })
		const depositAmount = solverBalance / 10n

		if (depositAmount === 0n) {
			this.logger.warn(
				{
					orderId: order.id,
					destination: order.destination,
					solverBalance: formatEther(solverBalance),
				},
				"Solver EOA balance too low to top up EntryPoint deposit",
			)
			return
		}

		this.logger.info(
			{
				orderId: order.id,
				destination: order.destination,
				requiredNative: formatEther(requiredNative),
				currentDeposit: formatEther(currentDeposit),
				solverBalance: formatEther(solverBalance),
				depositAmount: formatEther(depositAmount),
			},
			"Top up EntryPoint deposit by 10% of solver EOA balance",
		)

		await this.depositToEntryPoint(order.destination, depositAmount)
	}

	/**
	 * Calculates the total USD value of an order's inputs.
	 * Only stable (USDC/USDT) inputs contribute; non-stables contribute 0.
	 *
	 * @param order - The order to calculate input value for
	 * @returns The total USD value of inputs (sum of normalized stable amounts, or 0 if none)
	 */
	async getInputUsdValue(order: OrderV2): Promise<Decimal> {
		let inputUsdValue = new Decimal(0)
		const inputs = order.inputs
		const sourceUsdc = this.configService.getUsdcAsset(order.source).toLowerCase()
		const sourceUsdt = this.configService.getUsdtAsset(order.source).toLowerCase()

		for (const input of inputs) {
			const tokenAddress = bytes32ToBytes20(input.token)
			const addr = tokenAddress.toLowerCase()
			if (addr !== sourceUsdc && addr !== sourceUsdt) continue
			const decimals = await this.getTokenDecimals(tokenAddress, order.source)
			const tokenAmount = new Decimal(formatUnits(input.amount, decimals))
			inputUsdValue = inputUsdValue.plus(tokenAmount)
		}

		return inputUsdValue
	}

	/**
	 * Checks if solver selection mode is active on the destination chain
	 * When active, fillers must submit bids to Hyperbridge instead of filling directly
	 *
	 * @param chain - The chain identifier to check
	 * @returns True if solver selection is active
	 */
	async isSolverSelectionActive(chain: string): Promise<boolean> {
		const cached = this.cacheService.getSolverSelection(chain)
		if (cached !== null) {
			return cached
		}

		const client = this.clientManager.getPublicClient(chain)
		const params = await client.readContract({
			abi: INTENT_GATEWAY_V2_ABI,
			functionName: "params",
			address: this.configService.getIntentGatewayV2Address(chain),
		})

		this.cacheService.setSolverSelection(chain, params.solverSelection)
		return params.solverSelection
	}

	/**
	 * Reads the solver account's deposit balance on the ERC-4337 EntryPoint.
	 */
	async getSolverEntryPointBalance(chain: string): Promise<bigint> {
		const entryPointAddress = this.configService.getEntryPointAddress(chain)
		if (!entryPointAddress) {
			throw new Error(`EntryPoint not configured for chain ${chain}`)
		}

		const client = this.clientManager.getPublicClient(chain)
		return retryPromise(
			() =>
				client.readContract({
					address: entryPointAddress,
					abi: ENTRYPOINT_ABI,
					functionName: "balanceOf",
					args: [this.solverAccountAddress],
				}),
			{ maxRetries: 3, backoffMs: 250, logMessage: "Failed to read EntryPoint balance" },
		)
	}

	/**
	 * Deposits native tokens to the ERC-4337 EntryPoint on behalf of the solver account.
	 * @returns The transaction hash of the deposit.
	 */
	async depositToEntryPoint(chain: string, amount: bigint): Promise<HexString> {
		const entryPointAddress = this.configService.getEntryPointAddress(chain)
		if (!entryPointAddress) {
			throw new Error(`EntryPoint not configured for chain ${chain}`)
		}

		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)

		this.logger.info(
			{ chain, solver: this.solverAccountAddress, amount: formatEther(amount) },
			"Depositing to EntryPoint",
		)

		const hash = await walletClient.writeContract({
			address: entryPointAddress,
			abi: ENTRYPOINT_ABI,
			functionName: "depositTo",
			args: [this.solverAccountAddress],
			value: amount,
			chain: walletClient.chain,
			account: this.account,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash })
		if (receipt.status !== "success") {
			throw new Error(`EntryPoint deposit transaction reverted: ${hash}`)
		}

		this.logger.info({ chain, txHash: hash, amount: formatEther(amount) }, "EntryPoint deposit confirmed")
		return hash as HexString
	}

	/**
	 * Withdraws the solver's full EntryPoint deposit back to the solver EOA on a single chain.
	 * @returns The transaction hash, or null if there was nothing to withdraw.
	 */
	async withdrawFromEntryPoint(chain: string): Promise<HexString | null> {
		const entryPointAddress = this.configService.getEntryPointAddress(chain)
		if (!entryPointAddress) {
			throw new Error(`EntryPoint not configured for chain ${chain}`)
		}

		const balance = await this.getSolverEntryPointBalance(chain)
		if (balance === 0n) {
			this.logger.debug({ chain }, "No EntryPoint deposit to withdraw")
			return null
		}

		const walletClient = this.clientManager.getWalletClient(chain)
		const publicClient = this.clientManager.getPublicClient(chain)

		this.logger.info(
			{ chain, solver: this.solverAccountAddress, amount: formatEther(balance) },
			"Withdrawing from EntryPoint",
		)

		const hash = await walletClient.writeContract({
			address: entryPointAddress,
			abi: ENTRYPOINT_ABI,
			functionName: "withdrawTo",
			args: [this.solverAccountAddress, balance],
			chain: walletClient.chain,
			account: this.account,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash })
		if (receipt.status !== "success") {
			throw new Error(`EntryPoint withdrawal transaction reverted: ${hash}`)
		}

		this.logger.info({ chain, txHash: hash, amount: formatEther(balance) }, "EntryPoint withdrawal confirmed")
		return hash as HexString
	}

	/**
	 * Withdraws EntryPoint deposits on all configured chains that have a positive balance.
	 */
	async withdrawAllEntryPointDeposits(): Promise<void> {
		const chainIds = this.configService.getConfiguredChainIds()

		for (const chainId of chainIds) {
			const chain = `EVM-${chainId}`
			const entryPointAddress = this.configService.getEntryPointAddress(chain)
			if (!entryPointAddress) continue

			try {
				await this.withdrawFromEntryPoint(chain)
			} catch (error) {
				this.logger.error({ chain, err: error }, "Failed to withdraw EntryPoint deposit")
			}
		}
	}

	/**
	 * Prepares a signed PackedUserOperation for bid submission to Hyperbridge
	 *
	 * Uses cached gas estimates from prior profitability check (estimateGasFillPost)
	 * to avoid redundant RPC calls.
	 *
	 * @param order - The order to prepare a bid for
	 * @param entryPointAddress - The ERC-4337 EntryPoint address on the destination chain
	 * @param solverAccountAddress - The solver's smart account address
	 * @returns Object containing the commitment and encoded UserOp
	 */
	async prepareBidUserOp(
		order: OrderV2,
		entryPointAddress: HexString,
		solverAccountAddress: HexString,
	): Promise<{ commitment: HexString; userOp: HexString }> {
		// Use cached estimate from prior profitability check
		const cachedEstimate = this.cacheService.getGasEstimate(order.id!)
		if (!cachedEstimate) {
			throw new Error(`No cached gas estimate found for order ${order.id}. Call estimateGasFillPost first.`)
		}

		// Use cached filler outputs (calculated based on bps) for competitive bidding
		const cachedFillerOutputs = this.cacheService.getFillerOutputs(order.id!)

		if (!cachedFillerOutputs) {
			throw new Error(`No cached filler outputs found for order ${order.id}. Call calculateProfitability first.`)
		}

		const sdkHelper = await this.getIntentsV2(order.source, order.destination)

		const fillOptions: FillOptionsV2 = {
			relayerFee: cachedEstimate.dispatchFee,
			nativeDispatchFee: cachedEstimate.nativeDispatchFee,
			outputs: cachedFillerOutputs,
		}

		const callData = await this.buildApprovalAndFillCalldata(
			order,
			cachedFillerOutputs,
			fillOptions,
			cachedEstimate.totalCostInSourceFeeToken,
		)

		const commitment = orderV2Commitment(order)

		const userOp = await sdkHelper.prepareSubmitBid({
			order,
			fillOptions,
			solverAccount: solverAccountAddress,
			solverPrivateKey: this.privateKey,
			nonce: cachedEstimate.nonce,
			entryPointAddress,
			callGasLimit: cachedEstimate.callGasLimit,
			verificationGasLimit: cachedEstimate.verificationGasLimit,
			preVerificationGas: cachedEstimate.preVerificationGas,
			maxFeePerGas: cachedEstimate.maxFeePerGas,
			maxPriorityFeePerGas: cachedEstimate.maxPriorityFeePerGas,
			callData,
		})

		// Encode the UserOp as bytes for submission to Hyperbridge
		const encodedUserOp = encodeUserOpScale(userOp)

		this.logger.info(
			{
				commitment,
				solverAccount: solverAccountAddress,
				callGasLimit: cachedEstimate.callGasLimit.toString(),
				maxFeePerGas: cachedEstimate.maxFeePerGas.toString(),
			},
			"Prepared bid UserOp",
		)

		return { commitment, userOp: encodedUserOp }
	}

	/**
	 * Builds ERC-7821 batch calldata that prepends any required ERC20 approvals
	 * before the fillOrder call, all within a single UserOp payload.
	 */
	public async buildApprovalAndFillCalldata(
		order: OrderV2,
		fillerOutputs: TokenInfoV2[],
		fillOptions: FillOptionsV2,
		requiredFeeTokenAmount: bigint,
	): Promise<HexString> {
		const chain = order.destination
		const destClient = this.clientManager.getPublicClient(chain)
		const intentGatewayV2Address = this.configService.getIntentGatewayV2Address(chain)

		// Aggregate required amounts per ERC20 token
		const perTokenRequired = new Map<string, bigint>()
		for (const output of fillerOutputs) {
			const addr = bytes32ToBytes20(output.token)
			if (addr === ADDRESS_ZERO) continue
			const key = addr.toLowerCase()
			perTokenRequired.set(key, (perTokenRequired.get(key) ?? 0n) + output.amount)
		}

		const feeToken = await this.getFeeTokenWithDecimals(chain)
		const feeKey = feeToken.address.toLowerCase()
		perTokenRequired.set(feeKey, (perTokenRequired.get(feeKey) ?? 0n) + requiredFeeTokenAmount)

		// Check allowances in parallel
		const entries = [...perTokenRequired.entries()]
		const allowances = await Promise.all(
			entries.map(([tokenAddress]) =>
				destClient.readContract({
					abi: ERC20_ABI,
					address: tokenAddress as HexString,
					functionName: "allowance",
					args: [this.solverAccountAddress, intentGatewayV2Address],
				}),
			),
		)

		const calls: ERC7821Call[] = []
		for (const [i, [tokenAddress, required]] of entries.entries()) {
			if (allowances[i] < required) {
				calls.push({
					target: tokenAddress as HexString,
					value: 0n,
					data: encodeFunctionData({
						abi: ERC20_ABI,
						functionName: "approve",
						args: [intentGatewayV2Address, maxUint256],
					}) as HexString,
				})
			}
		}

		// Append fillOrder call (after any approvals, or as the sole call)
		const nativeOutputValue = fillerOutputs
			.filter((asset) => bytes32ToBytes20(asset.token) === ADDRESS_ZERO)
			.reduce((sum, asset) => sum + asset.amount, 0n)

		calls.push({
			target: intentGatewayV2Address,
			value: nativeOutputValue + fillOptions.nativeDispatchFee,
			data: encodeFunctionData({
				abi: INTENT_GATEWAY_V2_ABI,
				functionName: "fillOrder",
				args: [transformOrderForContract(order) as any, fillOptions as any],
			}) as HexString,
		})

		return encodeERC7821ExecuteBatch(calls)
	}
}
