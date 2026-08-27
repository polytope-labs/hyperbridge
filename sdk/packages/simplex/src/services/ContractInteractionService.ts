import { formatUnits, encodeFunctionData, formatEther } from "viem"
import {
	ADDRESS_ZERO,
	CryptoUtils,
	type HexString,
	bytes32ToBytes20,
	retryPromise,
	type Order,
	IntentGateway,
	EvmChain,
	getChainId,
	orderCommitment,
	encodePhantomBidDeclaration,
	encodeUserOpScale,
	type FillOptions,
	encodeERC7821ExecuteBatch,
	type ERC7821Call,
	transformOrderForContract,
	type TokenInfo,
	encodeFillOrder,
	getFillOptionsVersion,
} from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { ChainClientManager } from "./ChainClientManager"
import type { FillerConfigService } from "./FillerConfigService"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { CacheService } from "./CacheService"
import { type Logger , moduleLogger} from "@/services/Logger"
import { Decimal } from "decimal.js"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
import { sdkSigningAccount, type Signer } from "@/services/wallet"
import { buildPaymasterAndData } from "@/services/paymaster"

// Configure for financial precision
Decimal.config({ precision: 28, rounding: 4 })
/**
 * Handles contract interactions for tokens and other contracts
 */
export class ContractInteractionService {
	private configService: FillerConfigService
	public cacheService: CacheService
	private logger: Logger

	/** Chains already warned about a gateway with no validUntil support. */
	private readonly warnedNoValidUntil = new Set<string>()
	private sdkHelperCache: Map<string, IntentGateway> = new Map()
	private solverAccountAddress: HexString
	private signer: Signer

	constructor(
		private clientManager: ChainClientManager,
		configService: FillerConfigService,
		signer: Signer,
		sharedCacheService?: CacheService,
	) {
		this.logger = moduleLogger(configService.loggers, "contract-service")
		this.configService = configService
		this.cacheService = sharedCacheService || new CacheService()
		this.signer = signer
		this.solverAccountAddress = this.signer.address
		// `initCache` swallows its own failures; the guard is belt-and-braces so a future
		// change there can never become an unhandled rejection from an unawaited call.
		void this.initCache().catch((err) => this.logger.warn({ err }, "Token cache prewarm failed"))
	}

	/**
	 * The endpoint SDK helpers should use for `chain`: the first configured RPC,
	 * matching the one viem's `fallback` transport tries first.
	 */
	private primaryRpcUrl(chain: string): string {
		const [rpcUrl] = this.configService.getRpcUrls(chain)
		if (!rpcUrl) {
			throw new Error(`No RPC URL configured for ${chain}`)
		}
		return rpcUrl
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

		// Read the endpoint from the config, NOT from `publicClient.transport.url`:
		// a multi-URL chain is built on a viem `fallback` transport, whose `url`
		// is undefined. Passing that through leaves the SDK's EvmChain with no
		// RPC, so viem silently substitutes the chain's built-in public default
		// (e.g. polygon.drpc.org for Polygon) and the operator's endpoints are
		// bypassed entirely on this path.
		const sourceEvmChain = EvmChain.fromParams({
			chainId: getChainId(source)!,
			host: this.configService.getHostAddress(source),
			rpcUrl: this.primaryRpcUrl(source),
		})
		const bundlerUrl = this.configService.getBundlerUrl(destination)
		const destinationEvmChain = EvmChain.fromParams({
			chainId: getChainId(destination)!,
			host: this.configService.getHostAddress(destination),
			rpcUrl: this.primaryRpcUrl(destination),
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

	/**
	 * Best-effort cache warm-up. Deliberately never rejects: this runs unawaited from the
	 * constructor, so a rejection would surface as an unhandled rejection and take the
	 * process down. A failed prewarm only means the value is fetched on first use, where
	 * `getTokenDecimals` is strict — which is the right place for strictness, because
	 * there it can skip a single order instead of killing the filler at boot.
	 */
	async initCache(): Promise<void> {
		try {
			const chainIds = this.configService.getConfiguredChainIds()
			const chainNames = chainIds.map((id) => `EVM-${id}`)
			for (const chainName of chainNames) {
				try {
					await this.getFeeTokenWithDecimals(chainName)
				} catch (err) {
					this.logger.warn({ err, chain: chainName }, "Could not prewarm fee token decimals")
				}
			}

			for (const destChain of chainNames) {
				for (const token of [
					this.configService.getUsdcAsset(destChain),
					this.configService.getUsdtAsset(destChain),
				]) {
					try {
						await this.getTokenDecimals(token, destChain)
					} catch (err) {
						this.logger.warn({ err, chain: destChain, token }, "Could not prewarm token decimals")
					}
				}
			}
		} catch (err) {
			this.logger.warn({ err }, "Token cache prewarm failed")
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
			// The on-chain read is authoritative, but when it fails the SDK's per-chain asset
			// table already carries the curated value for every supported token — so fall back
			// to that rather than guessing. Guessing is what makes this dangerous: `decimals`
			// scales `policyMaxOutput` by `10 ** decimals`, so assuming 18 for a 6-decimal token
			// inflates the computed payout by 10^12, and the payout is no longer clamped to the
			// user's requested output.
			//
			// Deliberately not cached: the registry is a safety net, not the source of truth, so
			// a transient RPC failure must not pin this value for the lifetime of the process.
			// The next call retries the read and caches the real value.
			const configured = this.configService.getAssetDecimalsByAddress(chain, bytes20Address as HexString)
			if (configured !== undefined) {
				this.logger.warn(
					{ err: error, chain, token: bytes20Address, decimals: configured },
					"Error getting token decimals; using configured decimals from the asset registry",
				)
				return configured
			}

			// Hard error rather than a guess. `decimals` scales `policyMaxOutput` by
			// `10 ** decimals`, so a wrong value does not degrade the fill — it changes
			// its size by orders of magnitude. Every caller on the fill path treats a
			// throw as "skip this order" (`IntentFiller.evaluateOrder` is wrapped in a
			// try/catch, as are the phantom-quote and USD-sizing paths), which is the
			// outcome we want when the value is unknowable.
			this.logger.error(
				{ err: error, chain, token: bytes20Address },
				"Could not determine token decimals from RPC or configuration",
			)
			throw new Error(
				`Unable to determine decimals for token ${bytes20Address} on ${chain}: ` +
					`the on-chain decimals() read failed and the token is not in the asset registry`,
			)
		}
	}

	/**
	 * Estimates gas for filling an order and caches the full estimate for bid preparation
	 */
	async estimateGasFillPost(order: Order): Promise<{
		totalCostInSourceFeeToken: bigint
		relayerFeeInSourceFeeToken: bigint
		dispatchFee: bigint
		callGasLimit: bigint
	}> {
		try {
			const client = this.clientManager.getPublicClient(order.destination)
			const cachedEstimate = this.cacheService.getGasEstimate(order.id!)
			if (cachedEstimate) {
				return {
					totalCostInSourceFeeToken: cachedEstimate.totalCostInSourceFeeToken,
					relayerFeeInSourceFeeToken: cachedEstimate.relayerFeeInSourceFeeToken,
					dispatchFee: cachedEstimate.dispatchFee,
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
				args: [this.solverAccountAddress, CryptoUtils.bidNonceKey(orderCommitment(order), order.session)],
			})

			this.logger.info({ orderId: order.id }, "Caching gas estimate")
			this.logger.info({ estimate, fundingGasBump: fundingGasBump.toString() }, "Estimate")
			const callGasLimit = estimate.callGasLimit + fundingGasBump

			this.cacheService.setGasEstimate(
				order.id!,
				estimate.totalGasInFeeToken,
				estimate.relayerFeeInSourceFeeToken,
				estimate.fillOptions.relayerFee,
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
				relayerFeeInSourceFeeToken: estimate.relayerFeeInSourceFeeToken,
				dispatchFee: estimate.fillOptions.relayerFee,
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
	async topUpEntryPointDeposit(
		chain: string,
		targetGasUnits = 3_000_000n,
		thresholdGasUnits?: bigint,
	): Promise<void> {
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
			address: this.configService.getIntentGatewayAddress(chain),
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
	 * Output already delivered against this order by any solver, per output token.
	 *
	 * A partially filled order has had its escrow drawn down, so the pro-rata
	 * release a later filler receives is computed against the *residual*, not
	 * against `order.inputs[i].amount`. A strategy that prices off the original
	 * inputs would overstate its take, which is why the partial-fill path refuses
	 * any order this reports as already touched.
	 *
	 * Same-chain concept only — the cross-chain path has no partial-fill state.
	 */
	async partialFillsFor(order: Order, chain: string): Promise<bigint[]> {
		const client = this.clientManager.getPublicClient(chain)
		const address = this.configService.getIntentGatewayAddress(chain)
		const commitment = orderCommitment(order)

		return Promise.all(
			order.output.assets.map((asset) =>
				client.readContract({
					address,
					abi: INTENT_GATEWAY_V2_ABI,
					functionName: "_partialFills",
					args: [commitment, asset.token],
				}) as Promise<bigint>,
			),
		)
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
			// The dispatch is always paid in the fee token: the native rail drew
			// on the solver account's native balance, which nothing guarantees,
			// and a shortfall only surfaced as a reverted execution that still
			// billed the paymaster (estimation overrides the balance, so it
			// could never catch it).
			nativeDispatchFee: 0n,
			// Caps how long this quote stands. Without it the placer holds a free option:
			// they choose the moment of execution and we are committed to the old price.
			validUntil: await this.bidValidUntilBlock(order.destination),
			outputs: cachedFillerOutputs,
		}

		// dispatchWithFeeToken pulls relayerFee in fee token from the solver.
		const dispatchFeeTokenAmount = fillOptions.relayerFee
		const callData = await this.buildApprovalAndFillCalldata(
			order,
			cachedFillerOutputs,
			fillOptions,
			cachedEstimate.totalCostInSourceFeeToken + dispatchFeeTokenAmount,
		)

		const commitment = orderCommitment(order)

		// Build paymasterAndData — Circle (USDC permit) → Simplex → EntryPoint deposit
		const pmResult = await buildPaymasterAndData({
			chain: order.destination,
			solverAccount: solverAccountAddress,
			publicClient: this.clientManager.getPublicClient(order.destination),
			walletClient: this.clientManager.getWalletClient(order.destination),
			signer: this.signer,
			configService: this.configService,
		})
		const paymasterAndData = pmResult.paymasterAndData
		if (pmResult.type !== "none") {
			this.logger.info({ paymaster: pmResult.address, type: pmResult.type }, "Using paymaster for bid UserOp")
		}

		const userOp = await sdkHelper.prepareSubmitBid({
			order,
			fillOptions,
			solverAccount: solverAccountAddress,
			solverSigner: sdkSigningAccount(this.signer),
			nonce: cachedEstimate.nonce,
			entryPointAddress,
			callGasLimit: cachedEstimate.callGasLimit,
			verificationGasLimit: cachedEstimate.verificationGasLimit,
			preVerificationGas: cachedEstimate.preVerificationGas,
			maxFeePerGas: cachedEstimate.maxFeePerGas,
			maxPriorityFeePerGas: cachedEstimate.maxPriorityFeePerGas,
			callData,
			paymasterAndData,
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
	 * Last block on the destination chain at which a bid signed now may still execute.
	 *
	 * A bid is a firm quote the order placer takes up whenever they like, and nothing else
	 * bounds that window — `order.deadline` is placer-chosen with no ceiling, and retracting
	 * on Hyperbridge leaves the destination-chain calldata untouched. An unbounded bid is a
	 * free option on this filler's inventory, exercised only once the rate has moved against
	 * us. This caps its tenor.
	 *
	 * Operators configure seconds because that is the unit the risk is actually in, but the
	 * contract compares against block numbers so `order.deadline` and this read the same
	 * clock. The conversion uses the chain's nominal block time; it is deliberately rounded
	 * up, since erring long costs a slightly stale quote while erring short silently drops
	 * winnable bids.
	 */
	private async bidValidUntilBlock(chain: string): Promise<bigint> {
		const client = this.clientManager.getPublicClient(chain)
		const currentBlock = await client.getBlockNumber()
		const blockTimeMs = client.chain?.blockTime
		const blockTimeSec = blockTimeMs ? blockTimeMs / 1000 : 2
		const blocks = BigInt(Math.ceil(this.configService.getBidValiditySeconds() / blockTimeSec))
		return currentBlock + blocks
	}

	/**
	 * Builds a PackedUserOperation for a phantom (expired same-chain) order bid.
	 * Uses zero relayer fees and default gas values — no estimation needed since
	 * the order will never execute; the indexer only reads the proposed fill amounts.
	 *
	 * When the filler declares accepted source chains, the declaration rides in
	 * paymasterAndData so the userOpHash — and therefore the solver's bid signature —
	 * covers it. Phantom bids never reach a bundler or the EntryPoint, so the field is
	 * free for this; a real fill's paymasterAndData keeps its functional semantics.
	 */
	async preparePhantomBidUserOp(
		order: Order,
		entryPointAddress: HexString,
		solverAccountAddress: HexString,
		fillerOutputs: TokenInfo[],
		acceptedSourceChains?: string[],
		uniswapV4PositionIds?: string[],
	): Promise<{ commitment: HexString; userOp: HexString }> {
		const sdkHelper = await this.getIntentGateway(order.source, order.destination)
		const client = this.clientManager.getPublicClient(order.destination)

		// A phantom order is already expired, so this bid can never execute regardless — the
		// bound is set anyway so every signed artefact carries one.
		const fillOptions: FillOptions = {
			relayerFee: 0n,
			nativeDispatchFee: 0n,
			validUntil: await this.bidValidUntilBlock(order.destination),
			outputs: fillerOutputs,
		}
		const callData = await this.buildApprovalAndFillCalldata(order, fillerOutputs, fillOptions, 0n)

		const commitment = orderCommitment(order)

		let nonce = 0n
		try {
			nonce = (await client.readContract({
				address: entryPointAddress,
				abi: ENTRYPOINT_ABI,
				functionName: "getNonce",
				args: [solverAccountAddress, CryptoUtils.bidNonceKey(commitment, order.session)],
			})) as bigint
		} catch {
			// Nonce defaults to 0 for phantom bids — the bid is never executed on-chain
		}

		const gasPrice = await client.getGasPrice().catch(() => 1_000_000_000n)

		const userOp = await sdkHelper.prepareSubmitBid({
			order,
			fillOptions,
			solverAccount: solverAccountAddress,
			solverSigner: sdkSigningAccount(this.signer),
			nonce,
			entryPointAddress,
			callGasLimit: 500_000n,
			verificationGasLimit: 150_000n,
			preVerificationGas: 50_000n,
			maxFeePerGas: gasPrice,
			maxPriorityFeePerGas: gasPrice / 10n,
			callData,
			paymasterAndData:
				acceptedSourceChains || uniswapV4PositionIds?.length
					? encodePhantomBidDeclaration({
							acceptedSourceChains,
							uniswapV4Positions: uniswapV4PositionIds?.map((id) => BigInt(id)),
						})
					: ("0x" as HexString),
		})

		return { commitment, userOp: encodeUserOpScale(userOp) }
	}

	/**
	 * Builds ERC-7821 batch calldata that prepends any required ERC20 approvals
	 * before the fillOrder call, all within a single UserOp payload.
	 *
	 * Same-chain fills release escrow locally with no Hyperbridge dispatch, so the
	 * gateway never pulls the fee token — its approval is skipped. Only cross-chain
	 * fills, which dispatch a RedeemEscrow message paid in the fee token, need it.
	 */
	public async buildApprovalAndFillCalldata(
		order: Order,
		fillerOutputs: TokenInfo[],
		fillOptions: FillOptions,
		requiredFeeTokenAmount: bigint,
	): Promise<HexString> {
		const chain = order.destination
		const destClient = this.clientManager.getPublicClient(chain)
		const intentGatewayV2Address = this.configService.getIntentGatewayAddress(chain)

		// Aggregate required amounts per ERC20 token
		const perTokenRequired = new Map<string, bigint>()
		for (const output of fillerOutputs) {
			const addr = bytes32ToBytes20(output.token)
			if (addr === ADDRESS_ZERO) continue
			const key = addr.toLowerCase()
			perTokenRequired.set(key, (perTokenRequired.get(key) ?? 0n) + output.amount)
		}

		if (order.source !== order.destination) {
			const feeToken = await this.getFeeTokenWithDecimals(chain)
			const feeKey = feeToken.address.toLowerCase()
			perTokenRequired.set(feeKey, (perTokenRequired.get(feeKey) ?? 0n) + requiredFeeTokenAmount)
		}

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
						args: [intentGatewayV2Address, required],
					}) as HexString,
				})
			}
		}

		// Append fillOrder call (after any approvals, or as the sole call)
		const nativeOutputValue = fillerOutputs
			.filter((asset) => bytes32ToBytes20(asset.token) === ADDRESS_ZERO)
			.reduce((sum, asset) => sum + asset.amount, 0n)

		// Gateways predating `FillOptions.validUntil` take a differently-shaped (and
		// differently-selectored) fillOrder, so the encoding has to match the deployment.
		const fillOptionsVersion = await getFillOptionsVersion(destClient as any, intentGatewayV2Address)
		if (fillOptionsVersion === 1 && fillOptions.validUntil !== 0n && !this.warnedNoValidUntil.has(chain)) {
			this.warnedNoValidUntil.add(chain)
			this.logger.warn(
				{ chain, gateway: intentGatewayV2Address },
				"IntentGateway predates FillOptions.validUntil — bids on this chain carry no expiry and stay " +
					"executable until the order's own deadline. Upgrade the gateway to bound them.",
			)
		}

		calls.push({
			target: intentGatewayV2Address,
			value: nativeOutputValue,
			data: encodeFillOrder(transformOrderForContract(order) as any, fillOptions, fillOptionsVersion),
		})

		return encodeERC7821ExecuteBatch(calls)
	}
}
