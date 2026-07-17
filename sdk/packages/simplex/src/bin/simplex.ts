#!/usr/bin/env node
import { Command } from "commander"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { existsSync } from "fs"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { bootFiller, type FillerRuntime } from "@/core/bootstrap"
import { getLogger } from "@/services/Logger"
import { UiServer, type OperatorContext } from "@/services/server/UiServer"

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

function operatorContextFrom(runtime: FillerRuntime): OperatorContext {
	return {
		strategies: runtime.adminStrategies,
		filler: runtime.intentFiller,
		balances: runtime.balanceProvider,
		version: packageJson.version,
		startedAt: runtime.startedAt,
		configPath: runtime.configPath,
		chains: runtime.resolvedChains.map((c) => c.chainId),
		strategyTypes: runtime.config.strategies.map((s) => s.type),
		dataDir: runtime.dataDir,
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
	.option(
		"--ui [<[host:]port>]",
		`Bind address for the local web UI (status, pause/resume, price curves). Unauthenticated; default ${`127.0.0.1:${DEFAULT_UI_PORT}`}`,
	)
	.option("--no-ui", "Disable the local web UI")
	.action(async (options: { config: string; dataDir?: string; watchOnly?: boolean; port?: string; ui?: string | boolean }) => {
		try {
			// Display ASCII art header
			process.stdout.write(ASCII_HEADER)

			const configPath = resolve(process.cwd(), options.config)
			const tomlContent = readFileSync(configPath, "utf-8")
			const config = parse(tomlContent) as FillerTomlConfig

			validateConfig(config)

			let metricsBind: { host: string; port: number } | undefined
			if (options.port) {
				metricsBind = parseBind(options.port, "0.0.0.0")
				if (!metricsBind) {
					getLogger("cli").warn({ bind: options.port }, "Invalid metrics address, skipping")
				}
			}

			const runtime: FillerRuntime = await bootFiller(config, {
				configPath,
				dataDir: options.dataDir,
				watchOnlyOverride: options.watchOnly,
				metricsBind,
			})

			const logger = getLogger("cli")

			// Local web UI (status, pause/resume, inflight price curve updates).
			// On by default at 127.0.0.1; disable with --no-ui.
			let uiServer: UiServer | undefined
			if (options.ui !== false) {
				let uiBind = { host: "127.0.0.1", port: DEFAULT_UI_PORT }
				if (typeof options.ui === "string") {
					const parsed = parseBind(options.ui, "127.0.0.1")
					if (!parsed) {
						logger.warn({ bind: options.ui }, `Invalid UI address, using default 127.0.0.1:${DEFAULT_UI_PORT}`)
					} else {
						uiBind = parsed
					}
				}
				uiServer = new UiServer({
					mode: "operator",
					uiDistDir: resolveUiDistDir(),
					operator: operatorContextFrom(runtime),
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

			// Handle graceful shutdown
			const shutdown = async (signal: string) => {
				uiServer?.stop()
				await runtime.shutdown(signal)
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
