#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { IntentFiller } from "@/core/filler"
import { BasicFiller } from "@/strategies/basic"
import { FXFiller } from "@/strategies/fx"
import { ConfirmationPolicy, FillerBpsPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	UserProvidedChainConfig,
	FillerConfig as FillerServiceConfig,
	LoggingConfig,
} from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { RebalancingService } from "@/services/RebalancingService"
import { getLogger, configureLogger } from "@/services/Logger"
import { CacheService } from "@/services/CacheService"
import { BidStorageService } from "@/services/BidStorageService"
import { initializeSignerFromToml, type SignerConfig } from "@/services/wallet"
import { MetricsService } from "@/services/MetricsService"
import type { BinanceCexConfig } from "@/services/rebalancers/index"
import { Decimal } from "decimal.js"
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
	/** Per-chain confirmation policies keyed by chain ID string */
	confirmationPolicies: Record<string, ChainConfirmationPolicy>
}

interface FxStrategyConfig {
	type: "hyperfx"
	/**
	 * Bid price curve: exotic tokens per 1 USD when the filler *buys* exotic from a user
	 * (exotic→stable leg). Should have a higher exotic-per-USD rate than the ask curve so
	 * the filler pays out fewer stablecoins per exotic token received.
	 */
	bidPriceCurve: Array<{
		amount: string
		price: string
	}>
	/**
	 * Ask price curve: exotic tokens per 1 USD when the filler *sells* exotic to a user
	 * (stable→exotic leg). Should have a lower exotic-per-USD rate than the bid curve so
	 * the filler sends fewer exotic tokens per stablecoin received.
	 */
	askPriceCurve: Array<{
		amount: string
		price: string
	}>
	/** Maximum USD value per order */
	maxOrderUsd: string
	/** Map of chain identifier (e.g. "EVM-97") to exotic token contract address */
	exoticTokenAddresses: Record<string, HexString>
	/** Optional per-chain confirmation policies for cross-chain orders */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
}

type StrategyConfig = BasicStrategyConfig | FxStrategyConfig

// Compact number formatter for axis labels
function fmtNum(v: number): string {
	if (v >= 1_000_000) return (v / 1_000_000).toFixed(1).replace(/\.0$/, "") + "M"
	if (v >= 1_000) return (v / 1_000).toFixed(1).replace(/\.0$/, "") + "k"
	if (Number.isInteger(v)) return String(v)
	if (Math.abs(v) < 0.01) return v.toExponential(1)
	return parseFloat(v.toFixed(2)).toString()
}

// Render a 2D point set as ASCII chart rows (chart body + axis row)
// Axes: vertical = amount ($, high at top), horizontal = price/bps (low left, high right)
function renderCurveAscii(
	points: Array<{ x: number; y: number }>,
	yLabel: string,
	chartWidth = 30,
	chartHeight = 5,
): string[] {
	const sorted = [...points].sort((a, b) => a.x - b.x)
	// After flipping: horizontal = y (price/bps), vertical = x (amount)
	const minH = Math.min(...sorted.map((p) => p.y))
	const maxH = Math.max(...sorted.map((p) => p.y))
	const minV = sorted[0].x
	const maxV = sorted[sorted.length - 1].x
	const hRange = maxH - minH || 1
	const vRange = maxV - minV || 1

	const grid: string[][] = Array.from({ length: chartHeight }, () => Array(chartWidth).fill(" "))
	const plotted: Array<{ col: number; row: number }> = []

	for (const p of sorted) {
		const col = Math.round(((p.y - minH) / hRange) * (chartWidth - 1))
		const row = Math.round((1 - (p.x - minV) / vRange) * (chartHeight - 1))
		grid[row][col] = "●"
		plotted.push({ col, row })
	}

	// Connect consecutive points with vertical connectors (sorted by row now)
	const byRow = [...plotted].sort((a, b) => a.row - b.row)
	for (let i = 0; i < byRow.length - 1; i++) {
		const a = byRow[i],
			b = byRow[i + 1]
		for (let r = a.row + 1; r < b.row; r++) {
			const col = Math.round(a.col + ((r - a.row) / (b.row - a.row)) * (b.col - a.col))
			if (grid[r][col] === " ") grid[r][col] = "│"
		}
	}

	// Extend top point flat to left edge (lowest price, highest amount extends left)
	const top = byRow[0]
	for (let c = 0; c < top.col; c++) {
		if (grid[top.row][c] === " ") grid[top.row][c] = "─"
	}

	// Y-axis prefix: vertical axis = amount ($), show maxV at top, label at mid, minV at bottom
	const maxVStr = ("$" + fmtNum(maxV)).padStart(5)
	const minVStr = ("$" + fmtNum(minV)).padStart(5)
	const midLabel = ` ${yLabel.trim().slice(0, 4).padEnd(4)}`
	const midRow = Math.floor(chartHeight / 2)
	const rows: string[] = grid.map((row, i) => {
		let prefix: string
		if (i === 0) prefix = `${maxVStr}│`
		else if (i === chartHeight - 1) prefix = minV !== maxV ? `${minVStr}│` : `     │`
		else if (i === midRow) prefix = `${midLabel}│`
		else prefix = `     │`
		return prefix + row.join("")
	})

	// X-axis row: horizontal axis = price/bps, show min left and max right
	const minHStr = fmtNum(minH)
	const maxHStr = fmtNum(maxH)
	const dashes = chartWidth - minHStr.length - maxHStr.length
	const axisContent = dashes >= 1 ? minHStr + "─".repeat(dashes) + maxHStr : "─".repeat(chartWidth)
	rows.push(`     └${axisContent}`)
	rows.push(`     ${" ".repeat(Math.floor(chartWidth / 2) - 1)}${yLabel.trim()}`)
	return rows
}

// Build a boxed banner with the actual curve plotted inside
function getStrategyBanner(config: StrategyConfig): string {
	let chartRows: string[]
	let title: string
	let subtitle: string

	if (config.type === "basic") {
		const points = config.bpsCurve.map((p) => ({ x: parseFloat(p.amount), y: p.value }))
		chartRows = renderCurveAscii(points, " bps")
		title = "BASIC STRATEGY  ACTIVE"
		subtitle = "adaptive BPS spread curve"
	} else {
		const bidPoints = config.bidPriceCurve.map((p) => ({ x: parseFloat(p.amount), y: parseFloat(p.price) }))
		const askPoints = config.askPriceCurve.map((p) => ({ x: parseFloat(p.amount), y: parseFloat(p.price) }))
		chartRows = [...renderCurveAscii(bidPoints, " bid"), "", ...renderCurveAscii(askPoints, " ask")]
		title = "HYPERFX STRATEGY  ACTIVE"
		subtitle = "adaptive bid/ask FX price curve routing"
	}

	// innerWidth = 44 accommodates the longest chart row: "    └" + 30×"─" + " order($)" = 44 chars
	const innerWidth = 44
	const border = "═".repeat(innerWidth + 2)
	const pad = (s: string) => `  ║ ${s.padEnd(innerWidth)} ║`
	const center = (s: string) => {
		const pad = Math.max(0, innerWidth - s.length)
		return " ".repeat(Math.floor(pad / 2)) + s + " ".repeat(Math.ceil(pad / 2))
	}

	return [
		``,
		`  ╔${border}╗`,
		pad(center(title)),
		pad(""),
		...chartRows.map(pad),
		pad(""),
		pad(center(subtitle)),
		`  ╚${border}╝`,
		``,
	].join("\n")
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
	simplex: {
		// The signer is optional to keep the watch-only mode compatible
		signer?: SignerConfig
		maxConcurrentOrders: number
		pendingQueue: PendingQueueConfig
		logging?: LoggingConfig
		watchOnly?: boolean | Record<string, boolean>
		substratePrivateKey?: string
		hyperbridgeWsUrl?: string
		entryPointAddress?: string
		solverAccountContractAddress?: string
	}
	strategies: StrategyConfig[]
	chains: (UserProvidedChainConfig & { bundlerUrl?: string })[]
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

			// Print strategy banners immediately after the header
			for (const strategyConfig of config.strategies) {
				process.stdout.write(getStrategyBanner(strategyConfig))
			}

			// Configure logger based on config BEFORE creating any services
			if (config.simplex.logging) {
				configureLogger(config.simplex.logging)
			}

			const logger = getLogger("cli")
			logger.info({ configPath }, "Loading configuration")
			logger.info("Starting Filler...")

			logger.info("Initializing services...")

			const fillerChainConfigs: UserProvidedChainConfig[] = config.chains.map((chain) => ({
				chainId: chain.chainId,
				rpcUrl: chain.rpcUrl,
				bundlerUrl: chain.bundlerUrl,
			}))

			const fillerConfigForService: FillerServiceConfig = {
				maxConcurrentOrders: config.simplex.maxConcurrentOrders,
				logging: config.simplex.logging,
				substratePrivateKey: config.simplex.substratePrivateKey,
				hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
				entryPointAddress: config.simplex.entryPointAddress,
				dataDir: options.dataDir,
				rebalancing: config.rebalancing,
			}

			const configService = new FillerConfigService(fillerChainConfigs, fillerConfigForService)

			const chainConfigs: ChainConfig[] = config.chains.map((chain) => {
				// Get the chain name from chain ID for SDK compatibility
				const chainName = `EVM-${chain.chainId}`
				return configService.getChainConfig(chainName)
			})

			// Create filler configuration
			// Handle watchOnly: can be boolean (global) or Record<string, boolean> (per-chain)
			let watchOnlyConfig: Record<number, boolean> | undefined
			if (options.watchOnly) {
				// CLI flag overrides config - apply to all chains
				watchOnlyConfig = {}
				config.chains.forEach((chain) => {
					watchOnlyConfig![chain.chainId] = true
				})
			} else if (config.simplex.watchOnly !== undefined) {
				if (typeof config.simplex.watchOnly === "boolean") {
					// Global watch-only mode
					watchOnlyConfig = {}
					config.chains.forEach((chain) => {
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
				pendingQueueConfig: config.simplex.pendingQueue,
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
						const confirmationPolicy = new ConfirmationPolicy(strategyConfig.confirmationPolicies)
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
						const bidPricePolicy = new FillerPricePolicy({ points: strategyConfig.bidPriceCurve })
						const askPricePolicy = new FillerPricePolicy({ points: strategyConfig.askPriceCurve })
						const fxConfirmationPolicy = strategyConfig.confirmationPolicies
							? new ConfirmationPolicy(strategyConfig.confirmationPolicies)
							: undefined
						if (!fxConfirmationPolicy) {
							logger.warn(
								"No confirmationPolicies configured for hyperfx strategy; cross-chain orders will be skipped",
							)
						}
						return new FXFiller(
							runtimeSigner,
							configService,
							chainClientManager,
							contractService,
							bidPricePolicy,
							askPricePolicy,
							strategyConfig.maxOrderUsd,
							strategyConfig.exoticTokenAddresses,
							fxConfirmationPolicy,
						)
					}
					default:
						throw new Error(`Unknown strategy type: ${(strategyConfig as StrategyConfig).type}`)
				}
			})

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
					const exoticTokenAddresses: Record<string, string> = {}
					for (const s of config.strategies) {
						if (s.type === "hyperfx" && s.exoticTokenAddresses) {
							Object.assign(exoticTokenAddresses, s.exoticTokenAddresses)
						}
					}
					metrics = new MetricsService({
						monitor: intentFiller.monitor,
						bidStorage: bidStorageService,
						chainClientManager,
						configService,
						fillerAddress: runtimeSigner.account.address,
						chains: config.chains.map((c) => c.chainId),
						exoticTokenAddresses,
						hyperbridgeWsUrl: config.simplex.hyperbridgeWsUrl,
						substratePrivateKey: config.simplex.substratePrivateKey,
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
					chains: config.chains.map((c) => c.chainId),
					strategies: config.strategies.map((s) => s.type),
					maxConcurrentOrders: config.simplex.maxConcurrentOrders,
					watchOnlyChains: watchOnlyChains.length > 0 ? watchOnlyChains : undefined,
				},
				watchOnlyChains.length > 0
					? `Intent filler is running (watch-only on chains: ${watchOnlyChains.join(", ")})`
					: "Intent filler is running",
			)

			// Handle graceful shutdown
			process.on("SIGINT", async () => {
				logger.warn("Shutting down intent filler (SIGINT)...")
				metrics?.stop()
				await intentFiller.stop()
				process.exit(0)
			})

			process.on("SIGTERM", async () => {
				logger.warn("Shutting down intent filler (SIGTERM)...")
				metrics?.stop()
				await intentFiller.stop()
				process.exit(0)
			})
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
	const isWatchOnlyPerChain =
		config.simplex?.watchOnly !== undefined &&
		typeof config.simplex.watchOnly === "object" &&
		config.simplex.watchOnly !== null &&
		config.chains.every((chain) => {
			const chainIdStr = String(chain.chainId)
			const watchOnlyObj = config.simplex.watchOnly as Record<string, boolean>
			return chainIdStr in watchOnlyObj && watchOnlyObj[chainIdStr] === true
		})
	const allChainsWatchOnly = isWatchOnlyGlobal || isWatchOnlyPerChain

	const signer = config.simplex?.signer

	if (!signer && !allChainsWatchOnly) {
		throw new Error("Signer configuration is required via [simplex.signer]")
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

			// Validate confirmation policies
			if (!strategy.confirmationPolicies || Object.keys(strategy.confirmationPolicies).length === 0) {
				throw new Error("Basic strategy must have at least one confirmation policy")
			}

			for (const [chainId, policy] of Object.entries(strategy.confirmationPolicies)) {
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
			// Validate bid price curve
			if (
				!strategy.bidPriceCurve ||
				!Array.isArray(strategy.bidPriceCurve) ||
				strategy.bidPriceCurve.length < 2
			) {
				throw new Error("FX strategy must have a 'bidPriceCurve' array with at least 2 points")
			}

			for (const point of strategy.bidPriceCurve) {
				if (point.amount === undefined || point.price === undefined) {
					throw new Error("Each FX bidPriceCurve point must have 'amount' and 'price'")
				}
			}

			// Validate ask price curve
			if (
				!strategy.askPriceCurve ||
				!Array.isArray(strategy.askPriceCurve) ||
				strategy.askPriceCurve.length < 2
			) {
				throw new Error("FX strategy must have an 'askPriceCurve' array with at least 2 points")
			}

			for (const point of strategy.askPriceCurve) {
				if (point.amount === undefined || point.price === undefined) {
					throw new Error("Each FX askPriceCurve point must have 'amount' and 'price'")
				}
			}

			if (!strategy.maxOrderUsd) {
				throw new Error("FX strategy must have 'maxOrderUsd'")
			}

			if (!strategy.exoticTokenAddresses || Object.keys(strategy.exoticTokenAddresses).length === 0) {
				throw new Error("FX strategy must have at least one entry in 'exoticTokenAddresses'")
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
