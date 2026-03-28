#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { IntentFiller } from "@/core/filler"
import { BasicFiller } from "@/strategies/basic"
import { FXFiller } from "@/strategies/fx"
import type { AerodromePoolConfig, FundingVenue, UniswapV4PositionConfig } from "@/funding/types"
import { AerodromeFundingPlanner } from "@/funding/aerodrome/AerodromeFundingPlanner"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { ConfirmationPolicy, FillerBpsPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	type UserProvidedChainConfig,
	type ResolvedChainConfig,
	FillerConfig as FillerServiceConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { RebalancingService } from "@/services/RebalancingService"
import { getLogger, configureLogger, type LogLevel } from "@/services/Logger"
import { CacheService } from "@/services/CacheService"
import { BidStorageService } from "@/services/BidStorageService"
import { initializeSignerFromToml, type SignerConfig } from "@/services/wallet"
import { MetricsService } from "@/services/MetricsService"
import type { BinanceCexConfig } from "@/services/rebalancers/index"
import type { SigningAccount } from "@/services/wallet"

// ASCII art header
const ASCII_HEADER = `
███████╗██╗███╗   ███╗██████╗ ██╗     ███████╗██╗  ██╗
██╔════╝██║████╗ ████║██╔══██╗██║     ██╔════╝╚██╗██╔╝
███████╗██║██╔████╔██║██████╔╝██║     █████╗   ╚███╔╝
╚════██║██║██║╚██╔╝██║██╔═══╝ ██║     ██╔══╝   ██╔██╗
███████║██║██║ ╚═╝ ██║██║     ███████╗███████╗██╔╝ ██╗
╚══════╝╚═╝╚═╝     ╚═╝╚═╝     ╚══════╝╚══════╝╚═╝  ╚═╝

`

// Get package.json path
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const packageJsonPath = resolve(__dirname, "../../package.json")
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"))

interface ChainConfirmationPolicy {
	/**
	 * Array of (amount, value) coordinates defining the confirmation curve.
	 * value = number of confirmations at that order amount
	 */
	points: Array<{
		amount: string
		value: number
	}>
}

interface BasicStrategyConfig {
	type: "basic"
	/**
	 * Array of (amount, value) coordinates defining the BPS curve.
	 * value = basis points at that order amount
	 */
	bpsCurve: Array<{
		amount: string
		value: number
	}>
	/** Per-chain confirmation policies keyed by chain ID string. Defaults provided for ETH, BSC, Base, Arbitrum. */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
}

/** TOML row for an Aerodrome pool; `chain` is the state machine id e.g. `EVM-8453`. */
interface AerodromePoolToml extends AerodromePoolConfig {
	chain: string
}

/** TOML row for a Uniswap V4 position; only chain + tokenId needed. */
interface UniswapV4PositionToml {
	chain: string
	tokenId: string // bigint as string in TOML
}

interface FxStrategyConfig {
	type: "hyperfx"
	/**
	 * Bid price curve: exotic tokens per 1 USD when the filler *buys* exotic from a user
	 * (exotic→stable leg). Should have a higher exotic-per-USD rate than the ask curve so
	 * the filler pays out fewer stablecoins per exotic token received.
	 *
	 * Optional when `[strategies.vault.uniswapV4]` lists at least one position — bid/ask
	 * are then derived from the Uniswap V4 pool after startup.
	 */
	bidPriceCurve?: Array<{
		amount: string
		price: string
	}>
	/**
	 * Ask price curve: exotic tokens per 1 USD when the filler *sells* exotic to a user
	 * (stable→exotic leg). Should have a lower exotic-per-USD rate than the bid curve so
	 * the filler sends fewer exotic tokens per stablecoin received.
	 *
	 * Optional when `[strategies.vault.uniswapV4]` lists at least one position — bid/ask
	 * are then derived from the Uniswap V4 pool after startup.
	 */
	askPriceCurve?: Array<{
		amount: string
		price: string
	}>
	/**
	 * Symmetric spread (basis points) around Uniswap V4 pool mid when venue pricing is used.
	 * Ignored when only static bid/ask curves apply.
	 */
	spreadBps?: number
	/** Maximum USD value per order */
	maxOrderUsd: number
	/** Map of chain identifier (e.g. "EVM-97") to exotic token contract address */
	token1: Record<string, HexString>
	/** Optional per-chain confirmation policies for cross-chain orders */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
	/** Optional on-chain liquidity funding for destination-chain outputs */
	vault?: {
		aerodrome?: {
			pools?: AerodromePoolToml[]
		}
		uniswapV4?: {
			positions?: UniswapV4PositionToml[]
		}
	}
}

type StrategyConfig = BasicStrategyConfig | FxStrategyConfig

/** Sensible defaults based on chain finality characteristics. User config overrides per-chain. */
const DEFAULT_CONFIRMATION_POLICIES: Record<string, ChainConfirmationPolicy> = {
	"1":     { points: [{ amount: "1000", value: 2 },  { amount: "100000", value: 15 }] },   // Ethereum (~12s blocks, ~24s–3min)
	"56":    { points: [{ amount: "1000", value: 2 },  { amount: "100000", value: 3 }] },    // BNB Chain (~3s blocks, fast finality)
	"137":   { points: [{ amount: "1000", value: 2 },  { amount: "100000", value: 32 }] },   // Polygon (~2s blocks, milestone finality)
	"8453":  { points: [{ amount: "1000", value: 2 },  { amount: "100000", value: 90 }] },   // Base (~2s blocks, L2)
	"42161": { points: [{ amount: "1000", value: 8 },  { amount: "100000", value: 720 }] },  // Arbitrum (~0.25s blocks, L2)
}

interface QueueConfig {
	maxRechecks: number
	recheckDelayMs: number
}

interface RebalancingConfig {
	triggerPercentage: number
	baseBalances: {
		USDC?: Record<string, string>
		USDT?: Record<string, string>
	}
}

interface BinanceConfig {
	apiKey: string
	apiSecret: string
	basePath?: string
	timeout?: number
	depositTimeoutMs?: number
	pollIntervalMs?: number
	withdrawTimeoutMs?: number
}

interface FillerTomlConfig {
	simplex: {
		// The signer is optional to keep the watch-only mode compatible
		signer?: SignerConfig
		maxConcurrentOrders: number
		queue: QueueConfig
		logging?: string
		watchOnly?: boolean | Record<string, boolean>
		substratePrivateKey: string
		hyperbridgeWsUrl: string
		entryPointAddress?: string
		solverAccountContractAddress?: string
		/** Target gas units for EntryPoint deposits per chain. Defaults to 3,000,000. */
		targetGasUnits?: number
	}
	strategies: StrategyConfig[]
	chains: UserProvidedChainConfig[]
	rebalancing?: RebalancingConfig
	binance?: BinanceConfig
}

const program = new Command()

program
	.name("simplex")
	.description("Simplex: Automated market maker for Hyperbridge IntentGatewayV2")
	.version(packageJson.version)

program
	.command("run")
	.description("Run the intent filler with the specified configuration")
	.requiredOption("-c, --config <path>", "Path to TOML configuration file")
	.option("-d, --data-dir <path>", "Directory for persistent data storage (bids database, etc.)")
	.option("--watch-only", "Watch-only mode: monitor orders without executing fills", false)
	.option(
		"-p, --port <[host:]port>",
		"Enable Prometheus metrics server on the given address (e.g. 9090, 0.0.0.0:9090, 127.0.0.1:9090)",
	)
	.action(async (options: { config: string; dataDir?: string; watchOnly?: boolean; port?: string }) => {
		try {
			// Display ASCII art header
			process.stdout.write(ASCII_HEADER)

			const configPath = resolve(process.cwd(), options.config)
			const tomlContent = readFileSync(configPath, "utf-8")
			const config = parse(tomlContent) as FillerTomlConfig

			validateConfig(config)

			// Configure logger based on config BEFORE creating any services
			if (config.simplex.logging) {
				configureLogger(config.simplex.logging as LogLevel)
			}

			const logger = getLogger("cli")
			logger.info({ configPath }, "Loading configuration")
			logger.info("Starting Filler...")

			logger.info("Initializing services...")

			logger.info("Resolving chain IDs from RPC endpoints...")
			const resolvedChains: ResolvedChainConfig[] = await resolveChainConfigs(config.chains)
			logger.info(
				{ chains: resolvedChains.map((c) => c.chainId) },
				"Chain IDs resolved",
			)

			const fillerConfigForService: FillerServiceConfig = {
				maxConcurrentOrders: config.simplex.maxConcurrentOrders,
				logging: config.simplex.logging as LogLevel | undefined,
				substratePrivateKey: config.simplex.substratePrivateKey,
				hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
				entryPointAddress: config.simplex.entryPointAddress,
				dataDir: options.dataDir,
				rebalancing: config.rebalancing,
				targetGasUnits: config.simplex.targetGasUnits,
			}

			const configService = new FillerConfigService(resolvedChains, fillerConfigForService)

			const chainConfigs: ChainConfig[] = resolvedChains.map((chain) => {
				const chainName = `EVM-${chain.chainId}`
				return configService.getChainConfig(chainName)
			})

			// Create filler configuration
			// Handle watchOnly: can be boolean (global) or Record<string, boolean> (per-chain)
			let watchOnlyConfig: Record<number, boolean> | undefined
			if (options.watchOnly) {
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
			const configuredSigner = initializeSignerFromToml(config.simplex.signer)
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

			// Initialize strategies with shared services
			logger.info("Initializing strategies...")
			const strategies = config.strategies.map((strategyConfig) => {
				switch (strategyConfig.type) {
					case "basic": {
						const bpsPolicy = new FillerBpsPolicy({ points: strategyConfig.bpsCurve })
						const mergedBasicPolicies = {
							...DEFAULT_CONFIRMATION_POLICIES,
							...(strategyConfig.confirmationPolicies ?? {}),
						}
						const confirmationPolicy = new ConfirmationPolicy(mergedBasicPolicies)
						return new BasicFiller(
							runtimeSigner,
							configService,
							chainClientManager,
							contractService,
							bpsPolicy,
							confirmationPolicy,
						)
					}
					case "hyperfx": {
						const bidPricePolicy = strategyConfig.bidPriceCurve?.length
							? new FillerPricePolicy({ points: strategyConfig.bidPriceCurve })
							: undefined
						const askPricePolicy = strategyConfig.askPriceCurve?.length
							? new FillerPricePolicy({ points: strategyConfig.askPriceCurve })
							: undefined
						const mergedFxPolicies = {
							...DEFAULT_CONFIRMATION_POLICIES,
							...(strategyConfig.confirmationPolicies ?? {}),
						}
						const fxConfirmationPolicy = new ConfirmationPolicy(mergedFxPolicies)
						const fundingVenues: FundingVenue[] = []
						if (strategyConfig.vault?.aerodrome?.pools?.length) {
							const poolsByChain: Record<string, AerodromePoolConfig[]> = {}
							for (const row of strategyConfig.vault.aerodrome.pools) {
								const chain = row.chain
								if (!poolsByChain[chain]) poolsByChain[chain] = []
								poolsByChain[chain].push({
									pair: row.pair,
									gauge: row.gauge,
								})
							}
							fundingVenues.push(
								new AerodromeFundingPlanner(chainClientManager, { poolsByChain }, configService, strategyConfig.spreadBps),
							)
						}
						if (strategyConfig.vault?.uniswapV4?.positions?.length) {
							const positionsByChain: Record<string, UniswapV4PositionConfig[]> = {}
							for (const row of strategyConfig.vault.uniswapV4.positions) {
								const chain = row.chain
								if (!positionsByChain[chain]) positionsByChain[chain] = []
								positionsByChain[chain].push({
									tokenId: BigInt(row.tokenId),
								})
							}
							fundingVenues.push(
								new UniswapV4FundingPlanner(chainClientManager, { positionsByChain }, configService, strategyConfig.spreadBps),
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
							},
						)
					}
					default:
						throw new Error(`Unknown strategy type: ${(strategyConfig as StrategyConfig).type}`)
				}
			})

			// Initialise FXFiller strategies (hydrate funding venue state)
			for (const strategy of strategies) {
				if (strategy instanceof FXFiller) {
					logger.info("Hydrating funding venue state...")
					await strategy.initialise()
				}
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

			// Start optional Prometheus metrics server
			let metrics: MetricsService | undefined
			if (options.port) {
				const [metricsHost, metricsPortStr] = options.port.includes(":")
					? (options.port.split(":").slice(-2) as [string, string])
					: ["0.0.0.0", options.port]
				const metricsPort = parseInt(metricsPortStr, 10)
				if (isNaN(metricsPort) || metricsPort < 1 || metricsPort > 65535) {
					logger.warn({ bind: options.port }, "Invalid metrics address, skipping")
				} else {
					// Collect exotic token addresses from FX strategies
					const token1: Record<string, string> = {}
					for (const s of config.strategies) {
						if (s.type === "hyperfx" && s.token1) {
							Object.assign(token1, s.token1)
						}
					}
					metrics = new MetricsService({
						monitor: intentFiller.monitor,
						bidStorage: bidStorageService,
						chainClientManager,
						configService,
						fillerAddress: runtimeSigner.account.address,
						chains: resolvedChains.map((c) => c.chainId),
						token1,
						hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
						substratePrivateKey: config.simplex.substratePrivateKey,
						dataDir: options.dataDir,
					})
					metrics.start(metricsPort, metricsHost)
				}
			}

			// Start the filler
			intentFiller.start()

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

			// Handle graceful shutdown
			const shutdown = async (signal: string) => {
				logger.warn(`Shutting down intent filler (${signal})...`)
				metrics?.stop()
				await intentFiller.stop()
				process.exit(0)
			}

			process.on("SIGINT", () => shutdown("SIGINT"))
			process.on("SIGTERM", () => shutdown("SIGTERM"))
		} catch (error) {
			// Use console.error for initial startup errors since logger might not be configured yet
			console.error("Failed to start filler:", error)
			process.exit(1)
		}
	})

function validateConfig(config: FillerTomlConfig): void {
	// Validate required fields
	// Private key is only required if not all chains are in watch-only mode
	const isWatchOnlyGlobal = config.simplex?.watchOnly === true
	const allChainsWatchOnly = isWatchOnlyGlobal

	const signer = config.simplex?.signer

	if (!signer && !allChainsWatchOnly) {
		throw new Error("Signer configuration is required via [simplex.signer]")
	}

	if (!config.simplex?.substratePrivateKey) {
		throw new Error("simplex.substratePrivateKey is required")
	}

	if (!config.simplex?.hyperbridgeWsUrl) {
		throw new Error("simplex.hyperbridgeWsUrl is required")
	}

	if ((!config.strategies || config.strategies.length === 0) && !allChainsWatchOnly) {
		throw new Error("At least one strategy must be configured (unless all chains are in watchOnly mode)")
	}

	if (!config.chains || config.chains.length === 0) {
		throw new Error("At least one chain must be configured")
	}

	// Validate chain configurations
	for (const chain of config.chains) {
		if (!chain.rpcUrl) {
			throw new Error("Each chain configuration must have rpcUrl")
		}
		if (!chain.bundlerUrl) {
			throw new Error("Each chain configuration must have bundlerUrl")
		}
	}

	// Validate strategies
	for (const strategy of config.strategies) {
		if (!strategy.type) {
			throw new Error("Strategy type is required")
		}

		if (!["basic", "hyperfx"].includes(strategy.type)) {
			throw new Error(`Invalid strategy type: ${strategy.type}`)
		}

		if (strategy.type === "basic") {
			// Validate BPS curve
			if (!strategy.bpsCurve || !Array.isArray(strategy.bpsCurve) || strategy.bpsCurve.length < 2) {
				throw new Error("Basic strategy must have a 'bpsCurve' array with at least 2 points")
			}

			for (const point of strategy.bpsCurve) {
				if (point.amount === undefined || point.value === undefined) {
					throw new Error("Each BPS curve point must have 'amount' and 'value'")
				}
			}

			// Validate user-provided confirmation policies (defaults are always present)
			for (const [chainId, policy] of Object.entries(strategy.confirmationPolicies ?? {})) {
				if (!policy.points || !Array.isArray(policy.points) || policy.points.length < 2) {
					throw new Error(
						`Confirmation policy for chain ${chainId} must have a 'points' array with at least 2 points`,
					)
				}
				for (const point of policy.points) {
					if (point.amount === undefined || point.value === undefined) {
						throw new Error(
							`Each point in confirmation policy for chain ${chainId} must have 'amount' and 'value'`,
						)
					}
				}
			}
		}

		if (strategy.type === "hyperfx") {
			if (strategy.vault?.aerodrome?.pools?.length) {
				AerodromeFundingPlanner.validateConfig(strategy.vault.aerodrome.pools)
			}
			if (strategy.vault?.uniswapV4?.positions?.length) {
				UniswapV4FundingPlanner.validateConfig(strategy.vault.uniswapV4.positions)
			}

			const bidLen = strategy.bidPriceCurve?.length ?? 0
			const askLen = strategy.askPriceCurve?.length ?? 0
			if (bidLen > 0 !== askLen > 0) {
				throw new Error(
					"hyperfx: set both 'bidPriceCurve' and 'askPriceCurve', or omit both when using vault.uniswapV4 for pricing",
				)
			}

			const hasStaticCurves = bidLen >= 2 && askLen >= 2
			const hasUniswapV4Positions = (strategy.vault?.uniswapV4?.positions?.length ?? 0) > 0

			if (!hasStaticCurves && !hasUniswapV4Positions) {
				throw new Error(
					"hyperfx: provide bid+ask price curves (≥2 points each) or configure [strategies.vault.uniswapV4].positions for pool-based pricing",
				)
			}

			if (strategy.spreadBps !== undefined) {
				if (!Number.isFinite(strategy.spreadBps) || strategy.spreadBps < 0 || strategy.spreadBps > 10_000) {
					throw new Error("hyperfx: 'spreadBps' must be a number between 0 and 10000")
				}
			}

			if (bidLen > 0) {
				if (!hasStaticCurves) {
					throw new Error("hyperfx: bid and ask price curves must each have at least 2 points when provided")
				}
				for (const point of strategy.bidPriceCurve!) {
					if (point.amount === undefined || point.price === undefined) {
						throw new Error("Each FX bidPriceCurve point must have 'amount' and 'price'")
					}
				}
				for (const point of strategy.askPriceCurve!) {
					if (point.amount === undefined || point.price === undefined) {
						throw new Error("Each FX askPriceCurve point must have 'amount' and 'price'")
					}
				}
			}

			if (!strategy.maxOrderUsd) {
				throw new Error("FX strategy must have 'maxOrderUsd'")
			}

			if (!strategy.token1 || Object.keys(strategy.token1).length === 0) {
				throw new Error("FX strategy must have at least one entry in 'token1'")
			}

			if (strategy.confirmationPolicies) {
				for (const [chainId, policy] of Object.entries(strategy.confirmationPolicies)) {
					if (!policy.points || !Array.isArray(policy.points) || policy.points.length < 2) {
						throw new Error(
							`FX confirmation policy for chain ${chainId} must have a 'points' array with at least 2 points`,
						)
					}
					for (const point of policy.points) {
						if (point.amount === undefined || point.value === undefined) {
							throw new Error(
								`Each point in FX confirmation policy for chain ${chainId} must have 'amount' and 'value'`,
							)
						}
					}
				}
			}
		}
	}
}

// Parse command line arguments
program.parse(process.argv)

// Show help if no command is provided
if (!process.argv.slice(2).length) {
	program.outputHelp()
}
