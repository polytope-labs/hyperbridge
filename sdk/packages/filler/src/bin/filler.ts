#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { IntentFiller } from "../core/filler.js"
import { BasicFiller } from "../strategies/basic.js"
import { StableSwapFiller } from "../strategies/swap.js"
import { ConfirmationPolicy } from "../config/confirmation-policy.js"
import { ChainConfig, FillerConfig, HexString } from "@hyperbridge/sdk"
import {
	FillerConfigService,
	UserProvidedChainConfig,
	FillerConfig as FillerServiceConfig,
	CoinGeckoConfig,
} from "../services/FillerConfigService.js"
import { getLogger } from "../services/Logger.js"
const logger = getLogger("cli")

// ASCII art header
const ASCII_HEADER = `
██╗███╗   ██╗████████╗███████╗███╗   ██╗████████╗ ██████╗  █████╗ ████████╗███████╗██╗    ██╗ █████╗ ██╗   ██╗
██║████╗  ██║╚══██╔══╝██╔════╝████╗  ██║╚══██╔══╝██╔════╝ ██╔══██╗╚══██╔══╝██╔════╝██║    ██║██╔══██╗╚██╗ ██╔╝
██║██╔██╗ ██║   ██║   █████╗  ██╔██╗ ██║   ██║   ██║  ███╗███████║   ██║   █████╗  ██║ █╗ ██║███████║ ╚████╔╝
██║██║╚██╗██║   ██║   ██╔══╝  ██║╚██╗██║   ██║   ██║   ██║██╔══██║   ██║   ██╔══╝  ██║███╗██║██╔══██║  ╚██╔╝
██║██║ ╚████║   ██║   ███████╗██║ ╚████║   ██║   ╚██████╔╝██║  ██║   ██║   ███████╗╚███╔███╔╝██║  ██║   ██║
╚═╝╚═╝  ╚═══╝   ╚═╝   ╚══════╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚══════╝ ╚══╝╚══╝ ╚═╝  ╚═╝   ╚═╝

`

// Get package.json path
const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)
const packageJsonPath = resolve(__dirname, "../../package.json")
const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"))

interface StrategyConfig {
	type: "basic" | "stable-swap"
	privateKey: string
}

interface ChainConfirmationPolicy {
	minAmount: string
	maxAmount: string
	minConfirmations: number
	maxConfirmations: number
}

interface PendingQueueConfig {
	maxRechecks: number
	recheckDelayMs: number
}

interface FillerTomlConfig {
	filler: {
		privateKey: string
		maxConcurrentOrders: number
		pendingQueue: PendingQueueConfig
		coingecko?: CoinGeckoConfig
	}
	strategies: StrategyConfig[]
	chains: UserProvidedChainConfig[]
	confirmationPolicies: Record<string, ChainConfirmationPolicy>
}

const program = new Command()

program.name("filler").description("Hyperbridge IntentGateway Filler").version(packageJson.version)

program
	.command("run")
	.description("Run the intent filler with the specified configuration")
	.requiredOption("-c, --config <path>", "Path to TOML configuration file")
	.action(async (options: { config: string }) => {
		try {
			// Display ASCII art header
			process.stdout.write(ASCII_HEADER)

			logger.info("Starting Hyperbridge IntentGateway Filler...")

			const configPath = resolve(process.cwd(), options.config)
			logger.info({ configPath }, "Loading configuration")

			const tomlContent = readFileSync(configPath, "utf-8")
			const config = parse(tomlContent) as FillerTomlConfig

			validateConfig(config)

			logger.info("Initializing services...")

			const fillerChainConfigs: UserProvidedChainConfig[] = config.chains.map((chain) => ({
				chainId: chain.chainId,
				rpcUrl: chain.rpcUrl,
			}))

			const fillerConfigForService: FillerServiceConfig | undefined = config.filler.coingecko
				? {
						privateKey: config.filler.privateKey,
						maxConcurrentOrders: config.filler.maxConcurrentOrders,
						coingecko: config.filler.coingecko,
					}
				: undefined

			const configService = new FillerConfigService(fillerChainConfigs, fillerConfigForService)

			const chainConfigs: ChainConfig[] = config.chains.map((chain) => {
				// Get the chain name from chain ID for SDK compatibility
				const chainName = `EVM-${chain.chainId}`
				return configService.getChainConfig(chainName)
			})

			// Initialize confirmation policy
			const confirmationPolicy = new ConfirmationPolicy(config.confirmationPolicies)

			// Create filler configuration
			const fillerConfig: FillerConfig = {
				confirmationPolicy: {
					getConfirmationBlocks: (chainId: number, amount: bigint) =>
						confirmationPolicy.getConfirmationBlocks(chainId, amount),
				},
				maxConcurrentOrders: config.filler.maxConcurrentOrders,
				pendingQueueConfig: config.filler.pendingQueue,
			}

			// Initialize strategies
			logger.info("Initializing strategies...")
			const strategies = config.strategies.map((strategyConfig) => {
				switch (strategyConfig.type) {
					case "basic":
						return new BasicFiller(strategyConfig.privateKey as HexString, configService)
					case "stable-swap":
						return new StableSwapFiller(strategyConfig.privateKey as HexString, configService)
					default:
						throw new Error(`Unknown strategy type: ${strategyConfig.type}`)
				}
			})

			// Initialize and start the intent filler
			logger.info("Starting intent filler...")
			const intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig, configService)
			// Start the filler
			intentFiller.start()

			logger.info(
				{
					chains: config.chains.map((c) => c.chainId),
					strategies: config.strategies.map((s) => s.type),
					maxConcurrentOrders: config.filler.maxConcurrentOrders,
				},
				"Intent filler is running",
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
			logger.error({ err: error }, "Failed to start filler")
			process.exit(1)
		}
	})

function validateConfig(config: FillerTomlConfig): void {
	// Validate required fields
	if (!config.filler?.privateKey) {
		throw new Error("Filler private key is required")
	}

	if (!config.strategies || config.strategies.length === 0) {
		throw new Error("At least one strategy must be configured")
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
		if (!strategy.type || !strategy.privateKey) {
			throw new Error("Strategy type and private key are required")
		}

		if (!["basic", "stable-swap"].includes(strategy.type)) {
			throw new Error(`Invalid strategy type: ${strategy.type}`)
		}
	}

	// Validate confirmation policies
	for (const [chainId, policy] of Object.entries(config.confirmationPolicies)) {
		if (!policy.minAmount || !policy.maxAmount) {
			throw new Error(`Confirmation policy for chain ${chainId} must have minAmount and maxAmount`)
		}

		if (policy.minConfirmations === undefined || policy.maxConfirmations === undefined) {
			throw new Error(`Confirmation policy for chain ${chainId} must have minConfirmations and maxConfirmations`)
		}

		if (policy.minConfirmations > policy.maxConfirmations) {
			throw new Error(`Invalid confirmation range for chain ${chainId}`)
		}
	}
}

// Parse command line arguments
program.parse(process.argv)

// Show help if no command is provided
if (!process.argv.slice(2).length) {
	program.outputHelp()
}
