import { IntentFiller } from "@/core/filler"
import { loadRuntimeState } from "@/core/runtime-state"
import { StableFiller } from "@/strategies/stable"
import { FXFiller } from "@/strategies/fx"
import type { VaultConfig, FundingVenue, UniswapV4PositionConfig } from "@/funding/types"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
import { ConfirmationPolicy, FillerBpsPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	type ResolvedChainConfig,
	FillerConfig as FillerServiceConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import {
	validateConfig,
	DEFAULT_CONFIRMATION_POLICIES,
	type FillerTomlConfig,
	type StrategyConfig,
} from "@/config/filler-toml"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { UserOpSender } from "@/services/UserOpSender"
import { RebalancingService } from "@/services/RebalancingService"
import { getLogger, configureLogger, type LogLevel } from "@/services/Logger"
import { CacheService } from "@/services/CacheService"
import { BidStorageService } from "@/services/BidStorageService"
import { initializeSignerFromToml } from "@/services/wallet"
import { MetricsService } from "@/services/MetricsService"
import { BalanceProvider } from "@/services/BalanceProvider"
import type { AdminStrategy } from "@/services/server/UiServer"
import { ERC20_ABI } from "@/config/abis/ERC20"
import type { BinanceCexConfig } from "@/services/rebalancers/index"
import type { SigningAccount } from "@/services/wallet"

export interface BootOptions {
	configPath: string
	dataDir?: string
	/** --watch-only CLI flag: forces watch-only on every chain. */
	watchOnlyOverride?: boolean
	/** Prometheus bind, when -p was passed with a valid value. */
	metricsBind?: { host: string; port: number }
}

/** Everything a running filler exposes to the UI server and the CLI. */
export interface FillerRuntime {
	intentFiller: IntentFiller
	balanceProvider: BalanceProvider
	metrics?: MetricsService
	vaultVenue?: VaultFundingPlanner
	/** Live FillerPricePolicy handles shared with the FX strategies, exotic labels pre-resolved. */
	adminStrategies: AdminStrategy[]
	resolvedChains: ResolvedChainConfig[]
	fillerAddress: HexString
	watchOnly?: Record<number, boolean>
	config: FillerTomlConfig
	configPath: string
	dataDir?: string
	startedAt: number
	/** Stops everything bootFiller started. Idempotent; does NOT process.exit. */
	shutdown(signal: string): Promise<void>
}

/**
 * Boots the filler from a validated config: resolves chains, wires services and
 * strategies, starts the IntentFiller and background timers. Used by `simplex run`
 * and by the setup wizard's save-and-start transition.
 */
export async function bootFiller(config: FillerTomlConfig, options: BootOptions): Promise<FillerRuntime> {
	validateConfig(config)

	// Configure logger based on config BEFORE creating any services
	if (config.simplex.logging) {
		configureLogger(config.simplex.logging as LogLevel)
	}

	const logger = getLogger("cli")
	logger.info({ configPath: options.configPath }, "Loading configuration")
	logger.info("Starting Filler...")

	logger.info("Initializing services...")

	logger.info("Resolving chain IDs from RPC endpoints...")
	const resolvedChains: ResolvedChainConfig[] = await resolveChainConfigs(config.chains)
	logger.info({ chains: resolvedChains.map((c) => c.chainId) }, "Chain IDs resolved")

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: config.simplex.maxConcurrentOrders,
		logging: config.simplex.logging as LogLevel | undefined,
		substratePrivateKey: config.simplex.substratePrivateKey,
		hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
		entryPointAddress: config.simplex.entryPointAddress,
		dataDir: options.dataDir,
		rebalancing: config.rebalancing,
		targetGasUnits: config.simplex.targetGasUnits,
		gasFeeBump: config.simplex.gasFeeBump,
		overfillProtection: config.simplex.overfillProtection,
		allowlist: config.allowlist,
	}

	const configService = new FillerConfigService(resolvedChains, fillerConfigForService)

	const chainConfigs: ChainConfig[] = resolvedChains.map((chain) => {
		const chainName = `EVM-${chain.chainId}`
		return configService.getChainConfig(chainName)
	})

	// Create filler configuration
	// Handle watchOnly: can be boolean (global) or Record<string, boolean> (per-chain)
	let watchOnlyConfig: Record<number, boolean> | undefined
	if (options.watchOnlyOverride) {
		// CLI flag overrides config - apply to all chains
		watchOnlyConfig = {}
		resolvedChains.forEach((chain) => {
			watchOnlyConfig![chain.chainId] = true
		})
	} else if (config.simplex.watchOnly !== undefined) {
		if (typeof config.simplex.watchOnly === "boolean") {
			// Global watch-only mode
			watchOnlyConfig = {}
			resolvedChains.forEach((chain) => {
				watchOnlyConfig![chain.chainId] = config.simplex.watchOnly as boolean
			})
		} else {
			// Per-chain configuration
			watchOnlyConfig = {}
			Object.entries(config.simplex.watchOnly).forEach(([chainIdStr, value]) => {
				const chainId = Number.parseInt(chainIdStr, 10)
				if (!Number.isNaN(chainId)) {
					watchOnlyConfig![chainId] = value === true
				}
			})
		}
	}

	const fillerConfig: FillerConfig = {
		maxConcurrentOrders: config.simplex.maxConcurrentOrders,
		pendingQueueConfig: config.simplex.queue,
		watchOnly: watchOnlyConfig,
	} as FillerConfig

	// Create shared services to avoid duplicate RPC calls and reuse connections
	const sharedCacheService = new CacheService()
	const configuredSigner = await initializeSignerFromToml(config.simplex.signer)
	const chainClientManager = new ChainClientManager(configService, configuredSigner)
	const runtimeSigner: SigningAccount = chainClientManager.getSigner()

	const contractService = new ContractInteractionService(
		chainClientManager,
		configService,
		runtimeSigner,
		sharedCacheService,
	)

	// Initialize bid storage service for persistent storage of bid transaction hashes
	// This enables later cleanup and fund recovery from Hyperbridge
	const bidStorageService = new BidStorageService(configService.getDataDir())
	logger.info(
		{ dataDir: configService.getDataDir() || ".filler-data" },
		"Bid storage initialized for fund recovery tracking",
	)

	// Sponsors self-initiated UserOps (delegation, vault sweep/redeem) via the
	// Circle paymaster so gas is paid in USDC instead of native token.
	const userOpSender = new UserOpSender(chainClientManager, configService, runtimeSigner)

	// Build the shared vault venue (withdraw sourcing + threshold sweeping).
	// A single instance is shared across strategies and the sweep timer.
	let vaultVenue: VaultFundingPlanner | undefined
	if (config.vault?.vaults?.length) {
		const vaultsByChain: Record<string, VaultConfig[]> = {}
		for (const row of config.vault.vaults) {
			if (!vaultsByChain[row.chain]) vaultsByChain[row.chain] = []
			vaultsByChain[row.chain].push({
				vault: row.vault,
				threshold: row.threshold,
				minBalance: row.minBalance,
				redeemOnShutdown: row.redeemOnShutdown,
			})
		}
		vaultVenue = new VaultFundingPlanner(
			chainClientManager,
			{
				vaultsByChain,
				sweepIntervalMs: config.vault.sweepIntervalMs,
			},
			userOpSender,
		)
	}

	// Initialize strategies with shared services
	logger.info("Initializing strategies...")
	// Editable price curves for the UI server, collected at construction so the
	// server mutates the exact policy instances the strategies price with.
	const adminStrategies: AdminStrategy[] = []
	const adminTokenMaps = new Map<number, Record<string, HexString>>()
	const strategies = config.strategies.map((strategyConfig, strategyIndex) => {
		switch (strategyConfig.type) {
			case "stable": {
				const bpsPolicy = new FillerBpsPolicy({ points: strategyConfig.bpsCurve })
				const mergedStablePolicies = {
					...DEFAULT_CONFIRMATION_POLICIES,
					...(strategyConfig.confirmationPolicies ?? {}),
				}
				const confirmationPolicy = new ConfirmationPolicy(mergedStablePolicies)
				return new StableFiller(
					runtimeSigner,
					configService,
					chainClientManager,
					contractService,
					bpsPolicy,
					confirmationPolicy,
					vaultVenue ? [vaultVenue] : [],
				)
			}
			case "hyperfx": {
				const bidPricePolicy = strategyConfig.bidPriceCurve?.length
					? new FillerPricePolicy({ points: strategyConfig.bidPriceCurve })
					: undefined
				const askPricePolicy = strategyConfig.askPriceCurve?.length
					? new FillerPricePolicy({ points: strategyConfig.askPriceCurve })
					: undefined
				adminStrategies.push({
					index: strategyIndex,
					bid: bidPricePolicy,
					ask: askPricePolicy,
				})
				adminTokenMaps.set(strategyIndex, strategyConfig.token1)
				const mergedFxPolicies = {
					...DEFAULT_CONFIRMATION_POLICIES,
					...(strategyConfig.confirmationPolicies ?? {}),
				}
				const fxConfirmationPolicy = new ConfirmationPolicy(mergedFxPolicies)
				const fundingVenues: FundingVenue[] = []
				// Vault first: source stablecoins from the idle-yield treasury before
				// draining a V4 LP position (which also pulls the paired exotic and
				// perturbs the pool used for exotic pricing). V4 then covers the
				// exotic legs and any stablecoin the vault can't fully fund.
				if (vaultVenue) {
					fundingVenues.push(vaultVenue)
				}
				const priceGuard: Record<string, { referencePrice: string; maxDeviationBps: number }> = {}
				if (strategyConfig.vault?.uniswapV4?.positions?.length) {
					const positionsByChain: Record<string, UniswapV4PositionConfig[]> = {}
					for (const row of strategyConfig.vault.uniswapV4.positions) {
						const chain = row.chain
						if (!positionsByChain[chain]) positionsByChain[chain] = []
						positionsByChain[chain].push({
							tokenId: BigInt(row.tokenId),
						})
						if (row.referencePrice !== undefined) {
							priceGuard[chain] = {
								referencePrice: row.referencePrice,
								maxDeviationBps: row.maxDeviationBps!,
							}
						}
					}
					fundingVenues.push(
						new UniswapV4FundingPlanner(
							chainClientManager,
							{ positionsByChain },
							configService,
							strategyConfig.spreadBps,
						),
					)
				}
				return new FXFiller(
					runtimeSigner,
					configService,
					chainClientManager,
					contractService,
					strategyConfig.maxOrderUsd,
					strategyConfig.token1,
					{
						bidPricePolicy,
						askPricePolicy,
						confirmationPolicy: fxConfirmationPolicy,
						fundingVenues,
						spreadBps: strategyConfig.spreadBps,
						priceGuard,
						side: strategyConfig.vault?.uniswapV4?.side,
					},
				)
			}
			default:
				throw new Error(`Unknown strategy type: ${(strategyConfig as StrategyConfig).type}`)
		}
	})

	// Initialise strategies that source on-chain liquidity (hydrate funding venue state)
	for (const strategy of strategies) {
		if (strategy instanceof FXFiller || strategy instanceof StableFiller) {
			logger.info("Hydrating funding venue state...")
			await strategy.initialise()
		}
	}

	// Ensure the shared vault venue is hydrated even if no strategy
	// initialised it, so the sweep timer has live state. Idempotent.
	if (vaultVenue) {
		await vaultVenue.initialise(runtimeSigner.account.address as HexString)
	}

	// Initialize rebalancing service only if fully configured
	let rebalancingService: RebalancingService | undefined
	const rebalancingConfig = configService.getRebalancingConfig()
	if (rebalancingConfig?.triggerPercentage !== undefined && rebalancingConfig?.baseBalances) {
		let binanceConfig: BinanceCexConfig | undefined
		if (config.binance) {
			binanceConfig = {
				apiKey: config.binance.apiKey,
				apiSecret: config.binance.apiSecret,
				basePath: config.binance.basePath,
				timeout: config.binance.timeout,
				pollIntervalMs: config.binance.pollIntervalMs,
			}
			logger.info("Binance CEX rebalancing configured")
		}

		rebalancingService = new RebalancingService(chainClientManager, configService, binanceConfig)
		logger.info("Rebalancing service initialized")
	}

	// Initialize and start the intent filler
	logger.info("Starting intent filler...")
	const intentFiller = new IntentFiller(
		chainConfigs,
		strategies,
		fillerConfig,
		configService,
		chainClientManager,
		contractService,
		runtimeSigner,
		rebalancingService,
		bidStorageService,
	)

	// Initialize (sets up EIP-7702 delegation if solver selection is configured)
	await intentFiller.initialize()

	// Resolve exotic token symbols (display-only) best-effort from the first
	// configured chain; a failed read just leaves the strategy unlabelled.
	await Promise.all(
		adminStrategies.map(async (adminStrategy) => {
			const token1Map = adminTokenMaps.get(adminStrategy.index) ?? {}
			const [chain, address] = Object.entries(token1Map)[0] ?? []
			if (!chain || !address) return
			try {
				adminStrategy.exotic = (await chainClientManager.getPublicClient(chain).readContract({
					address,
					abi: ERC20_ABI,
					functionName: "symbol",
					args: [],
				})) as string
			} catch (err) {
				logger.warn({ err, chain, address }, "Could not resolve exotic token symbol for the UI")
			}
		}),
	)

	// Shared balance snapshots for Prometheus gauges and the UI API
	const token1: Record<string, string> = {}
	for (const s of config.strategies) {
		if (s.type === "hyperfx" && s.token1) {
			Object.assign(token1, s.token1)
		}
	}
	const balanceProvider = new BalanceProvider({
		chainClientManager,
		configService,
		fillerAddress: runtimeSigner.account.address,
		token1,
		hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
		substratePrivateKey: config.simplex.substratePrivateKey,
	})

	// Start optional Prometheus metrics server
	let metrics: MetricsService | undefined
	if (options.metricsBind) {
		metrics = new MetricsService({
			monitor: intentFiller.monitor,
			bidStorage: bidStorageService,
			balances: balanceProvider,
			dataDir: options.dataDir,
		})
		metrics.start(options.metricsBind.port, options.metricsBind.host)
	}

	// Start the filler
	intentFiller.start()

	// An operator-initiated pause survives restarts
	if (loadRuntimeState(options.dataDir).paused) {
		intentFiller.pause()
	}

	// Start the vault threshold-sweep timer (lifecycle owned here, not by the filler)
	vaultVenue?.startSweeping()

	balanceProvider.start()

	const watchOnlyChains = watchOnlyConfig
		? Object.entries(watchOnlyConfig)
				.filter(([, value]) => value === true)
				.map(([chainId]) => Number.parseInt(chainId, 10))
		: []

	logger.info(
		{
			chains: resolvedChains.map((c) => c.chainId),
			strategies: config.strategies.map((s) => s.type),
			maxConcurrentOrders: config.simplex.maxConcurrentOrders,
			watchOnlyChains: watchOnlyChains.length > 0 ? watchOnlyChains : undefined,
		},
		watchOnlyChains.length > 0
			? `Intent filler is running (watch-only on chains: ${watchOnlyChains.join(", ")})`
			: "Intent filler is running",
	)

	let stopping = false
	const shutdown = async (signal: string): Promise<void> => {
		if (stopping) return
		stopping = true
		logger.warn(`Shutting down intent filler (${signal})...`)
		metrics?.stop()
		balanceProvider.stop()
		vaultVenue?.stopSweeping()
		await intentFiller.stop()
		// Exit all vault positions back to the underlying asset (best-effort).
		await vaultVenue?.redeemAll()
	}

	return {
		intentFiller,
		balanceProvider,
		metrics,
		vaultVenue,
		adminStrategies,
		resolvedChains,
		fillerAddress: runtimeSigner.account.address as HexString,
		watchOnly: watchOnlyConfig,
		config,
		configPath: options.configPath,
		dataDir: options.dataDir,
		startedAt: Date.now(),
		shutdown,
	}
}
