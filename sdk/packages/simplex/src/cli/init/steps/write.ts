import { confirm, log, note, outro } from "@clack/prompts"
import { spawn } from "child_process"
import { chmodSync, existsSync, writeFileSync } from "fs"
import { resolve } from "path"
import { isDeepStrictEqual } from "util"
import { parse } from "toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { validateSignerConfig, type SignerConfig } from "@/services/wallet"
import { validateRpcUrls } from "@/services/FillerConfigService"
import { emitFillerToml } from "../emit-toml"
import { guard, maskSecret, askText } from "../prompt-utils"
import { FUNDING_CHECKLIST } from "../help-text"
import type { WizardState } from "../state"

export async function stepWrite(state: WizardState, outputPath: string): Promise<void> {
	const config = assembleConfig(state)

	// Hard gate: the wizard must never write a config `simplex run` would reject.
	validateSignerConfig(state.signer as SignerConfig)
	for (const chain of config.chains) validateRpcUrls(chain.rpcUrls)
	validateConfig(config)

	const emitted = emitFillerToml(config, { chainComments: chainComments(state) })
	// JSON-normalize both sides: strips undefined keys and the parser's null prototypes.
	const roundTripped = JSON.parse(JSON.stringify(parse(emitted)))
	if (!isDeepStrictEqual(JSON.parse(JSON.stringify(config)), roundTripped)) {
		throw new Error("Internal error: generated TOML does not round-trip; please report this")
	}

	showSummary(state, outputPath)

	let path = outputPath
	if (existsSync(path)) {
		const overwrite = guard(await confirm({ message: `${path} exists — overwrite it?`, initialValue: true }))
		if (!overwrite) {
			const other = await askText("Write to which path instead?", {
				initial: "filler-config.new.toml",
				required: "Path is required",
			})
			path = resolve(process.cwd(), other)
		}
	}

	writeFileSync(path, emitted, { mode: 0o600 })
	chmodSync(path, 0o600)
	log.success(`Config written to ${path} (permissions 600 — it contains secrets, don't commit it)`)

	note(FUNDING_CHECKLIST, "Before the filler can fill")

	const startNow = guard(await confirm({ message: "Start the filler now?", initialValue: true }))
	if (startNow) {
		outro("Starting the filler…")
		await startFiller(path)
		return
	}

	outro(`Start it any time with: simplex run -c ${path}`)
}

export function assembleConfig(state: WizardState): FillerTomlConfig {
	return {
		simplex: {
			signer: state.signer,
			maxConcurrentOrders: state.maxConcurrentOrders,
			queue: state.queue,
			...(state.logging !== undefined ? { logging: state.logging } : {}),
			substratePrivateKey: state.substratePrivateKey ?? "",
			hyperbridgeWsUrl: state.hyperbridgeWsUrl ?? "",
			...(state.gasFeeBump ? { gasFeeBump: state.gasFeeBump } : {}),
			...(state.overfillProtection ? { overfillProtection: state.overfillProtection } : {}),
		},
		strategies: state.strategies,
		chains: [
			...state.chains.map((chain) => ({ rpcUrls: chain.rpcUrls, bundlerUrl: chain.bundlerUrl ?? "" })),
			...state.passthroughChains,
		],
		...(state.rebalancing ? { rebalancing: state.rebalancing } : {}),
		...(state.vault ? { vault: state.vault } : {}),
		...(state.allowlist ? { allowlist: state.allowlist } : {}),
	}
}

function chainComments(state: WizardState): string[] {
	return [
		...state.chains.map((chain) => `${chain.meta.label} (chainId ${chain.meta.chainId})`),
		...state.passthroughChains.map(() => "Kept from the previous configuration"),
	]
}

function showSummary(state: WizardState, outputPath: string): void {
	const lines: string[] = []
	for (const chain of state.chains) {
		const bundlerHost = chain.bundlerUrl ? new URL(chain.bundlerUrl).hostname : "?"
		lines.push(
			`${chain.meta.label}: ${chain.rpcUrls.length} RPC${chain.rpcUrls.length > 1 ? "s (quorum)" : ""}, bundler ${bundlerHost}`,
		)
	}
	if (state.passthroughChains.length > 0) {
		lines.push(`+ ${state.passthroughChains.length} unmanaged chain(s) kept from the previous config`)
	}
	lines.push(`Signer: ${state.signer?.type}`)
	lines.push(`Substrate key: ${maskSecret(state.substratePrivateKey ?? "")}`)
	lines.push(`Hyperbridge: ${state.hyperbridgeWsUrl}`)
	lines.push(`Strategies: ${state.strategies.map((s) => s.type).join(", ")}`)
	lines.push(`Output: ${outputPath}`)
	note(lines.join("\n"), "Summary")
}

/**
 * Runs `simplex run -c <path>` as a child so `run` owns its SIGINT/SIGTERM
 * lifecycle. execArgv carries dev loaders (tsx) through.
 */
export function startFiller(configPath: string): Promise<never> {
	return new Promise(() => {
		const child = spawn(
			process.execPath,
			[...process.execArgv, process.argv[1], "run", "-c", configPath],
			{ stdio: "inherit" },
		)
		child.on("exit", (code) => process.exit(code ?? 0))
		child.on("error", (error) => {
			log.error(`Failed to start the filler: ${error.message}`)
			log.info(`Run it manually with: simplex run -c ${configPath}`)
			process.exit(1)
		})
	})
}
