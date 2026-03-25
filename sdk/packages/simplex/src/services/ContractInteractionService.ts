import { toHex, formatUnits, encodeFunctionData, maxUint256, formatEther } from "viem"
import {
	ADDRESS_ZERO,
	HexString,
	bytes32ToBytes20,
	retryPromise,
	Order,
	IntentGateway,
	EvmChain,
	getChainId,
	orderCommitment,
	encodeUserOpScale,
	type PackedUserOperation,
	type FillOptions,
	encodeERC7821ExecuteBatch,
	type ERC7821Call,
	transformOrderForContract,
	TokenInfo,
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
import type { SigningAccount } from "@/services/wallet"

// Configure for financial precision
Decimal.config({ precision: 28, rounding: 4 })
/**
 * Handles contract interactions for tokens and other contracts
 */
export class ContractInteractionService {
	private configService: FillerConfigService
	public cacheService: CacheService
	private logger = getLogger("contract-service")
	private sdkHelperCache: Map<string, IntentGateway> = new Map()
	private solverAccountAddress: HexString
	private signer: SigningAccount

	constructor(
		private clientManager: ChainClientManager,
		configService: FillerConfigService,
		signer: SigningAccount,
		sharedCacheService?: CacheService,
	) {
		this.configService = configService
		this.cacheService = sharedCacheService || new CacheService()
		this.signer = signer
		this.solverAccountAddress = this.signer.account.address
		this.initCache()
	}

	/**
	 * Gets the SDK helper for a given source and destination chain.
	 * Instances are cached and reused to avoid redundant RPC calls.
	 */
	async getIntentGateway(source: string, destination: string): Promise<IntentGateway> {
		const cacheKey = `${source}:${destination}`

		const cached = this.sdkHelperCache.get(cacheKey)
		if (cached) {
			return cached
		}

		const sourceClient = this.clientManager.getPublicClient(source)
		const destinationClient = this.clientManager.getPublicClient(destination)
		const sourceEvmChain = EvmChain.fromParams({
			chainId: getChainId(source)!,
			host: this.configService.getHostAddress(source),
			rpcUrl: sourceClient.transport.url,
		})
		const bundlerUrl = this.configService.getBundlerUrl(destination)
		const destinationEvmChain = EvmChain.fromParams({
			chainId: getChainId(destination)!,
			host: this.configService.getHostAddress(destination),
			rpcUrl: destinationClient.transport.url,
			bundlerUrl,
		})

		const helper = await IntentGateway.create(sourceEvmChain, destinationEvmChain)
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
				// The SDK's estimateFillOrder skips quoteNative for same-chain orders.
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
	async estimateGasFillPost(order: Order): Promise<{
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

			const sdkHelper = await this.getIntentGateway(order.source, order.destination)
			const gasFeeBumpConfig = this.configService.getGasFeeBumpConfig()
			const funding = this.cacheService.getFundingPrepends(order.id!)

			// NOTE: We intentionally do NOT pass funding prepend calls to the
			// estimation.  The V4 PositionManager's modifyLiquidities uses
			// flash-accounting and an internal msgSender() (via _getLocker) that
			// does not resolve correctly in the bundler's eth_estimateUserOperationGas
			// simulation context, causing FailedOpWithRevert.  Instead we estimate
			// without the prepends and apply a gas multiplier afterwards.
			const estimate = await sdkHelper.estimateFillOrder({
				order,
				prependCalls: undefined,
				maxPriorityFeePerGasBumpPercent: gasFeeBumpConfig?.maxPriorityFeePerGasBumpPercent,
				maxFeePerGasBumpPercent: gasFeeBumpConfig?.maxFeePerGasBumpPercent,
			})

			// If funding prepend calls are present, bump callGasLimit to account
			// for the extra V4 modifyLiquidities + take operations.  Each V4
			// decrease-liquidity + take-pair action costs roughly 200-350k gas.
			const FUNDING_GAS_PER_CALL = 400_000n
			const fundingGasBump = funding?.calls?.length ? FUNDING_GAS_PER_CALL * BigInt(funding.calls.length) : 0n

			const nonce = await client.readContract({
				address: this.configService.getEntryPointAddress(order.destination)!,
				abi: ENTRYPOINT_ABI,
				functionName: "getNonce",
				args: [this.solverAccountAddress, BigInt(orderCommitment(order)) & ((1n << 192n) - 1n)],
			})

			this.logger.info({ orderId: order.id }, "Caching gas estimate")
			this.logger.info({ estimate, fundingGasBump: fundingGasBump.toString() }, "Estimate")
			const callGasLimit = estimate.callGasLimit + fundingGasBump

			this.cacheService.setGasEstimate(
				order.id!,
				estimate.totalGasInFeeToken,
				estimate.fillOptions.relayerFee,
				estimate.fillOptions.nativeDispatchFee,
				callGasLimit,
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
	 * Tops up the solver's EntryPoint deposit so it covers at least
	 * `targetGasUnits` at the current gas price. Skips if the wallet
	 * balance cannot afford at least 1M gas units (not enough to send txs).
	 *
	 * @param chain - The chain identifier
	 * @param targetGasUnits - Gas units the deposit should cover (default 3M)
	 * @param thresholdGasUnits - Only top up if deposit is below this many gas units (defaults to targetGasUnits)
	 */
	async topUpEntryPointDeposit(chain: string, targetGasUnits: bigint = 3_000_000n, thresholdGasUnits?: bigint): Promise<void> {
		const effectiveThreshold = thresholdGasUnits ?? targetGasUnits
		const entryPointAddress = this.configService.getEntryPointAddress(chain)
		if (!entryPointAddress) {
			return
		}

		const publicClient = this.clientManager.getPublicClient(chain)
		const [currentDeposit, solverBalance, gasPrice] = await Promise.all([
			this.getSolverEntryPointBalance(chain),
			publicClient.getBalance({ address: this.solverAccountAddress }),
			publicClient.getGasPrice(),
		])

		if (gasPrice === 0n) {
			this.logger.warn({ chain }, "Gas price is zero, skipping EntryPoint top-up")
			return
		}

		// Skip if wallet can't afford at least 1M gas units
		const walletGasUnits = solverBalance / gasPrice
		const minWalletGasUnits = 1_000_000n

		if (walletGasUnits < minWalletGasUnits) {
			this.logger.warn(
				{
					chain,
					walletBalance: formatEther(solverBalance),
					walletGasUnits: walletGasUnits.toString(),
					gasPrice: gasPrice.toString(),
				},
				"Wallet balance too low to afford minimum gas, skipping EntryPoint top-up",
			)
			return
		}

		const targetDeposit = targetGasUnits * gasPrice
		const thresholdDeposit = effectiveThreshold * gasPrice
		const depositGasUnits = currentDeposit / gasPrice

		if (currentDeposit >= thresholdDeposit) {
			this.logger.info(
				{
					chain,
					currentDeposit: formatEther(currentDeposit),
					depositGasUnits: depositGasUnits.toString(),
					targetGasUnits: targetGasUnits.toString(),
					walletBalance: formatEther(solverBalance),
				},
				"EntryPoint deposit covers target gas units, no top-up needed",
			)
			return
		}

		const deficit = targetDeposit - currentDeposit

		if (solverBalance < deficit) {
			this.logger.warn(
				{
					chain,
					deficit: formatEther(deficit),
					solverBalance: formatEther(solverBalance),
					depositGasUnits: depositGasUnits.toString(),
					targetGasUnits: targetGasUnits.toString(),
				},
				"Solver EOA balance insufficient to reach target deposit, depositing available balance",
			)
			await this.depositToEntryPoint(chain, solverBalance)
			return
		}

		this.logger.info(
			{
				chain,
				currentDeposit: formatEther(currentDeposit),
				depositGasUnits: depositGasUnits.toString(),
				targetGasUnits: targetGasUnits.toString(),
				topUpAmount: formatEther(deficit),
			},
			"Topping up EntryPoint deposit to cover target gas units",
		)

		await this.depositToEntryPoint(chain, deficit)
	}

	/**
	 * Calculates the total USD value of an order's inputs.
	 * Only stable (USDC/USDT) inputs contribute; non-stables contribute 0.
	 *
	 * @param order - The order to calculate input value for
	 * @returns The total USD value of inputs (sum of normalized stable amounts, or 0 if none)
	 */
	async getInputUsdValue(order: Order): Promise<Decimal> {
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
		order: Order,
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

		const sdkHelper = await this.getIntentGateway(order.source, order.destination)

		const fillOptions: FillOptions = {
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

		const commitment = orderCommitment(order)

		const userOp = await sdkHelper.prepareSubmitBid({
			order,
			fillOptions,
			solverAccount: solverAccountAddress,
			solverSigner: this.signer,
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
		order: Order,
		fillerOutputs: TokenInfo[],
		fillOptions: FillOptions,
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

		const fundingPrepends = order.id ? this.cacheService.getFundingPrepends(order.id) : null
		const prependCalls = fundingPrepends?.calls ?? []

		const calls: ERC7821Call[] = [...prependCalls]
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
