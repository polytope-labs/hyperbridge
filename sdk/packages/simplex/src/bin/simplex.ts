#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { existsSync } from "fs"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { formatChainKey, parseChainKey } from "@/config/interpolated-curve"
import { normalizeSymbol, type AssetDefinition } from "@/config/asset-registry"
import type { PairConfig } from "@/config/pairs"
import { adminStrategyFor, tradingPairFrom, type FillerRuntime } from "@/core/boot"
import { Simplex } from "@/simplex"
import { discoverConfigPath, DEFAULT_CONFIG_FILENAME } from "@/cli/discover-config"
import { openBrowser } from "@/cli/open-browser"
import { addLogSink, getLogger, configureLogger, type LogLevel, type LogSink } from "@/services/Logger"
import prettyStream from "pino-pretty"
import {
	FillerConfigService,
	type ResolvedChainConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { PaymasterKeeperService } from "@/services/PaymasterKeeperService"
import { initializeSignerFromToml, type SigningAccount } from "@/services/wallet"
import { UiServer, type OperatorContext } from "@/services/server/UiServer"
import { deriveSubstrateKeyPair } from "@/services/substrate-key"

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

const DEFAULT_UI_PORT = 8686

/**
 * Sends the library's log records to this process's stdout, pretty-printed.
 *
 * The library logs nowhere until something registers a sink — correct for an
 * embedded filler, and the CLI *is* the application, so opting in here is the
 * whole point. Formatting happens in-process rather than through pino's
 * worker-thread transport, which keeps startup off the thread-stream path.
 */
function consoleSink(): LogSink {
	// `destination`, not `.pipe(process.stdout)`: piped, pino-pretty echoes each
	// record's raw NDJSON alongside the formatted line, so every log appears twice.
	return prettyStream({
		colorize: true,
		singleLine: true,
		ignore: "pid,hostname,moduleTag",
		messageFormat: "{moduleTag}: {msg}",
		destination: process.stdout,
	})
}

// The process-wide context covers everything outside a filler: the setup wizard,
// config validation, the keeper command. A running filler logs to its own
// context and gets a stream of its own below — one transform per writer, because
// two pino instances writing into a single pino-pretty transform interleave
// their chunks and it echoes the unparseable remainder as raw NDJSON.
addLogSink(consoleSink())

/**
 * Opens the CLI's persistent store.
 *
 * Imported lazily and by name so `--help`, `init` and a config error all still
 * work when the optional `better-sqlite3` native module failed to build — and so
 * that failure reports what to do instead of a module-resolution stack trace.
 * There is no memory fallback: the CLI submits bids, and bid records are how
 * locked deposits are found again.
 */
async function openDataStore(dataDir?: string) {
	try {
		const { SqliteDataStore } = await import("@/data/sqlite")
		return new SqliteDataStore(resolveDataDir(dataDir))
	} catch (err) {
		throw new Error(
			"Could not load better-sqlite3, which simplex needs to record the bids it submits. " +
				"Reinstall so its native module builds (it is an optional dependency, so a failed " +
				`build is not fatal to installation): ${err instanceof Error ? err.message : String(err)}`,
		)
	}
}

/**
 * Where the CLI keeps its SQLite databases and runtime state. Matches the
 * directory the pre-interface storage services defaulted to, so an existing
 * install keeps its bid history (and therefore its reclaimable deposits).
 */
function resolveDataDir(dataDir?: string): string {
	return dataDir || resolve(process.cwd(), ".simplex-data")
}

/** Parses a `[host:]port` spec; returns undefined when the port is invalid. */
function parseBind(spec: string, defaultHost: string): { host: string; port: number } | undefined {
	const [host, portStr] = spec.includes(":") ? (spec.split(":").slice(-2) as [string, string]) : [defaultHost, spec]
	const port = parseInt(portStr, 10)
	if (isNaN(port) || port < 1 || port > 65535) return undefined
	return { host, port }
}

/** The built SPA lives in dist/ui; resolve it for both the bundled bin and tsx dev runs. */
function resolveUiDistDir(): string | undefined {
	const candidates = [resolve(__dirname, "../ui"), resolve(__dirname, "../../dist/ui")]
	return candidates.find((dir) => existsSync(dir))
}

async function operatorContextFrom(runtime: FillerRuntime): Promise<OperatorContext> {
	const substrateAddress = await deriveSubstrateKeyPair(runtime.config.simplex.substratePrivateKey)
		.then((pair) => pair.address)
		.catch(() => undefined)
	// One array instance for /api/status, mutated by the market capabilities below.
	const strategyTypes = (runtime.config.pairs ?? []).map((p) => `${p.token0}/${p.token1}`)
	const { engine, tradingPairs, adminStrategies, assetRegistry, balanceTokens } = runtime
	const marketCapabilities =
		engine && tradingPairs
			? {
					addPair: (pair: PairConfig, assets: Record<string, AssetDefinition> | undefined, pairIndex: number) => {
						// Register new custom tokens on the live registry first so the
						// engine and balance tracking can resolve the pair's symbols.
						if (assets && Object.keys(assets).length > 0) assetRegistry.addAssets(assets)
						const tradingPair = tradingPairFrom(pair)
						// Pushes into the engine's live array — the same instance as
						// `tradingPairs`, so config.pairs indexes stay aligned.
						engine.addPair(tradingPair)
						strategyTypes.push(`${pair.token0}/${pair.token1}`)
						// Track the exotic side's balances from the next refresh.
						if (normalizeSymbol(pair.token0) !== normalizeSymbol(pair.token1) && pair.referenceOnly !== true) {
							for (const chain of runtime.resolvedChains) {
								const chainName = formatChainKey(chain.chainId)
								const address = assetRegistry.getAddress(pair.token1, chainName)
								if (!address) continue
								const list = (balanceTokens[chainName] ??= [])
								if (!list.includes(address)) list.push(address)
							}
						}
						const index = adminStrategies.reduce((max, s) => Math.max(max, s.index), -1) + 1
						const adminStrategy = adminStrategyFor(tradingPair, pairIndex, index)
						if (adminStrategy) adminStrategies.push(adminStrategy)
						return adminStrategy
					},
					removePair: (index: number) => {
						const position = adminStrategies.findIndex((s) => s.index === index)
						if (position < 0) throw new Error(`Unknown strategy ${index}`)
						const { pairIndex } = adminStrategies[position]
						// Splices the engine's live array (same instance as tradingPairs).
						engine.removePair(tradingPairs[pairIndex])
						adminStrategies.splice(position, 1)
						// config.pairs loses the entry too, so later pairs shift down.
						for (const s of adminStrategies) {
							if (s.pairIndex > pairIndex) s.pairIndex -= 1
						}
						strategyTypes.splice(pairIndex, 1)
					},
				}
			: {}
	return {
		...marketCapabilities,
		addresses: { evm: runtime.fillerAddress, substrate: substrateAddress },
		strategies: runtime.adminStrategies,
		filler: runtime.intentFiller,
		balances: runtime.balanceProvider,
		haltControls: runtime.haltControls,
		config: runtime.config,
		stop: async () => {
			await runtime.shutdown("UI")
			process.exit(0)
		},
		activity: runtime.activity,
		bids: runtime.data.bids,
		setPaused: (paused) => runtime.data.state.set({ paused }),
		setLogLevel: (level) => runtime.loggers.setLevel(level),
		vault: runtime.vaultVenue
			? {
					sweepNow: () => runtime.vaultVenue!.sweepExcessToVault(),
					redeemAll: () => runtime.vaultVenue!.redeemAll(),
					reconfigure: (vaults, sweepIntervalMs) => {
						const vaultsByChain: Record<string, { vault: `0x${string}`; threshold?: string; minBalance?: string; redeemOnShutdown?: boolean }[]> = {}
						for (const row of vaults) {
							if (!vaultsByChain[row.chain]) vaultsByChain[row.chain] = []
							vaultsByChain[row.chain].push({
								vault: row.vault,
								threshold: row.threshold,
								minBalance: row.minBalance,
								redeemOnShutdown: row.redeemOnShutdown,
							})
						}
						return runtime.vaultVenue!.reconfigure({ vaultsByChain, sweepIntervalMs })
					},
				}
			: undefined,
		rebalancing: runtime.rebalancingService
			? { checkTriggers: () => runtime.rebalancingService!.checkRebalanceTriggers() }
			: undefined,
		applyAllowlist: (allowlist) => runtime.configService.setAllowlist(allowlist),
		applyRebalancing: (rebalancing) => runtime.configService.setRebalancing(rebalancing),
		vaultPreflight: (vaults) => runtime.vaultPreflight(vaults),
		rpcUrlFor: (chain) => {
			const chainId = parseChainKey(chain)
			return runtime.resolvedChains.find((c) => c.chainId === chainId)?.rpcUrls[0]
		},
		send: (params) => runtime.tokenSender.send(params),
		version: packageJson.version,
		startedAt: runtime.startedAt,
		configPath: runtime.configPath,
		chains: runtime.resolvedChains.map((c) => c.chainId),
		strategyTypes,
	}
}

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
		try {
			const { runInit } = await import("@/cli/init")
			await runInit(options)
		} catch (error) {
			console.error(error instanceof Error ? error.message : String(error))
			process.exit(1)
		}
	})

program
	.command("run", { isDefault: true })
	.description("Run the intent filler; without a config it starts the browser setup wizard")
	.option("-c, --config <path>", `Path to TOML configuration file (default: ./${DEFAULT_CONFIG_FILENAME})`)
	.option("-d, --data-dir <path>", "Directory for persistent data storage (bids database, etc.)")
	.option("--watch-only", "Watch-only mode: monitor orders without executing fills", false)
	.option(
		"--ui [<[host:]port>]",
		`Bind address for the local web UI (status, pause/resume, price curves). Unauthenticated; default ${`127.0.0.1:${DEFAULT_UI_PORT}`}`,
	)
	.option("--no-ui", "Disable the local web UI")
	.action(async (options: { config?: string; dataDir?: string; watchOnly?: boolean; ui?: string | boolean }) => {
		try {
			// Display ASCII art header
			process.stdout.write(ASCII_HEADER)

			const logger = getLogger("cli")


			const uiEnabled = options.ui !== false
			let uiBind = { host: "127.0.0.1", port: DEFAULT_UI_PORT }
			if (typeof options.ui === "string") {
				const parsed = parseBind(options.ui, "127.0.0.1")
				if (!parsed) {
					logger.warn({ bind: options.ui }, `Invalid UI address, using default 127.0.0.1:${DEFAULT_UI_PORT}`)
				} else {
					uiBind = parsed
				}
			}

			let simplex: Simplex | undefined
			let runtime: FillerRuntime | undefined
			let uiServer: UiServer | undefined

			/** Starts the filler and everything the CLI layers on top of it. */
			const startFiller = async (config: FillerTomlConfig, path: string) => {
				simplex = await Simplex.start({
					config,
					configPath: path,
					logger: consoleSink(),
					data: await openDataStore(options.dataDir),
					watchOnly: options.watchOnly,
				})
				runtime = simplex.internals
				return simplex
			}

			// Registered once, up front: during init mode there is no runtime yet
			// (ctrl-c just closes the server); once save-and-start assigns `runtime`,
			// the same handler drains the filler. Nothing is re-registered on transition.
			const shutdown = async (signal: string) => {
				uiServer?.stop()
				if (simplex) await simplex.stop()
				process.exit(0)
			}
			process.on("SIGINT", () => void shutdown("SIGINT"))
			process.on("SIGTERM", () => void shutdown("SIGTERM"))

			const configPath = options.config ? resolve(process.cwd(), options.config) : discoverConfigPath()

			if (configPath) {
				const tomlContent = readFileSync(configPath, "utf-8")
				const config = parse(tomlContent) as FillerTomlConfig
				validateConfig(config, options.watchOnly === true)

				await startFiller(config, configPath)

				// Local web UI (status, pause/resume, inflight price curve updates).
				// On by default at 127.0.0.1; disable with --no-ui.
				if (uiEnabled) {
					uiServer = new UiServer({
						mode: "operator",
						uiDistDir: resolveUiDistDir(),
						operator: await operatorContextFrom(runtime!),
					})
					try {
						await uiServer.start(uiBind.port, uiBind.host)
					} catch (err) {
						// The filler is the primary workload; a bind failure (e.g. port in use)
						// costs the UI, not the process.
						logger.error({ err, bind: `${uiBind.host}:${uiBind.port}` }, "UI server failed to start")
						uiServer = undefined
					}
				}
				return
			}

			// No config anywhere: hand over to the browser setup wizard.
			if (!uiEnabled) {
				console.error(
					`No config found (looked for ./${DEFAULT_CONFIG_FILENAME}` +
						`${process.env.SIMPLEX_HOME ? " and $SIMPLEX_HOME/config.toml" : ""}). ` +
						"Run `simplex init` or pass -c <path>.",
				)
				process.exit(1)
			}

			const outputPath = resolve(process.cwd(), DEFAULT_CONFIG_FILENAME)
			const server = new UiServer({
				mode: "init",
				uiDistDir: resolveUiDistDir(),
				setup: {
					configPath: outputPath,
					onSaveAndStart: async (config, _toml, path) => {
						await startFiller(config, path)
						server.enterOperatorMode(await operatorContextFrom(runtime!))
					},
				},
			})
			uiServer = server

			let boundPort: number
			try {
				boundPort = await server.start(uiBind.port, uiBind.host)
			} catch (err) {
				logger.warn({ err, bind: `${uiBind.host}:${uiBind.port}` }, "Preferred UI port unavailable, retrying")
				boundPort = await server.start(0, uiBind.host)
			}
			// A wildcard bind is not an address to browse to — Safari refuses 0.0.0.0
			// outright. The operator reaches it on localhost, via whatever they published.
			const browsableHost = uiBind.host === "0.0.0.0" || uiBind.host === "::" ? "localhost" : uiBind.host
			const url = `http://${browsableHost}:${boundPort}/`
			console.log(`\n  No config found — starting the setup wizard.\n\n  ${url}\n`)
			openBrowser(url)
			// The server keeps the event loop alive until the wizard completes.
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

// Parse command line arguments
program.parse(process.argv)
