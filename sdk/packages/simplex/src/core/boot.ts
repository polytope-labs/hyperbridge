import { Decimal } from "decimal.js"
import { IntentFiller } from "@/core/filler"
import { FXFiller, type TradingPair } from "@/strategies/fx"
import type { VaultConfig, FundingVenue, UniswapV4PositionConfig } from "@/funding/types"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
import { VaultLiquidityState } from "@/funding/vault/VaultLiquidityState"
import { TokenSender } from "@/services/TokenSender"
import { FillerPricePolicy, parseChainKey } from "@/config/interpolated-curve"
import { AssetRegistry, normalizeSymbol } from "@/config/asset-registry"
import { assertPairSymbolsResolve, type PairConfig } from "@/config/pairs"
import type { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	type ResolvedChainConfig,
	type FillerConfig as FillerServiceConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import { assertConfirmationCoverage, validateConfig, type FillerTomlConfig, type VaultToml } from "@/config/filler-toml"
import type { ConfirmationPolicy } from "@/config/interpolated-curve"
import { DEFAULT_MAX_CONCURRENT_ORDERS } from "@/config/defaults"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { UserOpSender } from "@/services/UserOpSender"
import { RebalancingService } from "@/services/RebalancingService"
import { getLogger, moduleLogger, type Logger, type LogLevel, type LoggerContext } from "@/services/Logger"
import { CacheService } from "@/services/CacheService"
import { BalanceProvider } from "@/services/BalanceProvider"
import { ActivityRecorder } from "@/data/recorder"
import type { SimplexDataStore } from "@/data/types"
import type { HyperbridgeScanner, OrderScanner } from "@/scanner/types"
import type { AdminStrategy, HaltControl } from "@/services/server/UiServer"
import type { BinanceCexConfig } from "@/services/rebalancers/index"
import type { Signer } from "@/services/wallet"

export interface BootOptions {
	/** Where the config was loaded from, when it came from a file. Purely informational. */
	configPath?: string
	/** Persistence backend. Required — `Simplex.start` defaults it to a MemoryDataStore. */
	data: SimplexDataStore
	/**
	 * Whether shutdown may close `data`. False for a caller-supplied store: two
	 * solvers can share one, and the first to stop must not pull it out from
	 * under the second — the same ownership rule the scanners follow.
	 */
	ownsData?: boolean
	/**
	 * This filler's logging destination. Every service resolves its logger from
	 * here, so two fillers in one process never write into each other's sinks.
	 */
	loggers: LoggerContext
	/**
	 * Event scanners this filler reads from. `Simplex.start` supplies the ones the
	 * caller passed, or builds private ones from this config.
	 */
	scanners: { orders: OrderScanner; hyperbridge?: HyperbridgeScanner }
	/** --watch-only CLI flag: forces watch-only on every chain. */
	watchOnlyOverride?: boolean
	/**
	 * How this solver signs. Required unless every chain is watch-only, where a
	 * throwaway key stands in for an account that never signs anything.
	 */
	signer?: Signer
}

/** Everything a running filler exposes to the UI server and the CLI. */
export interface FillerRuntime {
	intentFiller: IntentFiller
	balanceProvider: BalanceProvider
	vaultVenue?: VaultFundingPlanner
	/** Live FillerPricePolicy handles shared with the trading engine, one per curve-priced pair. */
	adminStrategies: AdminStrategy[]
	/** Self-halt visibility/reset for the trading engine (overfill protection). */
	haltControls: HaltControl[]
	/** Live order-activity bridge; emits `event` per stored row. */
	activity: ActivityRecorder
	/** The persistence backend this filler was started with. */
	data: SimplexDataStore
	configService: FillerConfigService
	/** Owns the per-chain viem clients; chain edits invalidate through it. */
	chainClientManager: ChainClientManager
	/** The order scanner this filler reads from, whether it built it or was handed one. */
	orderScanner: OrderScanner
	/**
	 * True when the operator asked for watch-only across the board, rather than
	 * per chain. Boot expands it over the chains it knows; keeping the intent is
	 * what lets a chain added later inherit it instead of quietly filling.
	 */
	globalWatchOnly: boolean
	/**
	 * True when this filler was started without a signer — a watch-only observer
	 * running on a throwaway key. The chain controller refuses to take a chain
	 * out of watch-only while this is set: the generated key holds no funds and
	 * dies with the process, so "filling" from it can only burn bids.
	 */
	signerless: boolean
	/** The live confirmation policy the engine prices with; runtime chain adds install into it. */
	confirmationPolicy?: ConfirmationPolicy
	/** This filler's logging destination. */
	loggers: LoggerContext
	/** Symbol-to-address resolution for the configured chains (send options, balance labels). */
	assetRegistry: AssetRegistry
	/** The live trading engine, absent when the config declared no pairs. */
	engine?: FXFiller
	/** The engine's live pair array (same instance), indexed 1:1 with config.pairs. */
	tradingPairs?: TradingPair[]
	/** BalanceProvider's live token1 map (chain name to exotic addresses); mutations apply on its next refresh. */
	balanceTokens: Record<string, string[]>
	rebalancingService?: RebalancingService
	resolvedChains: ResolvedChainConfig[]
	fillerAddress: HexString
	watchOnly?: Record<number, boolean>
	config: FillerTomlConfig
	/** Where the config came from, when it came from a file. */
	configPath?: string
	/**
	 * Hydrates throwaway state for a prospective vault set (read-only on-chain
	 * calls) so hydration-time errors — unconfigured chain, same-asset
	 * duplicates, non-vault address — surface before the set is persisted.
	 */
	vaultPreflight(vaults: VaultToml[]): Promise<void>
	/** Operator-initiated outbound transfers (dashboard Send). */
	tokenSender: TokenSender
	startedAt: number
	/** Stops everything bootFiller started. Idempotent; does NOT process.exit. */
	shutdown(signal: string): Promise<void>
}

/**
 * True when every resolved chain is marked watch-only, so nothing ever needs
 * signing. Exported for its tests: this is the exemption that admits a
 * signerless boot, and a bug here puts a throwaway key to work filling.
 */
export function allChainsWatchOnly(watchOnly: Record<number, boolean> | undefined, chains: ResolvedChainConfig[]): boolean {
	if (!watchOnly) return false
	return chains.every((chain) => watchOnly[chain.chainId] === true)
}

/** One TOML pair to the engine's TradingPair shape (curves become live policies). */
export function tradingPairFrom(pair: PairConfig): TradingPair {
	return {
		token0: pair.token0,
		token1: pair.token1,
		// Optional: absent means uncapped. Reference-only pairs never fill, so the
		// cap is never consulted for them either way.
		maxOrderSize: pair.maxOrderSize === undefined ? undefined : new Decimal(pair.maxOrderSize),
		referenceOnly: pair.referenceOnly === true,
		bidPricePolicy: pair.bidPriceCurve?.length ? new FillerPricePolicy({ points: pair.bidPriceCurve }) : undefined,
		askPricePolicy: pair.askPriceCurve?.length ? new FillerPricePolicy({ points: pair.askPriceCurve }) : undefined,
	}
}

/**
 * Editable view of one trading pair for the UI server, or null for
 * venue-priced (curve-less) pairs — they have nothing to edit. The
 * enableSide/disableSide/setMaxOrderSize/clearMaxOrderSize closures mutate the
 * live TradingPair:
 * the engine reads curve presence and the cap per order, so assignment opens or
 * closes a direction and resizes the market from the next evaluation.
 */
export function adminStrategyFor(
	pair: TradingPair,
	pairIndex: number,
	index: number,
	logger: Logger = getLogger("cli"),
): AdminStrategy | null {
	if (!pair.bidPricePolicy && !pair.askPricePolicy) return null
	const sameToken = normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1)
	const adminStrategy: AdminStrategy = {
		index,
		pairIndex,
		exotic: `${pair.token0}/${pair.token1}${pair.referenceOnly ? " (reference)" : ""}`,
		token0: pair.token0,
		token1: pair.token1,
		bid: pair.bidPricePolicy,
		ask: pair.askPricePolicy,
		sameToken,
		referenceOnly: pair.referenceOnly === true,
	}
	// A reference pair's cap is never consulted (it never fills), so leave it
	// absent rather than surfacing an editable field that does nothing. An
	// uncapped pair reports no current value but can still be given one.
	if (pair.referenceOnly !== true) {
		adminStrategy.maxOrderSize = pair.maxOrderSize?.toString()
		adminStrategy.setMaxOrderSize = (value) => {
			const previous = pair.maxOrderSize?.toString() ?? "uncapped"
			pair.maxOrderSize = new Decimal(value)
			adminStrategy.maxOrderSize = value
			logger.warn(
				{ pair: `${pair.token0}/${pair.token1}`, previous, next: value },
				"Per-order cap resized by operator",
			)
		}
		adminStrategy.clearMaxOrderSize = () => {
			const previous = pair.maxOrderSize?.toString() ?? "uncapped"
			pair.maxOrderSize = undefined
			adminStrategy.maxOrderSize = undefined
			logger.warn(
				{ pair: `${pair.token0}/${pair.token1}`, previous },
				"Per-order cap removed by operator — market is now uncapped",
			)
		}
	}
	// Reference pairs never fill, so opening a side is a no-op; same-token
	// markets are ask-only by engine rule.
	if (!sameToken && !pair.referenceOnly) {
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
	return adminStrategy
}

/**
 * Boots the filler from a validated config: resolves chains, wires services and
 * the pair-trading engine, starts the IntentFiller and background timers. Used
 * by `simplex run` and by the setup wizard's save-and-start transition.
 */
export async function bootFiller(config: FillerTomlConfig, options: BootOptions): Promise<FillerRuntime> {
	validateConfig(config, options.watchOnlyOverride === true)

	const logger = moduleLogger(options.loggers, "cli")
	logger.info({ configPath: options.configPath }, "Loading configuration")
	logger.info("Starting Filler...")

	logger.info("Initializing services...")

	// Everything constructed past this point needs tearing down if a later step
	// throws. `initialize()` legitimately rejects (delegation on an unfunded EOA),
	// and without this the Hyperbridge socket, scanners and timers outlive the
	// failure with nothing holding a reference to stop them.
	const started: Array<() => Promise<void> | void> = []
	const unwind = async () => {
		for (const stop of started.reverse()) {
			try {
				await stop()
			} catch (err) {
				logger.warn({ err }, "Cleanup step failed while unwinding a failed boot")
			}
		}
	}

	logger.info("Resolving chain IDs from RPC endpoints...")
	const resolvedChains: ResolvedChainConfig[] = await resolveChainConfigs(config.chains)
	logger.info({ chains: resolvedChains.map((c) => c.chainId) }, "Chain IDs resolved")

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: config.simplex.maxConcurrentOrders ?? DEFAULT_MAX_CONCURRENT_ORDERS,
		logging: config.simplex.logging as LogLevel | undefined,
		substratePrivateKey: config.simplex.substratePrivateKey,
		hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
		entryPointAddress: config.simplex.entryPointAddress,
		rebalancing: config.rebalancing,
		targetGasUnits: config.simplex.targetGasUnits,
		bidValiditySeconds: config.simplex.bidValiditySeconds,
		blockScanIntervalSeconds: config.simplex.blockScanIntervalSeconds,
		gasFeeBump: config.simplex.gasFeeBump,
		overfillProtection: config.simplex.overfillProtection,
		allowlist: config.allowlist,
	}

	const configService = new FillerConfigService(resolvedChains, fillerConfigForService, options.loggers)

	const chainConfigs: ChainConfig[] = resolvedChains.map((chain) => {
		const chainName = `EVM-${chain.chainId}`
		return configService.getChainConfig(chainName)
	})

	// Create filler configuration
	// Handle watchOnly: can be boolean (global) or Record<string, boolean> (per-chain)
	let watchOnlyConfig: Record<number, boolean> | undefined
	const globalWatchOnly = options.watchOnlyOverride === true || config.simplex.watchOnly === true
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
					throw new Error(
						`simplex.watchOnly key '${chainIdStr}' is not a chain id — use "EVM-<id>" or "<id>"`,
					)
				}
				watchOnlyConfig![chainId] = value === true
			})
		}
	}

	const fillerConfig: FillerConfig = {
		maxConcurrentOrders: config.simplex.maxConcurrentOrders ?? DEFAULT_MAX_CONCURRENT_ORDERS,
		watchOnly: watchOnlyConfig,
		acceptedSourceChains: config.simplex.acceptedSourceChains,
		// Same list the V4 funding venue is built from, so a position can never back a fill without
		// also being declared to the snapshot that measures the depth behind it.
		uniswapV4PositionsByChain: (config.vault?.uniswapV4?.positions ?? []).reduce<Record<string, string[]>>(
			(byChain, row) => {
				;(byChain[row.chain] ??= []).push(String(row.tokenId))
				return byChain
			},
			{},
		),
	} as FillerConfig

	// Create shared services to avoid duplicate RPC calls and reuse connections
	const sharedCacheService = new CacheService(options.loggers)
	// Watch-only chains sign nothing, so ChainClientManager stands a throwaway key
	// in for the missing signer. Anywhere else, a missing signer means a solver
	// that would bid with an address nobody funded — refuse to start instead.
	if (!options.signer && !globalWatchOnly && !allChainsWatchOnly(watchOnlyConfig, resolvedChains)) {
		throw new Error(
			"A signer is required unless every chain is watch-only: pass `signer` to Simplex.start " +
				"(privateKeySigner, turnkeySigner, mpcVaultSigner, viemSigner, or your own Signer implementation). " +
				"Running the binary? Configure it under [simplex.signer].",
		)
	}
	const chainClientManager = new ChainClientManager(configService, options.signer)
	const runtimeSigner: Signer = chainClientManager.getSigner()
	if (options.signer) {
		const strategy = options.signer.mode ?? "custom"
		options.loggers
			.get("signer")
			.info({ signingStrategy: strategy, address: runtimeSigner.address }, `EVM signing strategy: ${strategy}`)
	} else {
		// The throwaway key. Say so, so nobody funds the address in the logs.
		options.loggers
			.get("signer")
			.info({ address: runtimeSigner.address }, "Watch-only: no signer, using a throwaway key")
	}

	const contractService = new ContractInteractionService(
		chainClientManager,
		configService,
		runtimeSigner,
		sharedCacheService,
	)

	// Bid records are how submitted deposits are found again for retraction, so
	// the store is wired in before anything can bid.
	const bidStore = options.data.bids

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
	// Held so ChainController can install a curve for a chain added at runtime.
	let confirmationPolicy: ConfirmationPolicy | undefined
	let tradingPairs: TradingPair[] | undefined
	let engine: FXFiller | undefined
	if (config.pairs?.length) {
		tradingPairs = config.pairs.map(tradingPairFrom)
		tradingPairs.forEach((pair, pairIndex) => {
			const adminStrategy = adminStrategyFor(pair, pairIndex, adminStrategies.length, logger)
			if (adminStrategy) adminStrategies.push(adminStrategy)
		})

		// Orders can be sourced on any configured chain (watch-only ones
		// included), so each needs a confirmation curve — fail at boot,
		// not with silently dropped orders at fill time. Same construction the
		// wizard write gates run against the selected chain ids.
		confirmationPolicy = assertConfirmationCoverage(
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

		engine = new FXFiller(
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
		await vaultVenue.initialise(runtimeSigner.address as HexString)
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
		options.scanners,
		rebalancingService,
		bidStore,
	)

	started.push(() => intentFiller.stop())

	// Initialize (sets up EIP-7702 delegation if solver selection is configured)
	try {
		await intentFiller.initialize()
	} catch (error) {
		await unwind()
		throw error
	}

	// Order-activity feed for the operator UI
	const activity = new ActivityRecorder(options.data.activity, options.loggers)
	activity.attach(intentFiller.monitor)
	if (vaultVenue) {
		vaultVenue.onTx = ({ chain, kind, txHash, sponsored }) => {
			options.data.activity
				.recordWalletTx({
					kind,
					chainId: parseChainKey(chain),
					token: null,
					amount: null,
					to: null,
					txHash,
					sponsored,
				})
				.catch((err) => logger.warn({ err }, "Failed to record vault tx in wallet history"))
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
		fillerAddress: runtimeSigner.address,
		token1,
		hyperbridge: intentFiller.hyperbridgeConnection,
		substratePrivateKey: config.simplex.substratePrivateKey,
	})

	// Read operator state BEFORE anything starts: this is a call into a
	// caller-supplied store, and a rejection after start() would leave a filler
	// committing capital with no shutdown handle while boot reports failure.
	let restoredState: Awaited<ReturnType<typeof options.data.state.get>>
	try {
		restoredState = await options.data.state.get()
	} catch (error) {
		await unwind()
		throw error
	}

	// Start the filler
	intentFiller.start()

	// An operator-initiated pause survives restarts
	if (restoredState.paused) {
		intentFiller.pause()
	}



	// Start the vault threshold-sweep timer (lifecycle owned here, not by the filler)
	started.push(() => vaultVenue?.stopSweeping())
	vaultVenue?.startSweeping()

	started.push(() => balanceProvider.stop())
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
			maxConcurrentOrders: config.simplex.maxConcurrentOrders ?? DEFAULT_MAX_CONCURRENT_ORDERS,
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
		balanceProvider.stop()
		vaultVenue?.stopSweeping()
		await intentFiller.stop()
		// Exit all vault positions back to the underlying asset (best-effort).
		await vaultVenue?.redeemAll()
		activity.detach()
		if (options.ownsData) await options.data.close?.()
	}

	return {
		intentFiller,
		balanceProvider,
		vaultVenue,
		adminStrategies,
		haltControls,
		activity,
		data: options.data,
		configService,
		chainClientManager,
		orderScanner: options.scanners.orders,
		globalWatchOnly,
		signerless: !options.signer,
		confirmationPolicy,
		loggers: options.loggers,
		assetRegistry,
		engine,
		tradingPairs,
		balanceTokens: token1,
		rebalancingService,
		resolvedChains,
		fillerAddress: runtimeSigner.address as HexString,
		watchOnly: watchOnlyConfig,
		config,
		configPath: options.configPath,
		startedAt: Date.now(),
		tokenSender: new TokenSender(
			chainClientManager,
			runtimeSigner.address as HexString,
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
					runtimeSigner.address as HexString,
					chainClientManager,
				).hydrate()
			}
		},
		shutdown,
	}
}
