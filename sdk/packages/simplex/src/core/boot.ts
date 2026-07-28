import { Decimal } from "decimal.js"
import { IntentFiller } from "@/core/filler"
import { loadRuntimeState } from "@/core/runtime-state"
import { FXFiller, type TradingPair } from "@/strategies/fx"
import type { VaultConfig, FundingVenue, UniswapV4PositionConfig } from "@/funding/types"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
import { VaultLiquidityState } from "@/funding/vault/VaultLiquidityState"
import { TokenSender } from "@/services/TokenSender"
import { FillerPricePolicy, parseChainKey } from "@/config/interpolated-curve"
import { AssetRegistry, normalizeSymbol } from "@/config/asset-registry"
import { assertPairSymbolsResolve } from "@/config/pairs"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	type ResolvedChainConfig,
	FillerConfig as FillerServiceConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import { assertConfirmationCoverage, validateConfig, type FillerTomlConfig, type VaultToml } from "@/config/filler-toml"
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
import { ActivityLogService } from "@/services/ActivityLogService"
import type { AdminStrategy, HaltControl } from "@/services/server/UiServer"
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
	/** Live FillerPricePolicy handles shared with the trading engine, one per curve-priced pair. */
	adminStrategies: AdminStrategy[]
	/** Self-halt visibility/reset for the trading engine (overfill protection). */
	haltControls: HaltControl[]
	activityLog: ActivityLogService
	bidStorage: BidStorageService
	configService: FillerConfigService
	/** Symbol-to-address resolution for the configured chains (send options, balance labels). */
	assetRegistry: AssetRegistry
	rebalancingService?: RebalancingService
	resolvedChains: ResolvedChainConfig[]
	fillerAddress: HexString
	watchOnly?: Record<number, boolean>
	config: FillerTomlConfig
	configPath: string
	/**
	 * Hydrates throwaway state for a prospective vault set (read-only on-chain
	 * calls) so hydration-time errors — unconfigured chain, same-asset
	 * duplicates, non-vault address — surface before the set is persisted.
	 */
	vaultPreflight(vaults: VaultToml[]): Promise<void>
	/** Operator-initiated outbound transfers (dashboard Send). */
	tokenSender: TokenSender
	dataDir?: string
	startedAt: number
	/** Stops everything bootFiller started. Idempotent; does NOT process.exit. */
	shutdown(signal: string): Promise<void>
}

/**
 * Boots the filler from a validated config: resolves chains, wires services and
 * the pair-trading engine, starts the IntentFiller and background timers. Used
 * by `simplex run` and by the setup wizard's save-and-start transition.
 */
export async function bootFiller(config: FillerTomlConfig, options: BootOptions): Promise<FillerRuntime> {
	validateConfig(config, options.watchOnlyOverride === true)

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
				const chainId = parseChainKey(chainIdStr)
				if (chainId === null) {
					// Never discard silently: a dropped key here means the
					// filler commits capital on a chain meant to be observed.
					throw new Error(`simplex.watchOnly key '${chainIdStr}' is not a chain id — use "EVM-<id>" or "<id>"`)
				}
				watchOnlyConfig![chainId] = value === true
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
	// A single instance is shared with the sweep timer.
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

	// Build the trading engine from top-level [[pairs]].
	logger.info("Initializing trading engine...")

	// Asset symbol registry: built-ins (USDC/USDT/DAI/CNGN) resolved from the
	// SDK chain registry, extended/overridden by the user's [assets] table.
	const assetRegistry = new AssetRegistry(configService, config.assets)

	// Symbols must resolve to real, distinct deployments on the configured
	// chains — the same checks the wizard gates run, re-run here against the
	// chains actually resolved from the RPCs.
	const configuredChainNames = resolvedChains.map((chain) => `EVM-${chain.chainId}`)
	if (config.pairs?.length) {
		assertPairSymbolsResolve(config.pairs, assetRegistry, configuredChainNames)
	}

	// Editable price curves for the UI server, collected at construction so the
	// server mutates the exact policy instances the engine prices with.
	const adminStrategies: AdminStrategy[] = []
	const strategies: FXFiller[] = []
	if (config.pairs?.length) {
		const tradingPairs: TradingPair[] = config.pairs.map((pair) => ({
			token0: pair.token0,
			token1: pair.token1,
			// Reference-only pairs never fill, so the cap is never consulted.
			maxOrderSize: new Decimal(pair.maxOrderSize ?? "0"),
			referenceOnly: pair.referenceOnly === true,
			bidPricePolicy: pair.bidPriceCurve?.length ? new FillerPricePolicy({ points: pair.bidPriceCurve }) : undefined,
			askPricePolicy: pair.askPriceCurve?.length ? new FillerPricePolicy({ points: pair.askPriceCurve }) : undefined,
		}))
		tradingPairs.forEach((pair, pairIndex) => {
			if (!pair.bidPricePolicy && !pair.askPricePolicy) return
			const sameToken = normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1)
			const adminStrategy: AdminStrategy = {
				index: adminStrategies.length,
				pairIndex,
				exotic: `${pair.token0}/${pair.token1}${pair.referenceOnly ? " (reference)" : ""}`,
				token0: pair.token0,
				token1: pair.token1,
				bid: pair.bidPricePolicy,
				ask: pair.askPricePolicy,
				sameToken,
				referenceOnly: pair.referenceOnly === true,
			}
			// Reference pairs never fill, so opening a side is a no-op; same-token
			// markets are ask-only by engine rule.
			if (!sameToken && !pair.referenceOnly) {
				// The engine reads curve presence live on every order, so
				// assigning a policy onto the pair opens that direction.
				adminStrategy.enableSide = (side, policy) => {
					if (side === "bid") {
						pair.bidPricePolicy = policy
						adminStrategy.bid = policy
					} else {
						pair.askPricePolicy = policy
						adminStrategy.ask = policy
					}
					logger.warn(
						{ pair: `${pair.token0}/${pair.token1}`, side },
						"Trading direction enabled by operator with a new price curve",
					)
				}
				// Clearing the policy closes the direction on the next order —
				// the operator's path back to one-sided LP.
				adminStrategy.disableSide = (side) => {
					if (side === "bid") {
						pair.bidPricePolicy = undefined
						adminStrategy.bid = undefined
					} else {
						pair.askPricePolicy = undefined
						adminStrategy.ask = undefined
					}
					logger.warn(
						{ pair: `${pair.token0}/${pair.token1}`, side },
						"Trading direction disabled by operator (one-sided LP)",
					)
				}
			}
			adminStrategies.push(adminStrategy)
		})

		// Orders can be sourced on any configured chain (watch-only ones
		// included), so each needs a confirmation curve — fail at boot,
		// not with silently dropped orders at fill time. Same construction the
		// wizard write gates run against the selected chain ids.
		const confirmationPolicy = assertConfirmationCoverage(
			config.confirmationPolicies,
			resolvedChains.map((c) => c.chainId),
		)

		const fundingVenues: FundingVenue[] = []
		// Vault first: source stablecoins from the idle-yield treasury before
		// draining a V4 LP position (which also pulls the paired exotic and
		// perturbs the pool used for exotic pricing). V4 then covers the
		// exotic legs and any stablecoin the vault can't fully fund.
		if (vaultVenue) {
			fundingVenues.push(vaultVenue)
		}
		const priceGuard: Record<string, { referencePrice: string; maxDeviationBps: number }> = {}
		if (config.vault?.uniswapV4?.positions?.length) {
			const positionsByChain: Record<string, UniswapV4PositionConfig[]> = {}
			for (const row of config.vault.uniswapV4.positions) {
				const chain = row.chain
				if (!positionsByChain[chain]) positionsByChain[chain] = []
				positionsByChain[chain].push({ tokenId: BigInt(row.tokenId) })
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
					config.vault.uniswapV4.spreadBps,
				),
			)
		}

		const engine = new FXFiller(
			runtimeSigner,
			configService,
			chainClientManager,
			contractService,
			tradingPairs,
			assetRegistry,
			{
				confirmationPolicy,
				fundingVenues,
				priceGuard,
				side: config.vault?.uniswapV4?.side,
			},
		)
		logger.info("Hydrating funding venue state...")
		await engine.initialise()
		strategies.push(engine)
	}

	const haltControls: HaltControl[] = strategies.map((engine, index) => ({
		index,
		isHalted: () => engine.isHalted(),
		resetHalt: () => engine.resetHalt(),
	}))

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

	// Persistent order-activity feed for the operator UI
	const activityLog = new ActivityLogService(options.dataDir)
	activityLog.attach(intentFiller.monitor)
	if (vaultVenue) {
		vaultVenue.onTx = ({ chain, kind, txHash, sponsored }) => {
			try {
				activityLog.recordWalletTx({
					kind,
					chainId: parseChainKey(chain),
					token: null,
					amount: null,
					to: null,
					txHash,
					sponsored,
				})
			} catch (err) {
				logger.warn({ err }, "Failed to record vault tx in wallet history")
			}
		}
	}

	// Collect exotic token addresses (the non-quote side of cross-asset pairs)
	// via the asset registry; same-token pairs have no exotic side. Every
	// pair's token1 is tracked — keying one address per chain would silently
	// drop all but the last pair's balances.
	const token1: Record<string, string[]> = {}
	for (const pair of config.pairs ?? []) {
		if (normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1)) continue
		if (pair.referenceOnly) continue // price feed only — never holds fill inventory
		for (const chainName of configuredChainNames) {
			const address = assetRegistry.getAddress(pair.token1, chainName)
			if (!address) continue
			const list = (token1[chainName] ??= [])
			if (!list.includes(address)) list.push(address)
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
			pairs: (config.pairs ?? []).map((p) => `${p.token0}/${p.token1}`),
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
		activityLog.close()
	}

	return {
		intentFiller,
		balanceProvider,
		metrics,
		vaultVenue,
		adminStrategies,
		haltControls,
		activityLog,
		bidStorage: bidStorageService,
		configService,
		assetRegistry,
		rebalancingService,
		resolvedChains,
		fillerAddress: runtimeSigner.account.address as HexString,
		watchOnly: watchOnlyConfig,
		config,
		configPath: options.configPath,
		dataDir: options.dataDir,
		startedAt: Date.now(),
		tokenSender: new TokenSender(
			chainClientManager,
			runtimeSigner.account.address as HexString,
			() => config.vault?.vaults ?? [],
			userOpSender,
		),
		vaultPreflight: async (vaults) => {
			const byChain: Record<string, VaultConfig[]> = {}
			for (const row of vaults) {
				if (!byChain[row.chain]) byChain[row.chain] = []
				byChain[row.chain].push({
					vault: row.vault,
					threshold: row.threshold,
					minBalance: row.minBalance,
					redeemOnShutdown: row.redeemOnShutdown,
				})
			}
			for (const [chain, chainVaults] of Object.entries(byChain)) {
				await new VaultLiquidityState(
					chain,
					chainVaults,
					runtimeSigner.account.address as HexString,
					chainClientManager,
				).hydrate()
			}
		},
		shutdown,
	}
}
