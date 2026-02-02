import { toHex, maxUint256, formatUnits } from "viem"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import {
	ADDRESS_ZERO,
	HexString,
	bytes32ToBytes20,
	getGasPriceFromEtherscan,
	USE_ETHERSCAN_CHAINS,
	retryPromise,
	OrderV2,
	IntentGatewayV2,
	EvmChain,
	getChainId,
	orderV2Commitment,
	encodeUserOpScale,
	type PackedUserOperation,
	type FillOptionsV2,
} from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { ApiPromise } from "@polkadot/api"
import { CacheService } from "./CacheService"
import { getLogger } from "@/services/Logger"
import { Decimal } from "decimal.js"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"

// Configure for financial precision
Decimal.config({ precision: 28, rounding: 4 })
/**
 * Handles contract interactions for tokens and other contracts
 */
export class ContractInteractionService {
	private configService: FillerConfigService
	public cacheService: CacheService
	private logger = getLogger("contract-service")
	private sdkHelperCache: Map<string, IntentGatewayV2> = new Map()
	private solverAccountAddress: HexString

	constructor(
		private clientManager: ChainClientManager,
		private privateKey: HexString,
		configService: FillerConfigService,
		sharedCacheService?: CacheService,
	) {
		this.configService = configService
		this.cacheService = sharedCacheService || new CacheService()
		this.solverAccountAddress = privateKeyToAddress(this.privateKey)
		this.initCache()
	}

	/**
	 * Gets the SDK helper for a given source and destination chain.
	 * Instances are cached and reused to avoid redundant RPC calls.
	 */
	async getSdkHelper(source: string, destination: string): Promise<IntentGatewayV2> {
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

		const helper = new IntentGatewayV2(sourceEvmChain, destinationEvmChain)
		await helper.ensureInitialized()
		this.sdkHelperCache.set(cacheKey, helper)

		this.logger.debug({ source, destination }, "Created and cached new IntentGatewayV2 instance")

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
	 * Approves ERC20 tokens for the contract if needed
	 */
	async approveTokensIfNeeded(order: OrderV2): Promise<void> {
		const wallet = privateKeyToAccount(this.privateKey)
		const destClient = this.clientManager.getPublicClient(order.destination)
		const walletClient = this.clientManager.getWalletClient(order.destination)
		const intentGateway = this.configService.getIntentGatewayV2Address(order.destination)

		const tokens = [
			...new Set(
				order.output.assets.map((o) => bytes32ToBytes20(o.token)).filter((addr) => addr !== ADDRESS_ZERO),
			),
			(await this.getFeeTokenWithDecimals(order.destination)).address,
		].map((address) => ({
			address,
			amount: order.output.assets.find((o) => bytes32ToBytes20(o.token) === address)?.amount || maxUint256 / 2n,
		}))

		for (const token of tokens) {
			const allowance = await retryPromise(
				() =>
					destClient.readContract({
						abi: ERC20_ABI,
						address: token.address as HexString,
						functionName: "allowance",
						args: [wallet.address, intentGateway],
					}),
				{
					maxRetries: 3,
					backoffMs: 250,
					logMessage: "Failed to get token allowance",
				},
			)

			if (allowance < token.amount) {
				this.logger.info({ token: token.address }, "Approving token")
				const gasPrice = await destClient.getGasPrice()
				const tx = await walletClient.writeContract({
					abi: ERC20_ABI,
					address: token.address as HexString,
					functionName: "approve",
					args: [intentGateway, maxUint256],
					account: wallet,
					chain: walletClient.chain,
					gasPrice: gasPrice + (gasPrice * 2000n) / 10000n,
				})

				await retryPromise(() => destClient.waitForTransactionReceipt({ hash: tx }), {
					maxRetries: 3,
					backoffMs: 250,
					logMessage: "Failed while waiting for approval transaction receipt",
				})
				this.logger.info({ token: token.address }, "Approved token")
			}
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
			const cachedEstimate = this.cacheService.getGasEstimate(order.id!)
			if (cachedEstimate) {
				return {
					totalCostInSourceFeeToken: cachedEstimate.totalCostInSourceFeeToken,
					dispatchFee: cachedEstimate.dispatchFee,
					nativeDispatchFee: cachedEstimate.nativeDispatchFee,
					callGasLimit: cachedEstimate.callGasLimit,
				}
			}
			const sdkHelper = await this.getSdkHelper(order.source, order.destination)
			const estimate = await sdkHelper.estimateFillOrderV2({
				order,
				solverAccountAddress: this.solverAccountAddress,
			})
			// Cache the full estimate including gas parameters for bid preparation
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
	 * Calculates the total USD value of tokens in an order's inputs and outputs.
	 *
	 * @param order - The order to calculate token values for
	 * @returns An object containing the total USD values of outputs and inputs
	 */
	async getTokenUsdValue(order: OrderV2): Promise<{ outputUsdValue: Decimal; inputUsdValue: Decimal }> {
		let outputUsdValue = new Decimal(0)
		let inputUsdValue = new Decimal(0)
		const outputs = order.output.assets
		const inputs = order.inputs

		// Restrict to only USDC and USDT on both sides; otherwise throw error
		const destUsdc = this.configService.getUsdcAsset(order.destination).toLowerCase()
		const destUsdt = this.configService.getUsdtAsset(order.destination).toLowerCase()
		const sourceUsdc = this.configService.getUsdcAsset(order.source).toLowerCase()
		const sourceUsdt = this.configService.getUsdtAsset(order.source).toLowerCase()

		const outputsAreStableOnly = outputs.every((o) => {
			const addr = bytes32ToBytes20(o.token).toLowerCase()
			return addr === destUsdc || addr === destUsdt
		})
		const inputsAreStableOnly = inputs.every((i) => {
			const addr = bytes32ToBytes20(i.token).toLowerCase()
			return addr === sourceUsdc || addr === sourceUsdt
		})

		if (!outputsAreStableOnly || !inputsAreStableOnly) {
			throw new Error("Only USDC and USDT are supported for token value calculation")
		}

		// For stables, USD value equals the normalized token amount (peg ~ $1)
		for (const output of outputs) {
			const tokenAddress = bytes32ToBytes20(output.token)
			const decimals = await this.getTokenDecimals(tokenAddress, order.destination)
			const amount = output.amount
			const tokenAmount = new Decimal(formatUnits(amount, decimals))
			outputUsdValue = outputUsdValue.plus(tokenAmount)
		}

		for (const input of inputs) {
			const tokenAddress = bytes32ToBytes20(input.token)
			const decimals = await this.getTokenDecimals(tokenAddress, order.source)
			const amount = input.amount
			const tokenAmount = new Decimal(formatUnits(amount, decimals))
			inputUsdValue = inputUsdValue.plus(tokenAmount)
		}

		return { outputUsdValue, inputUsdValue }
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

		const sdkHelper = await this.getSdkHelper(order.source, order.destination)

		const fillOptions: FillOptionsV2 = {
			relayerFee: cachedEstimate.dispatchFee,
			nativeDispatchFee: cachedEstimate.nativeDispatchFee,
			outputs: cachedFillerOutputs,
		}

		const commitment = orderV2Commitment(order)

		// Fetch the current nonce from EntryPoint
		// ERC-4337 v0.7+ uses 2D nonce: getNonce(address sender, uint192 key)
		// The key cannot exceed 192 bits (24 bytes)
		const destClient = this.clientManager.getPublicClient(order.destination)
		const nonce = await destClient.readContract({
			address: entryPointAddress,
			abi: [
				{
					inputs: [
						{ name: "sender", type: "address" },
						{ name: "key", type: "uint192" },
					],
					name: "getNonce",
					outputs: [{ name: "nonce", type: "uint256" }],
					stateMutability: "view",
					type: "function",
				},
			],
			functionName: "getNonce",
			args: [solverAccountAddress, BigInt(commitment) & ((1n << 192n) - 1n)],
		})

		const userOp = await sdkHelper.prepareSubmitBid({
			order,
			fillOptions,
			solverAccount: solverAccountAddress,
			solverPrivateKey: this.privateKey,
			nonce,
			entryPointAddress,
			callGasLimit: cachedEstimate.callGasLimit,
			verificationGasLimit: cachedEstimate.verificationGasLimit,
			preVerificationGas: cachedEstimate.preVerificationGas,
			maxFeePerGas: cachedEstimate.maxFeePerGas,
			maxPriorityFeePerGas: cachedEstimate.maxPriorityFeePerGas,
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
}
