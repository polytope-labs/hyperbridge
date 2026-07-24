#!/usr/bin/env node
import { Command } from "commander"
import { Decimal } from "decimal.js"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { isAddress } from "viem"
import { IntentFiller } from "@/core/filler"
import { FXFiller, type TradingPair } from "@/strategies/fx"
import type { VaultConfig, FundingVenue, UniswapV4PositionConfig } from "@/funding/types"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import { AssetRegistry, normalizeSymbol, validateAssetDefinitions, type AssetDefinition } from "@/config/asset-registry"
import { validatePairConfigs, type PairConfig } from "@/config/pairs"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	type UserProvidedChainConfig,
	type ResolvedChainConfig,
	type AllowlistConfig,
	FillerConfig as FillerServiceConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { PaymasterKeeperService, type PaymasterKeeperConfig } from "@/services/PaymasterKeeperService"
import { UserOpSender } from "@/services/UserOpSender"
import { RebalancingService } from "@/services/RebalancingService"
import { getLogger, configureLogger, type LogLevel } from "@/services/Logger"
import { CacheService } from "@/services/CacheService"
import { BidStorageService } from "@/services/BidStorageService"
import { initializeSignerFromToml, type SignerConfig } from "@/services/wallet"
import { MetricsService } from "@/services/MetricsService"
import { AdminServer, type AdminStrategy } from "@/services/server/AdminServer"
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

/** TOML row for a Uniswap V4 position; only chain + tokenId required. */
interface UniswapV4PositionToml {
	chain: string
	tokenId: string // bigint as string in TOML
	/**
	 * Optional price guard. When set (alongside `maxDeviationBps`), the filler rejects
	 * orders whenever the pool quote on this chain drifts more than `maxDeviationBps`
	 * from this static reference price (exotic per USD, same units as the bid/ask curves).
	 * Guards against a manipulated, stale, or thin pool.
	 */
	referencePrice?: string
	/** Tolerance in basis points for the price guard. Required when `referencePrice` is set. */
	maxDeviationBps?: number
}

/**
 * TOML row for an ERC-4626 vault entry. `threshold` (absolute human units) is the
 * high-water mark that triggers a sweep down to `minBalance`; omit both for
 * withdraw-only sourcing.
 */
interface VaultToml {
	chain: string
	vault: HexString
	threshold?: string
	minBalance?: string
	redeemOnShutdown?: boolean
}

/**
 * Top-level `[vault]` config — every on-chain liquidity venue in one place:
 * ERC-4626 treasury vaults (withdraw sourcing + threshold sweeping) and
 * Uniswap V4 LP positions (exotic funding + pool-based pricing).
 */
interface VaultTomlConfig {
	vaults?: VaultToml[]
	sweepIntervalMs?: number
	uniswapV4?: {
		positions?: UniswapV4PositionToml[]
		/**
		 * One-sided LP under pool pricing. "bid" only buys token1; "ask" only
		 * sells it. Only valid when no pair has static curves (curves express
		 * one-sidedness by omission). Omit to fill both directions.
		 */
		side?: "bid" | "ask"
		/**
		 * Slippage tolerance (basis points) for LP redemptions and the symmetric
		 * spread around pool mid when venue pricing is used. Default 50.
		 */
		spreadBps?: number
	}
}


/** Sensible defaults based on chain finality characteristics. User config overrides per-chain. */
const DEFAULT_CONFIRMATION_POLICIES: Record<string, ChainConfirmationPolicy> = {
	"1": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 15 },
		],
	}, // Ethereum (~12s blocks, ~24s–3min)
	"56": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 3 },
		],
	}, // BNB Chain (~3s blocks, fast finality)
	"137": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 32 },
		],
	}, // Polygon (~2s blocks, milestone finality)
	"8453": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 90 },
		],
	}, // Base (~2s blocks, L2)
	"42161": {
		points: [
			{ amount: "1000", value: 8 },
			{ amount: "100000", value: 720 },
		],
	}, // Arbitrum (~0.25s blocks, L2)
}

const DEFAULT_ADMIN_PORT = 8686

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
	/**
	 * Optional asset-registry escape hatch: symbol → { chain → address }. Only
	 * needed for assets the built-in registry does not ship, or to override a
	 * shipped address for a deployment. Shipped symbols: USDC, USDT, DAI, CNGN
	 * (SDK chain registry) + curated ZARP, EURC, XSGD, TRYB.
	 */
	assets?: Record<string, AssetDefinition>
	/**
	 * Trading pairs — the entire trading configuration. Each pair prices
	 * `token1` in units of `token0` via its own bid/ask curves and carries a
	 * per-order `maxOrderSize` cap; a same-token pair (token0 == token1) is the
	 * same-asset cross-chain market. Required unless running watch-only.
	 */
	pairs?: PairConfig[]
	/**
	 * Per-chain confirmation policies for cross-chain orders, keyed by chain id.
	 * Merged over built-in defaults (ETH, BSC, Polygon, Base, Arbitrum). The
	 * curve amount axis is the order's token0 notional.
	 */
	confirmationPolicies?: Record<string, ChainConfirmationPolicy>
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
		/** Gas fee bump (percentages added to base gasPrice). Defaults: priority=8%, max=10%. */
		gasFeeBump?: {
			maxPriorityFeePerGasBumpPercent?: number
			maxFeePerGasBumpPercent?: number
		}
		/**
		 * Overfill protection knobs. Defaults: maxOverfillBps=500, maxConsecutiveClamps=3.
		 * `maxOverfillBps` clamps the per-leg output ceiling on every strategy.
		 * `maxConsecutiveClamps` only halts FXFiller, and only when the clamped legs
		 * were priced by an on-chain venue (e.g. Uniswap V4). Offline-curve clamps warn
		 * but never halt.
		 */
		overfillProtection?: {
			maxOverfillBps?: number
			maxConsecutiveClamps?: number
		}
	}
	chains: UserProvidedChainConfig[]
	rebalancing?: RebalancingConfig
	binance?: BinanceConfig
	/** Filler-wide vault config: stablecoin sourcing for fills + threshold sweeping. */
	vault?: VaultTomlConfig
	/** Restricts order processing to listed user addresses. Omit to accept all users. */
	allowlist?: AllowlistConfig
	/** SimplexPaymaster fee-recycling keeper (`paymaster-keeper` subcommand). */
	keeper?: PaymasterKeeperConfig
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
	.option(
		"--admin-port <[host:]port>",
		"Enable the admin server (inflight price curve updates: UI + RPC) on the given address. Unauthenticated; e.g. 8686, 127.0.0.1:8686",
	)
	.action(async (options: { config: string; dataDir?: string; watchOnly?: boolean; port?: string; adminPort?: string }) => {
		try {
			// Display ASCII art header
			process.stdout.write(ASCII_HEADER)

			const configPath = resolve(process.cwd(), options.config)
			const tomlContent = readFileSync(configPath, "utf-8")
			const config = parse(tomlContent) as FillerTomlConfig

			validateConfig(config, options.watchOnly === true)

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

			// Build the trading engine from top-level [[pairs]].
			logger.info("Initializing trading engine...")

			// Asset symbol registry: built-ins (USDC/USDT/DAI/CNGN) resolved from the
			// SDK chain registry, extended/overridden by the user's [assets] table.
			const assetRegistry = new AssetRegistry(configService, config.assets)

			// Editable price curves for the admin server, collected at construction so
			// the server mutates the exact policy instances the engine prices with.
			// Every pair symbol must resolve to a real deployment on at least one
			// configured chain. This catches assets the registry marks absent (the
			// SDK stores zero-address sentinels for undeployed assets) before the
			// engine can match a zero address — which doubles as the native-token
			// sentinel in the fill path — against an order leg.
			const configuredChainNames = resolvedChains.map((chain) => `EVM-${chain.chainId}`)
			const pairSymbols = new Set((config.pairs ?? []).flatMap((p) => [p.token0, p.token1]))
			for (const pair of config.pairs ?? []) {
				for (const symbol of [pair.token0, pair.token1]) {
					const resolvesSomewhere = configuredChainNames.some(
						(chainName) => assetRegistry.getAddress(symbol, chainName) !== null,
					)
					if (!resolvesSomewhere) {
						throw new Error(
							`pairs.${pair.token0}/${pair.token1}: '${symbol}' does not resolve to a deployed contract on any configured chain`,
						)
					}
				}
			}

			// No two distinct symbols may resolve to the SAME contract on a chain
			// (e.g. an [assets] alias of USDC's address). Aliasing collapses a
			// cross-asset pair into a same-asset market — bypassing the same-token
			// safeguards — and makes leg matching order-dependent.
			for (const chainName of configuredChainNames) {
				const addressOwner = new Map<string, string>()
				for (const symbol of pairSymbols) {
					const address = assetRegistry.getAddress(symbol, chainName)?.toLowerCase()
					if (!address) continue
					const owner = addressOwner.get(address)
					if (owner && normalizeSymbol(owner) !== normalizeSymbol(symbol)) {
						throw new Error(
							`assets: '${symbol}' and '${owner}' both resolve to ${address} on ${chainName} — symbols must map to distinct contracts`,
						)
					}
					addressOwner.set(address, symbol)
				}
			}

			const adminStrategies: AdminStrategy[] = []
			const strategies: FXFiller[] = []
			if (config.pairs?.length) {
				const tradingPairs: TradingPair[] = config.pairs.map((pair) => ({
					token0: pair.token0,
					token1: pair.token1,
					maxOrderSize: new Decimal(pair.maxOrderSize),
					bidPricePolicy: pair.bidPriceCurve?.length
						? new FillerPricePolicy({ points: pair.bidPriceCurve })
						: undefined,
					askPricePolicy: pair.askPriceCurve?.length
						? new FillerPricePolicy({ points: pair.askPriceCurve })
						: undefined,
				}))
				for (const pair of tradingPairs) {
					if (pair.bidPricePolicy || pair.askPricePolicy) {
						adminStrategies.push({
							index: adminStrategies.length,
							exotic: `${pair.token0}/${pair.token1}`,
							bid: pair.bidPricePolicy,
							ask: pair.askPricePolicy,
						})
					}
				}

				const confirmationPolicy = new ConfirmationPolicy({
					...DEFAULT_CONFIRMATION_POLICIES,
					...(config.confirmationPolicies ?? {}),
				})

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
					// Collect exotic token addresses (the non-quote side of cross-asset
					// pairs) via the asset registry; same-token pairs have no exotic side.
					const token1: Record<string, string> = {}
					for (const pair of config.pairs ?? []) {
						if (normalizeSymbol(pair.token0) === normalizeSymbol(pair.token1)) continue
						for (const chain of resolvedChains) {
							const chainName = `EVM-${chain.chainId}`
							const address = assetRegistry.getAddress(pair.token1, chainName)
							if (address) token1[chainName] = address
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

			// The admin server for inflight price curve updates is opt-in;
			// it only starts when --admin-port is passed.
			let adminServer: AdminServer | undefined
			if (options.adminPort) {
				let adminHost = "127.0.0.1"
				let adminPort = DEFAULT_ADMIN_PORT
				const [host, portStr] = options.adminPort.includes(":")
					? (options.adminPort.split(":").slice(-2) as [string, string])
					: [adminHost, options.adminPort]
				const parsed = parseInt(portStr, 10)
				if (isNaN(parsed) || parsed < 1 || parsed > 65535) {
					logger.warn(
						{ bind: options.adminPort },
						`Invalid admin address, using default 127.0.0.1:${DEFAULT_ADMIN_PORT}`,
					)
				} else {
					adminHost = host
					adminPort = parsed
				}
				if (adminStrategies.length === 0) {
					logger.warn("No curve-priced pairs are configured; the admin server has nothing editable")
				}
				adminServer = new AdminServer(adminStrategies)
				try {
					await adminServer.start(adminPort, adminHost)
				} catch (err) {
					// The filler is the primary workload; a bind failure (e.g. port in use)
					// costs the admin UI, not the process.
					logger.error({ err, bind: `${adminHost}:${adminPort}` }, "Admin server failed to start")
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
					pairs: (config.pairs ?? []).map((p) => `${p.token0}/${p.token1}`),
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
				adminServer?.stop()
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

program
	.command("paymaster-keeper")
	.description(
		"Run the SimplexPaymaster keeper: periodically recycles accrued stablecoins into the EntryPoint deposit via the paymaster's onchain swapAndDeposit",
	)
	.requiredOption("-c, --config <path>", "Path to TOML configuration file")
	.action(async (options: { config: string }) => {
		try {
			const configPath = resolve(process.cwd(), options.config)
			const config = parse(readFileSync(configPath, "utf-8")) as FillerTomlConfig

			// Only [[chains]], [simplex.signer] and the optional [keeper] block are used.
			if (!config.chains || config.chains.length === 0) {
				throw new Error("At least one chain must be configured")
			}
			if (!config.simplex?.signer) {
				throw new Error("Signer configuration is required via [simplex.signer]")
			}

			if (config.simplex.logging) {
				configureLogger(config.simplex.logging as LogLevel)
			}
			const logger = getLogger("cli")

			const resolvedChains: ResolvedChainConfig[] = await resolveChainConfigs(config.chains)
			const configService = new FillerConfigService(resolvedChains, {
				maxConcurrentOrders: config.simplex.maxConcurrentOrders ?? 1,
				logging: config.simplex.logging as LogLevel | undefined,
				entryPointAddress: config.simplex.entryPointAddress,
			})

			const configuredSigner = await initializeSignerFromToml(config.simplex.signer)
			const chainClientManager = new ChainClientManager(configService, configuredSigner)
			const runtimeSigner: SigningAccount = chainClientManager.getSigner()

			const chains = resolvedChains.map((chain) => `EVM-${chain.chainId}`)
			const keeper = new PaymasterKeeperService(
				chainClientManager,
				configService,
				runtimeSigner,
				config.keeper,
			)
			keeper.start(chains)

			const shutdown = (signal: string) => {
				logger.warn(`Shutting down paymaster keeper (${signal})...`)
				keeper.stop()
				process.exit(0)
			}
			process.on("SIGINT", () => shutdown("SIGINT"))
			process.on("SIGTERM", () => shutdown("SIGTERM"))
		} catch (error) {
			console.error("Failed to start paymaster keeper:", error)
			process.exit(1)
		}
	})

function validateConfig(config: FillerTomlConfig, cliWatchOnly = false): void {
	// The [[strategies]] array was removed when the pair engine subsumed the
	// stable strategy — fail loudly so stale configs are migrated, not ignored.
	if ("strategies" in config) {
		throw new Error(
			"[[strategies]] was removed — declare top-level [[pairs]] instead (a same-token pair like USDC/USDC with an ask curve below par replaces the stable strategy; engine settings moved to [confirmationPolicies] and [vault.uniswapV4])",
		)
	}

	// Private key is only required if not all chains are in watch-only mode.
	// The --watch-only CLI flag forces global watch-only, so honour it here too
	// (otherwise the flag's own config would still trip the signer requirement).
	const allChainsWatchOnly = cliWatchOnly || config.simplex?.watchOnly === true

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

	if (!config.chains || config.chains.length === 0) {
		throw new Error("At least one chain must be configured")
	}

	// Validate chain configurations
	for (const chain of config.chains) {
		if (!Array.isArray(chain.rpcUrls) || chain.rpcUrls.length === 0 || chain.rpcUrls.some((u) => !u)) {
			throw new Error("Each chain configuration must have rpcUrls (a non-empty array of strings)")
		}
		if (!chain.bundlerUrl) {
			throw new Error("Each chain configuration must have bundlerUrl")
		}
	}

	// Validate allowlist addresses (when present)
	if (config.allowlist) {
		for (const user of config.allowlist.users ?? []) {
			if (!isAddress(user)) {
				throw new Error(`allowlist.users contains an invalid address: ${user}`)
			}
		}
		for (const [chain, users] of Object.entries(config.allowlist.bySource ?? {})) {
			if (!Array.isArray(users)) {
				throw new Error(`allowlist.bySource."${chain}" must be an array of addresses`)
			}
			for (const user of users) {
				if (!isAddress(user)) {
					throw new Error(`allowlist.bySource."${chain}" contains an invalid address: ${user}`)
				}
			}
		}
	}

	if (config.vault?.vaults?.length) {
		VaultFundingPlanner.validateConfig(config.vault.vaults)
	}

	// Asset registry and trading pairs — the entire trading configuration.
	if (config.assets) {
		validateAssetDefinitions(config.assets)
	}
	const hasPairs = (config.pairs?.length ?? 0) > 0
	if (!hasPairs && !allChainsWatchOnly) {
		throw new Error("At least one [[pairs]] entry must be configured (unless all chains are in watchOnly mode)")
	}
	const hasVenuePricing = (config.vault?.uniswapV4?.positions?.length ?? 0) > 0
	if (hasPairs) {
		validatePairConfigs(config.pairs!, config.assets, hasVenuePricing)
	}

	// Per-chain confirmation policies (merged over built-in defaults at startup).
	for (const [chainId, policy] of Object.entries(config.confirmationPolicies ?? {})) {
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

	// Uniswap V4 venue config ([vault.uniswapV4]): positions, price guards,
	// one-sided switch, redemption slippage.
	const uniswapV4 = config.vault?.uniswapV4
	if (uniswapV4?.positions?.length) {
		UniswapV4FundingPlanner.validateConfig(uniswapV4.positions)
	}

	if (uniswapV4?.spreadBps !== undefined) {
		if (!Number.isFinite(uniswapV4.spreadBps) || uniswapV4.spreadBps < 0 || uniswapV4.spreadBps > 10_000) {
			throw new Error("vault.uniswapV4: 'spreadBps' must be a number between 0 and 10000")
		}
	}

	// Per-position price guard: referencePrice and maxDeviationBps are optional but
	// must be set together. A given chain may not carry conflicting guard values.
	const guardByChain: Record<string, { referencePrice: string; maxDeviationBps: number }> = {}
	for (const position of uniswapV4?.positions ?? []) {
		const hasRef = position.referencePrice !== undefined
		const hasBps = position.maxDeviationBps !== undefined
		if (hasRef !== hasBps) {
			throw new Error(
				"vault.uniswapV4: a position price guard needs both 'referencePrice' and 'maxDeviationBps', or neither",
			)
		}
		if (!hasRef) continue

		const parsedRef = Number(position.referencePrice)
		if (!Number.isFinite(parsedRef) || parsedRef <= 0) {
			throw new Error(
				`vault.uniswapV4: position 'referencePrice' for chain '${position.chain}' must be a positive number`,
			)
		}
		if (
			!Number.isFinite(position.maxDeviationBps!) ||
			position.maxDeviationBps! <= 0 ||
			position.maxDeviationBps! > 10_000
		) {
			throw new Error(
				`vault.uniswapV4: position 'maxDeviationBps' for chain '${position.chain}' must be a number between 0 (exclusive) and 10000`,
			)
		}
		const existing = guardByChain[position.chain]
		if (
			existing &&
			(existing.referencePrice !== position.referencePrice || existing.maxDeviationBps !== position.maxDeviationBps)
		) {
			throw new Error(`vault.uniswapV4: conflicting price guard values for chain '${position.chain}'`)
		}
		guardByChain[position.chain] = {
			referencePrice: position.referencePrice!,
			maxDeviationBps: position.maxDeviationBps!,
		}
	}

	// One-sided LP under pool pricing: `side` enables a single direction. Only valid
	// with venue pricing and no static curves (curves express one-sided by omission).
	const side = uniswapV4?.side
	if (side !== undefined) {
		if (side !== "bid" && side !== "ask") {
			throw new Error("vault.uniswapV4: 'side' must be either 'bid' or 'ask'")
		}
		if (!hasVenuePricing) {
			throw new Error("vault.uniswapV4: 'side' requires [vault.uniswapV4].positions")
		}
		const anyCurves = (config.pairs ?? []).some(
			(p) => (p.bidPriceCurve?.length ?? 0) > 0 || (p.askPriceCurve?.length ?? 0) > 0,
		)
		if (anyCurves) {
			throw new Error(
				"vault.uniswapV4: 'side' only applies to pool pricing; omit the pair price curves (or drop one curve to do one-sided LP with static pricing)",
			)
		}
	}
}

// Parse command line arguments
program.parse(process.argv)

// Show help if no command is provided
if (!process.argv.slice(2).length) {
	program.outputHelp()
}
