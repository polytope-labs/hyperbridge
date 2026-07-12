#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { IntentFiller } from "@/core/filler"
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

const program = new Command()

program
	.name("simplex")
	.description("Simplex: Automated market maker for Hyperbridge IntentGatewayV2")
	.version(packageJson.version)

program
	.command("init")
	.description("Interactively create a filler-config.toml (and optionally start the filler)")
	.option("-o, --output <path>", "Where to write the config", "filler-config.toml")
	.action(async (options: { output: string }) => {
		const { runInit } = await import("@/cli/init")
		await runInit(options)
	})

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
			const strategies = config.strategies.map((strategyConfig) => {
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

			// Start the vault threshold-sweep timer (lifecycle owned here, not by the filler)
			vaultVenue?.startSweeping()

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
				vaultVenue?.stopSweeping()
				await intentFiller.stop()
				// Exit all vault positions back to the underlying asset (best-effort).
				await vaultVenue?.redeemAll()
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


// Parse command line arguments
program.parse(process.argv)

// Show help if no command is provided
if (!process.argv.slice(2).length) {
	program.outputHelp()
}
