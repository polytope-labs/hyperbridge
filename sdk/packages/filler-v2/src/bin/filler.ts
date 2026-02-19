#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { IntentFiller } from "../core/filler.js"
import { BasicFiller } from "../strategies/basic.js"
import { ConfirmationPolicy, FillerBpsPolicy } from "../config/interpolated-curve.js"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	UserProvidedChainConfig,
	FillerConfig as FillerServiceConfig,
	LoggingConfig,
} from "../services/FillerConfigService.js"
import { ChainClientManager } from "../services/ChainClientManager.js"
import { ContractInteractionService } from "../services/ContractInteractionService.js"
import { RebalancingService } from "../services/RebalancingService.js"
import { getLogger, configureLogger } from "../services/Logger.js"
import { CacheService } from "../services/CacheService.js"
import { BidStorageService } from "../services/BidStorageService.js"
import type { BinanceCexConfig } from "../services/rebalancers/index.js"
import { Decimal } from "decimal.js"

// ASCII art header
const ASCII_HEADER = `
███████╗██╗██╗     ██╗     ███████╗██████╗     ██╗   ██╗██████╗
██╔════╝██║██║     ██║     ██╔════╝██╔══██╗    ██║   ██║╚════██╗
█████╗  ██║██║     ██║     █████╗  ██████╔╝    ██║   ██║ █████╔╝
██╔══╝  ██║██║     ██║     ██╔══╝  ██╔══██╗    ╚██╗ ██╔╝██╔═══╝
██║     ██║███████╗███████╗███████╗██║  ██║     ╚████╔╝ ███████╗
╚═╝     ╚═╝╚══════╝╚══════╝╚══════╝╚═╝  ╚═╝      ╚═══╝  ╚══════╝
                    Hyperbridge IntentGatewayV2

`

// Get package.json path
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const packageJsonPath = resolve(__dirname, "../../package.json")
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"))

interface StrategyConfig {
	type: "basic"
	/**
	 * Array of (amount, value) coordinates defining the BPS curve.
	 * value = basis points at that order amount
	 */
	bpsCurve: Array<{
		amount: string
		value: number
	}>
}

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

interface PendingQueueConfig {
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
	filler: {
		privateKey: string
		maxConcurrentOrders: number
		pendingQueue: PendingQueueConfig
		logging?: LoggingConfig
		watchOnly?: boolean | Record<string, boolean>
		substratePrivateKey?: string
		hyperbridgeWsUrl?: string
		entryPointAddress?: string
		solverAccountContractAddress?: string
		/** Directory for persistent data storage (bids database, etc.) */
		dataDir?: string
		bundlerUrl?: string
	}
	strategies: StrategyConfig[]
	chains: UserProvidedChainConfig[]
	confirmationPolicies: Record<string, ChainConfirmationPolicy>
	rebalancing?: RebalancingConfig
	binance?: BinanceConfig
}

const program = new Command()

program.name("filler").description("Hyperbridge IntentGatewayV2 FillerV2").version(packageJson.version)

program
	.command("run")
	.description("Run the intent filler with the specified configuration")
	.requiredOption("-c, --config <path>", "Path to TOML configuration file")
	.option("--watch-only", "Watch-only mode: monitor orders without executing fills", false)
	.action(async (options: { config: string; watchOnly?: boolean }) => {
		try {
			// Display ASCII art header
			process.stdout.write(ASCII_HEADER)

			const configPath = resolve(process.cwd(), options.config)
			const tomlContent = readFileSync(configPath, "utf-8")
			const config = parse(tomlContent) as FillerTomlConfig

			validateConfig(config)

			// Configure logger based on config BEFORE creating any services
			if (config.filler.logging) {
				configureLogger(config.filler.logging)
			}

			const logger = getLogger("cli")
			logger.info({ configPath }, "Loading configuration")
			logger.info("Starting Hyperbridge IntentGatewayV2 FillerV2...")

			logger.info("Initializing services...")

			const fillerChainConfigs: UserProvidedChainConfig[] = config.chains.map((chain) => ({
				chainId: chain.chainId,
				rpcUrl: chain.rpcUrl,
			}))

			const fillerConfigForService: FillerServiceConfig = {
				privateKey: config.filler.privateKey,
				maxConcurrentOrders: config.filler.maxConcurrentOrders,
				logging: config.filler.logging,
				substratePrivateKey: config.filler.substratePrivateKey,
				hyperbridgeWsUrl: config.filler.hyperbridgeWsUrl,
				entryPointAddress: config.filler.entryPointAddress,
				solverAccountContractAddress: config.filler.solverAccountContractAddress,
				dataDir: config.filler.dataDir,
				bundlerUrl: config.filler.bundlerUrl,
				rebalancing: config.rebalancing,
			}

			const configService = new FillerConfigService(fillerChainConfigs, fillerConfigForService)

			const chainConfigs: ChainConfig[] = config.chains.map((chain) => {
				// Get the chain name from chain ID for SDK compatibility
				const chainName = `EVM-${chain.chainId}`
				return configService.getChainConfig(chainName)
			})

			// Initialize confirmation policy
			const confirmationPolicy = new ConfirmationPolicy(config.confirmationPolicies)

			// Create filler configuration
			// Handle watchOnly: can be boolean (global) or Record<string, boolean> (per-chain)
			let watchOnlyConfig: Record<number, boolean> | undefined
			if (options.watchOnly) {
				// CLI flag overrides config - apply to all chains
				watchOnlyConfig = {}
				config.chains.forEach((chain) => {
					watchOnlyConfig![chain.chainId] = true
				})
			} else if (config.filler.watchOnly !== undefined) {
				if (typeof config.filler.watchOnly === "boolean") {
					// Global watch-only mode
					watchOnlyConfig = {}
					config.chains.forEach((chain) => {
						watchOnlyConfig![chain.chainId] = config.filler.watchOnly as boolean
					})
				} else {
					// Per-chain configuration
					watchOnlyConfig = {}
					Object.entries(config.filler.watchOnly).forEach(([chainIdStr, value]) => {
						const chainId = Number.parseInt(chainIdStr, 10)
						if (!Number.isNaN(chainId)) {
							watchOnlyConfig![chainId] = value === true
						}
					})
				}
			}

			const fillerConfig: FillerConfig = {
				confirmationPolicy: {
					getConfirmationBlocks: (chainId: number, amount: number) =>
						confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amount)),
				},
				maxConcurrentOrders: config.filler.maxConcurrentOrders,
				pendingQueueConfig: config.filler.pendingQueue,
				watchOnly: watchOnlyConfig,
			} as FillerConfig

			// Create shared services to avoid duplicate RPC calls and reuse connections
			const sharedCacheService = new CacheService()
			const privateKey = config.filler.privateKey as HexString
			const chainClientManager = new ChainClientManager(configService, privateKey)
			const contractService = new ContractInteractionService(
				chainClientManager,
				privateKey,
				configService,
				sharedCacheService,
				configService.getBundlerUrl(),
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
						return new BasicFiller(
							privateKey,
							configService,
							chainClientManager,
							contractService,
							bpsPolicy,
							bidStorageService,
						)
					}
					default:
						throw new Error(`Unknown strategy type: ${strategyConfig.type}`)
				}
			})

			// Initialize rebalancing service if config is provided
			let rebalancingService: RebalancingService | undefined
			const rebalancingConfig = configService.getRebalancingConfig()
			if (rebalancingConfig) {
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

				rebalancingService = new RebalancingService(
					chainClientManager,
					configService,
					privateKey,
					binanceConfig,
				)
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
				privateKey,
				rebalancingService,
			)

			// Initialize (sets up EIP-7702 delegation if solver selection is configured)
			await intentFiller.initialize()

			// Start the filler
			intentFiller.start()

			const watchOnlyChains = watchOnlyConfig
				? Object.entries(watchOnlyConfig)
						.filter(([, value]) => value === true)
						.map(([chainId]) => Number.parseInt(chainId, 10))
				: []

			logger.info(
				{
					chains: config.chains.map((c) => c.chainId),
					strategies: config.strategies.map((s) => s.type),
					maxConcurrentOrders: config.filler.maxConcurrentOrders,
					watchOnlyChains: watchOnlyChains.length > 0 ? watchOnlyChains : undefined,
				},
				watchOnlyChains.length > 0
					? `Intent filler is running (watch-only on chains: ${watchOnlyChains.join(", ")})`
					: "Intent filler is running",
			)

			// Handle graceful shutdown
			process.on("SIGINT", () => {
				logger.warn("Shutting down intent filler (SIGINT)...")
				intentFiller.stop()
				process.exit(0)
			})

			process.on("SIGTERM", () => {
				logger.warn("Shutting down intent filler (SIGTERM)...")
				intentFiller.stop()
				process.exit(0)
			})

			// Keep the process running
			process.stdin.resume()
		} catch (error) {
			// Use console.error for initial startup errors since logger might not be configured yet
			console.error("Failed to start filler:", error)
			process.exit(1)
		}
	})

function validateConfig(config: FillerTomlConfig): void {
	// Validate required fields
	// Private key is only required if not all chains are in watch-only mode
	const isWatchOnlyGlobal = config.filler?.watchOnly === true
	const isWatchOnlyPerChain =
		config.filler?.watchOnly !== undefined &&
		typeof config.filler.watchOnly === "object" &&
		config.filler.watchOnly !== null &&
		config.chains.every((chain) => {
			const chainIdStr = String(chain.chainId)
			const watchOnlyObj = config.filler.watchOnly as Record<string, boolean>
			return chainIdStr in watchOnlyObj && watchOnlyObj[chainIdStr] === true
		})
	const allChainsWatchOnly = isWatchOnlyGlobal || isWatchOnlyPerChain

	if (!config.filler?.privateKey && !allChainsWatchOnly) {
		throw new Error("Filler private key is required (unless all chains are in watchOnly mode)")
	}

	if ((!config.strategies || config.strategies.length === 0) && !allChainsWatchOnly) {
		throw new Error("At least one strategy must be configured (unless all chains are in watchOnly mode)")
	}

	if (!config.chains || config.chains.length === 0) {
		throw new Error("At least one chain must be configured")
	}

	// Validate chain configurations
	for (const chain of config.chains) {
		if (!chain.chainId) {
			throw new Error(`Chain configuration must have chainId`)
		}
		if (typeof chain.chainId !== "number") {
			throw new Error(`Chain ${chain.chainId} chainId must be a number`)
		}
		if (!chain.rpcUrl) {
			throw new Error(`Chain ${chain.chainId} must have rpcUrl`)
		}
	}

	// Validate strategies
	for (const strategy of config.strategies) {
		if (!strategy.type) {
			throw new Error("Strategy type is required")
		}

		if (!["basic"].includes(strategy.type)) {
			throw new Error(`Invalid strategy type: ${strategy.type}`)
		}

		// Validate BPS curve
		if (!strategy.bpsCurve || !Array.isArray(strategy.bpsCurve) || strategy.bpsCurve.length < 2) {
			throw new Error("Strategy must have a 'bpsCurve' array with at least 2 points")
		}

		for (const point of strategy.bpsCurve) {
			if (point.amount === undefined || point.value === undefined) {
				throw new Error("Each BPS curve point must have 'amount' and 'value'")
			}
		}
	}

	// Validate confirmation policies
	for (const [chainId, policy] of Object.entries(config.confirmationPolicies)) {
		if (!policy.points || !Array.isArray(policy.points) || policy.points.length < 2) {
			throw new Error(
				`Confirmation policy for chain ${chainId} must have a 'points' array with at least 2 points`,
			)
		}

		for (const point of policy.points) {
			if (point.amount === undefined || point.value === undefined) {
				throw new Error(`Each point in confirmation policy for chain ${chainId} must have 'amount' and 'value'`)
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
