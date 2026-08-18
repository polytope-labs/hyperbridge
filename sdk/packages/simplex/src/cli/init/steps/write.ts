import { confirm, log, note, outro } from "@clack/prompts"
import { spawn } from "child_process"
import { existsSync } from "fs"
import { resolve } from "path"
import { isDeepStrictEqual } from "util"
import { parse } from "toml"
import { ChainConfigService } from "@hyperbridge/sdk"
import { assertConfirmationCoverage, validateConfig, type FillerConfigFile, type FillerTomlConfig } from "@/config/filler-toml"
import { AssetRegistry } from "@/config/asset-registry"
import { assertPairSymbolsResolve } from "@/config/pairs"
import { DEFAULT_CONFIRMATION_POLICIES, parseChainKey } from "@/config/interpolated-curve"
import { validateSignerConfig, type SignerConfig } from "@/services/wallet"
import { validateRpcUrls } from "@/services/FillerConfigService"
import { emitFillerToml, writeConfigFileAtomic } from "../emit-toml"
import { guard, maskSecret, askText } from "../prompt-utils"
import { FUNDING_CHECKLIST } from "../help-text"
import type { Prefill, WizardState } from "../state"

export async function stepWrite(state: WizardState, outputPath: string, prefill?: Prefill): Promise<void> {
	const config = assembleConfig(state)

	// Hard gate: the wizard must never write a config `simplex run` would reject.
	validateSignerConfig(state.signer as SignerConfig)
	for (const chain of config.chains) validateRpcUrls(chain.rpcUrls)
	validateConfig(config)
	// Boot-parity symbol resolution — a registry symbol not deployed on any
	// selected chain fails here, not first on `simplex run`. Skipped when
	// passthrough chains exist: their chain ids are unknown offline and a
	// symbol may resolve only there (boot still enforces).
	if (state.passthroughChains.length === 0 && config.pairs?.length) {
		assertPairSymbolsResolve(
			config.pairs,
			new AssetRegistry(new ChainConfigService({}), config.assets),
			state.chains.map((chain) => chain.meta.stateMachineId),
		)
	}
	// Boot-parity confirmation coverage for the selected chains; passthrough
	// chains keep the softer warnUncoveredPassthroughChains treatment.
	assertConfirmationCoverage(
		config.confirmationPolicies,
		state.chains.map((chain) => chain.meta.chainId),
	)

	const emitted = emitFillerToml(config, { chainComments: chainComments(state) })
	// JSON-normalize both sides: strips undefined keys and the parser's null prototypes.
	const roundTripped = JSON.parse(JSON.stringify(parse(emitted)))
	if (!isDeepStrictEqual(JSON.parse(JSON.stringify(config)), roundTripped)) {
		throw new Error("Internal error: generated TOML does not round-trip; please report this")
	}

	showSummary(state, outputPath)
	warnUncoveredPassthroughChains(state, prefill)

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

	writeConfigFileAtomic(path, emitted)
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

export function assembleConfig(state: WizardState): FillerConfigFile {
	// On an update run, overlay the wizard-managed fields onto a copy of the
	// existing config so sections the wizard never prompts for (binance, keeper,
	// targetGasUnits, entryPointAddress, watchOnly, …) survive verbatim.
	const base: Partial<FillerConfigFile> = state.prefillConfig
		? JSON.parse(JSON.stringify(state.prefillConfig))
		: {}

	// A legacy [[strategies]] array must never survive into the output — the
	// pair engine rejects it at startup (the wizard migrates it to prefills).
	delete (base as Record<string, unknown>).strategies

	// Empty-but-present collections don't survive the TOML emit (there is no
	// section to write), so they must not survive assembly either — the
	// round-trip gate would trip on them.
	if (base.assets && Object.keys(base.assets).length === 0) delete base.assets
	const allowlist = state.allowlist ? { ...state.allowlist } : undefined
	if (allowlist?.bySource && Object.keys(allowlist.bySource).length === 0) delete allowlist.bySource
	const scrubbedAllowlist = allowlist && Object.keys(allowlist).length > 0 ? allowlist : undefined
	// [rebalancing] without base balances can neither trigger nor emit.
	const rebalancing =
		state.rebalancing?.baseBalances && Object.keys(state.rebalancing.baseBalances).length > 0
			? state.rebalancing
			: undefined

	// The pairs step owns [vault.uniswapV4] wholesale: the block it configured
	// this run replaces whatever the prefill had (or drops it when the operator
	// switched to curve pricing).
	const vault: NonNullable<FillerTomlConfig["vault"]> = { ...(state.vault ?? {}) }
	delete vault.uniswapV4
	if (vault.vaults && vault.vaults.length === 0) delete vault.vaults
	if (state.vaultUniswapV4) {
		vault.uniswapV4 = { ...state.vaultUniswapV4 }
		if (vault.uniswapV4.positions && vault.uniswapV4.positions.length === 0) delete vault.uniswapV4.positions
	}
	const hasVault = Boolean(vault.vaults?.length || vault.uniswapV4 || vault.sweepIntervalMs !== undefined)

	// Merge, prefill first: a custom token added this run must not drop the
	// existing [assets] entries.
	const assets = { ...(base.assets ?? {}), ...(state.assets ?? {}) }

	return {
		...base,
		simplex: {
			...base.simplex,
			// Dead knobs: parsed for compatibility, never read, and not emitted — carrying
			// them through would fail the round-trip gate below.
			queue: undefined,
			entryPointAddress: undefined,
			solverAccountContractAddress: undefined,
			signer: state.signer,
			maxConcurrentOrders: state.maxConcurrentOrders,
			...(state.logging !== undefined ? { logging: state.logging } : {}),
			substratePrivateKey: state.substratePrivateKey ?? "",
			hyperbridgeWsUrl: state.hyperbridgeWsUrl ?? "",
			...(state.gasFeeBump ? { gasFeeBump: state.gasFeeBump } : { gasFeeBump: undefined }),
			...(state.overfillProtection
				? { overfillProtection: state.overfillProtection }
				: { overfillProtection: undefined }),
		},
		pairs: state.pairs,
		assets: Object.keys(assets).length > 0 ? assets : undefined,
		confirmationPolicies: state.confirmationPolicies,
		// Passthrough chains first: a chain that also got re-selected as managed
		// would otherwise shadow the fresh entry (last [[chains]] wins at load).
		chains: [
			...state.passthroughChains,
			...state.chains.map((chain) => ({ rpcUrls: chain.rpcUrls, bundlerUrl: chain.bundlerUrl ?? "" })),
		],
		rebalancing,
		vault: hasVault ? vault : undefined,
		allowlist: scrubbedAllowlist,
	}
}

function chainComments(state: WizardState): string[] {
	return [
		...state.passthroughChains.map(() => "Kept from the previous configuration"),
		...state.chains.map((chain) => `${chain.meta.label} (chainId ${chain.meta.chainId})`),
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
	lines.push(`Pairs: ${state.pairs.map((p) => `${p.token0}/${p.token1}`).join(", ")}`)
	lines.push(`Output: ${outputPath}`)
	note(lines.join("\n"), "Summary")
}

/**
 * Boot requires a confirmation policy for every chain, passthrough ones
 * included — the wizard never prompts for those, so it can only warn.
 */
function warnUncoveredPassthroughChains(state: WizardState, prefill?: Prefill): void {
	if (!prefill) return
	const covered = new Set<number>()
	for (const source of [DEFAULT_CONFIRMATION_POLICIES, state.confirmationPolicies ?? {}]) {
		for (const key of Object.keys(source)) {
			const chainId = parseChainKey(key)
			if (chainId !== null) covered.add(chainId)
		}
	}
	prefill.config.chains.forEach((chain, index) => {
		const chainId = prefill.chainIds[index]
		if (chainId === null || !state.passthroughChains.includes(chain) || covered.has(chainId)) return
		log.warn(
			`No confirmation policy covers the kept chain ${chainId} (${chain.rpcUrls[0]}) — \`simplex run\` will refuse to start until a [confirmationPolicies."${chainId}"] entry is added.`,
		)
	})
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
