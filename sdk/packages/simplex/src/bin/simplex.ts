#!/usr/bin/env -S node --enable-source-maps --disable-warning=ExperimentalWarning

// First import, deliberately: silences @polkadot/* init noise, which fires
// while the imports below are still evaluating.
import "./quiet"
import { patchRuntimeState } from "@/data/state"
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname, join } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { existsSync } from "fs"
import { validateConfig, type FillerConfigFile } from "@/config/filler-toml"
import { parseChainKey } from "@/config/interpolated-curve"
import type { AssetDefinition } from "@/config/asset-registry"
import type { PairConfig } from "@/config/pairs"
import type { FillerRuntime } from "@/core/boot"
import { Simplex } from "@/simplex"
import { SqliteDataStore } from "@/data/sqlite"
import { discoverConfigPath, DEFAULT_CONFIG_FILENAME } from "@/cli/discover-config"
import { addRunOptions, DEFAULT_UI_PORT, type RunOptions } from "@/cli/run-options"
import { openBrowser } from "@/cli/open-browser"
import { logFormatFromArgv, type LogFormat } from "@/cli/log-format"
import { addLogSink, getLogger, configureLogger, type LogLevel, type LogSink } from "@/services/Logger"
import prettyStream from "pino-pretty"
import {
	FillerConfigService,
	type ResolvedChainConfig,
	resolveChainConfigs,
} from "@/services/FillerConfigService"
import { ChainClientManager } from "@/services/ChainClientManager"
import { PaymasterKeeperService } from "@/services/PaymasterKeeperService"
import { signerFromToml, type Signer } from "@/services/wallet"
import { UiServer, type OperatorContext } from "@/services/server/UiServer"
import { LogStore } from "@/services/server/LogStore"
import { TunnelService } from "@/services/tunnel/TunnelService"
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

/**
 * Sends the library's log records to this process's stdout.
 *
 * The library logs nowhere until something registers a sink — correct for an
 * embedded filler, and the CLI *is* the application, so opting in here is the
 * whole point. Formatting happens in-process rather than through pino's
 * worker-thread transport, which keeps startup off the thread-stream path.
 */
function consoleSink(format: LogFormat): LogSink {
	// Nothing to format: pino hands the destination one finished NDJSON record per
	// write, newline included, so json mode is the *absence* of a transform rather
	// than a different one — no pino-pretty, and therefore none of its parsing.
	if (format === "json") return process.stdout
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

// Read from raw argv, not the parsed options: the sink below is registered while
// this module is still evaluating, and commander has not run yet.
const logFormat = logFormatFromArgv(process.argv)

// pino-pretty's pump() chain attaches an `error` listener to whatever destination
// it is handed; a bare `process.stdout` has none. Without one, the first write
// after something closes the read end — the desktop app quitting while the filler
// it spawned keeps running, or a plain `simplex ... | head` — raises an unhandled
// `error` event and kills the process, which is the opposite of what json mode is
// for. Swallowing matches the pretty path, where the same error tears the
// transform chain down and logging simply goes quiet; LoggerContext already treats
// a broken sink as the host's problem rather than a reason to stop filling.
// Registered here, once, because consoleSink() is called per writer.
if (logFormat === "json") process.stdout.on("error", () => {})

// The process-wide context covers everything outside a filler: the setup wizard,
// config validation, the keeper command. A running filler logs to its own
// context and gets a sink of its own below — on the pretty path that means one
// transform per writer, because two pino instances writing into a single
// pino-pretty transform interleave their chunks and it echoes the unparseable
// remainder as raw NDJSON. Sharing stdout in json mode is safe for the same
// reason it is a hazard here: there is no transform in between reassembling
// anything, and pino writes each record whole.
addLogSink(consoleSink(logFormat))

/**
 * What the dashboard's Logs page reads. One store for the whole process, fed
 * from both contexts — the UI server and the config layer log here, the filler
 * logs to its own — so the page shows the operator one feed rather than making
 * them know which half of the binary emitted a line.
 *
 * Registered at module load, before anything has had a chance to log, so the
 * history really does start at launch. Unlike the console sink it is registered
 * once rather than per writer: it stores parsed records, so interleaving is a
 * non-issue. Its file is opened later, once `--data-dir` has been parsed.
 */
const logStore = new LogStore()
addLogSink(logStore.sink())

/** Sends one record to several destinations — the console and the log store, in practice. */
function fanout(sinks: LogSink[]): LogSink {
	return {
		write(line: string) {
			for (const sink of sinks) {
				try {
					sink.write(line)
				} catch {
					// One broken destination must not cost the others their record.
				}
			}
		},
	}
}

/**
 * Opens the CLI's persistent store.
 *
 * There is no memory fallback: the CLI submits bids, and bid records are how
 * locked deposits are found again. This used to be a lazy import wrapped in a
 * try/catch that translated a failed native build into readable advice; on
 * `node:sqlite` there is no native build to fail, so a throw here is a real
 * problem with the data directory and should surface as itself.
 */
function openDataStore(dataDir?: string) {
	return new SqliteDataStore(resolveDataDir(dataDir))
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
	const port = Number.parseInt(portStr, 10)
	if (isNaN(port) || port < 1 || port > 65535) return undefined
	return { host, port }
}

/** The built SPA lives in dist/ui; resolve it for both the bundled bin and tsx dev runs. */
function resolveUiDistDir(): string | undefined {
	const candidates = [resolve(__dirname, "../ui"), resolve(__dirname, "../../dist/ui")]
	return candidates.find((dir) => existsSync(dir))
}

async function operatorContextFrom(
	simplex: Simplex,
	stopAll: () => Promise<never>,
	tunnel?: TunnelService,
): Promise<OperatorContext> {
	const runtime = simplex.internals
	const substrateAddress = await deriveSubstrateKeyPair(runtime.config.simplex.substratePrivateKey)
		.then((pair) => pair.address)
		.catch(() => undefined)
	// One array instance for /api/status, mutated by the market capabilities below.
	const strategyTypes = (runtime.config.pairs ?? []).map((p) => `${p.token0}/${p.token1}`)
	const { engine, tradingPairs, adminStrategies } = runtime
	const marketCapabilities =
		engine && tradingPairs
			? {
					// Thin wrappers over PairController — the same code path the library's
					// `simplex.pairs` exposes — so the CLI cannot drift from it again. The
					// controller validates, registers assets, updates the engine, config
					// and admin strategies, and persists; only the dashboard's
					// strategy-index addressing and its status array are adapted here.
					addPair: async (
						pair: PairConfig,
						assets: Record<string, AssetDefinition> | undefined,
						_pairIndex: number,
					) => {
						const view = await simplex.pairs.add(pair, { assets })
						strategyTypes.push(`${pair.token0}/${pair.token1}`)
						return adminStrategies.find((s) => s.pairIndex === view.index) ?? null
					},
					removePair: async (index: number) => {
						const strategy = adminStrategies.find((s) => s.index === index)
						if (!strategy) throw new Error(`Unknown strategy ${index}`)
						const { pairIndex } = strategy
						await simplex.pairs.remove(pairIndex)
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
		// The CLI's one teardown — the same path the signal handlers take, so the
		// dashboard's Stop cannot skip steps (the store close, most recently) that
		// ctrl-C performs.
		stop: () => stopAll(),
		activity: runtime.activity,
		bids: runtime.data.bids,
		setPaused: (paused) => patchRuntimeState(runtime.data.state, { paused }),
		// Both contexts, not just the filler's: the dashboard shows one merged feed
		// and reports one level for it, so leaving the process-wide context (the UI
		// server, the config layer) pinned at its default would make that a lie in
		// both directions — stray info records below a raised floor, and modules
		// that never follow a lowered one.
		setLogLevel: (level) => {
			runtime.loggers.setLevel(level)
			configureLogger(level)
		},
		logs: logStore,
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
		tunnel,
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

addRunOptions(program.command("run", { isDefault: true }))
	.description("Run the intent filler; without a config it starts the browser setup wizard")
	.action(async (options: RunOptions) => {
		try {
			// Decoration for a terminal. In json mode stdout is a log file someone
			// else is parsing, and a banner is not a record.
			if (logFormat === "pretty") process.stdout.write(ASCII_HEADER)

			// Now that --data-dir is parsed, give the store somewhere to keep this
			// launch's history. Records logged before this point are already in the
			// in-memory tail and get written out with the rest.
			logStore.openLaunchFile(join(resolveDataDir(options.dataDir), "logs"))

			const logger = getLogger("cli")


			const uiEnabled = options.ui !== false
			const uiSocket = options.uiSocket
			// An empty value would be falsy at every use below, so the UI would quietly
			// come up on the TCP port the operator was trying to avoid.
			if (uiSocket !== undefined && uiSocket.trim() === "") {
				throw new Error("--ui-socket needs a path; it was given an empty value")
			}
			// Two listen addresses, or an address and an off switch, are a mistake worth
			// naming: silently picking one leaves the operator watching a port nothing
			// is on. Only an explicit `--ui <addr>` conflicts — a bare `--ui` just turns
			// the UI on and names no address, so it pairs with a socket fine.
			if (uiSocket && !uiEnabled) {
				throw new Error("--ui-socket and --no-ui contradict each other: one serves the UI, the other turns it off")
			}
			if (uiSocket && typeof options.ui === "string") {
				throw new Error(
					`--ui and --ui-socket name two different listen addresses; pass one (got --ui ${options.ui} and --ui-socket ${uiSocket})`,
				)
			}
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
			let dataStore: SqliteDataStore | undefined
			let runtime: FillerRuntime | undefined
			let uiServer: UiServer | undefined
			let tunnel: TunnelService | undefined
			// The port the UI actually bound; the setup wizard may fall back to an ephemeral one.
			let uiBoundPort = uiBind.port

			/** Starts the filler and everything the CLI layers on top of it. */
			const startFiller = async (config: FillerConfigFile, path: string) => {
				dataStore = openDataStore(options.dataDir)
				// The TOML block is the binary's way of naming a signer; the library
				// takes the resolved instance, so the file format stops here.
				const signer = await signerFromToml(config.simplex?.signer)
				simplex = await Simplex.start({
					config,
					signer,
					configPath: path,
					// Handed in rather than added after start() resolves: the filler
					// builds its context and runs the whole of boot inside start(), so
					// a sink attached afterwards misses every boot record.
					logger: fanout([consoleSink(logFormat), logStore.sink()]),
					data: dataStore,
					watchOnly: options.watchOnly,
				})
				runtime = simplex.internals
				return simplex
			}

			/**
			 * Builds the tunnel, without connecting it. Remote access rides on the
			 * operator-mode UI only, so the caller connects it once the UI has
			 * actually bound: `--no-ui` or a lost race for the port would otherwise
			 * point the tunnel at whatever else answers on that port.
			 *
			 * A failure here costs remote access, never filling — the filler is the
			 * workload, and the key store is the only thing that can throw.
			 */
			const createTunnel = (config: FillerConfigFile): TunnelService | undefined => {
				// A wildcard bind is reached on loopback; every specific address is kept
				// as given. Collapsing loopback addresses too sent the tunnel to
				// 127.0.0.1 when the UI was listening on, say, 127.0.0.2 — the channel
				// opened and the connection behind it was refused.
				const uiHost = uiBind.host === "0.0.0.0" || uiBind.host === "::" ? "127.0.0.1" : uiBind.host
				try {
					return new TunnelService({
						dataDir: resolveDataDir(options.dataDir),
						config: config.simplex.tunnel,
						uiTarget: () => ({ host: uiHost, port: uiBoundPort }),
						// Late-bound: the UI server is built after this, and no channel
						// can arrive before the tunnel is started, which is later still.
						deliver: (socket) => uiServer?.accept(socket) ?? false,
					})
				} catch (err) {
					logger.error({ err }, "Remote access unavailable; filling continues without it")
					return undefined
				}
			}

			/** Connects the tunnel if there is one. Same rule: never fatal. */
			const startTunnel = (): void => {
				try {
					tunnel?.start()
				} catch (err) {
					logger.error({ err }, "Remote access failed to start; filling continues without it")
					tunnel = undefined
				}
			}

			// Registered once, up front: during init mode there is no runtime yet
			// (ctrl-c just closes the server); once save-and-start assigns `runtime`,
			// the same handler drains the filler. Nothing is re-registered on transition.
			const shutdown = async (signal: string): Promise<never> => {
				uiServer?.stop()
				await tunnel?.stop()
				if (simplex) await simplex.stop()
				// Ours to close: the library no longer closes a caller-supplied store.
				await dataStore?.close?.()
				logStore.close()
				process.exit(0)
			}
			process.on("SIGINT", () => void shutdown("SIGINT"))
			process.on("SIGTERM", () => void shutdown("SIGTERM"))

			const configPath = options.config ? resolve(process.cwd(), options.config) : discoverConfigPath()

			if (configPath) {
				const tomlContent = readFileSync(configPath, "utf-8")
				const config = parse(tomlContent) as FillerConfigFile
				validateConfig(config, options.watchOnly === true)
				// validateConfig no longer owns this rule — the signer is an argument to
				// the library, not a config field — but the binary's users configure it
				// in the file, so name the file here rather than let boot say it later.
				if (!config.simplex?.signer && options.watchOnly !== true && config.simplex?.watchOnly !== true) {
					throw new Error("Signer configuration is required via [simplex.signer]")
				}

				await startFiller(config, configPath)

				// Local web UI (status, pause/resume, inflight price curve updates).
				// On by default at 127.0.0.1; disable with --no-ui.
				// No UI, no tunnel: the tunnel exists to carry devices to this dashboard,
				// and with nothing bound it would forward to whatever else holds the port.
				if (uiEnabled) {
					tunnel = createTunnel(config)
					uiServer = new UiServer({
						mode: "operator",
						uiDistDir: resolveUiDistDir(),
						operator: await operatorContextFrom(simplex!, () => shutdown("UI"), tunnel),
					})
					try {
						// A socket has no port, so `uiBoundPort` keeps its default — which is
						// only the address paired devices dial through the tunnel's forward.
						// Nothing listens there in socket mode and nothing needs to: channels
						// are injected via `deliver` above, never connected to.
						if (uiSocket) await uiServer.start({ socketPath: uiSocket })
						else uiBoundPort = await uiServer.start(uiBind.port, uiBind.host)
						startTunnel()
					} catch (err) {
						// The filler is the primary workload; a bind failure (e.g. port in use)
						// costs the UI, not the process.
						logger.error({ err, bind: uiSocket ?? `${uiBind.host}:${uiBind.port}` }, "UI server failed to start")
						uiServer = undefined
						await tunnel?.stop()
						tunnel = undefined
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
						// The wizard's own server is already bound, so the tunnel has a UI
						// to point at the moment it comes up.
						tunnel = createTunnel(config)
						server.enterOperatorMode(await operatorContextFrom(simplex!, () => shutdown("UI"), tunnel))
						startTunnel()
					},
				},
			})
			uiServer = server

			if (uiSocket) {
				// No URL to print and no browser that could open a socket: an embedding
				// application renders the wizard itself over this socket. A bind failure
				// here is fatal — there is no other way in.
				await server.start({ socketPath: uiSocket })
				// Same rule as the TCP announcement below: in json mode this is a record,
				// not prose, or it is the one non-JSON line in a stream someone is parsing.
				if (logFormat === "json") logger.info({ socket: uiSocket }, "No config found, starting the setup wizard")
				else console.log(`\n  No config found — the setup wizard is serving on ${uiSocket}\n`)
				// The server keeps the event loop alive until the wizard completes.
				return
			}

			let boundPort: number
			try {
				boundPort = await server.start(uiBind.port, uiBind.host)
			} catch (err) {
				logger.warn({ err, bind: `${uiBind.host}:${uiBind.port}` }, "Preferred UI port unavailable, retrying")
				boundPort = await server.start(0, uiBind.host)
			}
			uiBoundPort = boundPort
			// A wildcard bind is not an address to browse to — Safari refuses 0.0.0.0
			// outright. The operator reaches it on localhost, via whatever they published.
			const browsableHost = uiBind.host === "0.0.0.0" || uiBind.host === "::" ? "localhost" : uiBind.host
			const url = `http://${browsableHost}:${boundPort}/`
			// The URL is the one thing the operator must see. In json mode it is also
			// the one thing a supervisor must see, so it goes out as a record with a
			// `url` field rather than as prose no parser will look at.
			if (logFormat === "json") logger.info({ url }, "No config found, starting the setup wizard")
			else console.log(`\n  No config found — starting the setup wizard.\n\n  ${url}\n`)
			// --no-open: the caller renders the wizard itself (the desktop app puts it
			// in its own window), so a system browser opening alongside is wrong.
			if (options.open !== false) openBrowser(url)
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
			const config = parse(readFileSync(configPath, "utf-8")) as FillerConfigFile

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

			const configuredSigner = await signerFromToml(config.simplex.signer)
			const chainClientManager = new ChainClientManager(configService, configuredSigner)
			const runtimeSigner: Signer = chainClientManager.getSigner()

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
