import { confirm, intro, log, select, spinner } from "@clack/prompts"
import { existsSync, readFileSync } from "fs"
import { resolve } from "path"
import { parse } from "toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { fetchChainId } from "@/services/FillerConfigService"
import { guard, withTimeout, PROBE_TIMEOUT_MS } from "./prompt-utils"
import { migrateLegacyConfig } from "./migrate-legacy"
import { newWizardState, type Prefill } from "./state"
import { stepChains } from "./steps/chains"
import { stepBundlers } from "./steps/bundlers"
import { stepSigner } from "./steps/signer"
import { stepHyperbridge } from "./steps/hyperbridge"
import { stepStrategies } from "./steps/strategies"
import { stepFineTune } from "./steps/finetune"
import { stepWrite, startFiller } from "./steps/write"

export interface InitOptions {
	output: string
}

export async function runInit(options: InitOptions): Promise<void> {
	if (!process.stdout.isTTY || !process.stdin.isTTY) {
		console.error(
			"simplex init needs an interactive terminal. For non-interactive setups, copy filler-config-example.toml and edit it by hand.",
		)
		process.exit(1)
	}

	const outputPath = resolve(process.cwd(), options.output)

	intro("simplex init — interactive filler setup")
	log.message(
		[
			"This wizard asks only for what the filler needs to boot and fill orders,",
			"explains why each value matters, then writes a commented filler-config.toml.",
			"Nothing is written until you confirm at the end. Cancel any time with ctrl-c.",
		].join("\n"),
	)

	let prefill: Prefill | undefined
	if (existsSync(outputPath)) {
		prefill = await handleExistingConfig(outputPath)
	}

	const state = newWizardState()
	state.prefillConfig = prefill?.config
	await stepChains(state, prefill)
	await stepBundlers(state, prefill)
	await stepSigner(state, prefill)
	await stepHyperbridge(state, prefill)

	// Salvage loop: a failure at the write gate must not lose everything the
	// operator typed — offer to redo the markets steps with the same state.
	for (;;) {
		try {
			await stepStrategies(state, prefill)
			await stepFineTune(state, prefill)
			await stepWrite(state, outputPath, prefill)
			return
		} catch (error) {
			log.error(error instanceof Error ? error.message : String(error))
			const goBack = guard(
				await confirm({ message: "Go back and fix the markets configuration?", initialValue: true }),
			)
			if (!goBack) {
				log.info("Nothing was written.")
				process.exit(1)
			}
			state.pairs = []
		}
	}
}

/**
 * A config already exists at the output path: offer to start with it as-is,
 * walk the wizard with its values prefilled, or start over.
 */
async function handleExistingConfig(outputPath: string): Promise<Prefill | undefined> {
	let config: FillerTomlConfig | undefined
	let invalidReason: string | undefined
	let degraded = false
	try {
		config = parse(readFileSync(outputPath, "utf-8")) as FillerTomlConfig
		// Pre-pair-engine configs ([[strategies]]) are migrated to pairs so an
		// update run offers the old values as prefills instead of failing.
		if ("strategies" in config) {
			try {
				const notes = migrateLegacyConfig(config)
				if (notes.length > 0) {
					log.warn(
						`This config predates the pair engine — migrated for the update run:\n${notes.map((n) => `  - ${n}`).join("\n")}\nReview the pair prompts before writing.`,
					)
				}
			} catch (error) {
				// A failed migration must not discard the whole prefill: chains,
				// signer, substrate and vault values still carry over; the markets
				// are configured fresh. Skip validation — this prefill has no pairs.
				log.warn(
					`Legacy strategy migration failed (${error instanceof Error ? error.message : String(error)}) — the old markets can't be prefilled; everything else still is.`,
				)
				delete (config as Record<string, unknown>).strategies
				degraded = true
			}
		}
		if (!degraded) validateConfig(config)
	} catch (error) {
		invalidReason = error instanceof Error ? error.message : String(error)
	}

	if (!config || invalidReason) {
		log.warn(`Found ${outputPath}, but it doesn't pass validation: ${invalidReason}`)
		const fresh = guard(
			await confirm({ message: "Start fresh? (the file is only replaced after you confirm)", initialValue: true }),
		)
		if (!fresh) {
			log.info("Nothing changed. Fix the file by hand or re-run simplex init.")
			process.exit(0)
		}
		return undefined
	}

	const action = guard(
		await select({
			message: `Found an existing config at ${outputPath} — what do you want to do?`,
			options: [
				{ value: "start", label: "Start the filler with it as-is" },
				{ value: "update", label: "Update values", hint: "walk through the wizard with current values prefilled" },
				{ value: "fresh", label: "Start fresh", hint: "ignore the existing values" },
			],
		}),
	)

	if (action === "start") {
		await startFiller(outputPath)
	}
	if (action === "fresh") return undefined

	// Chain identity lives in the RPCs, not the file — resolve ids so prompts can be prefilled.
	const spin = spinner()
	spin.start("Resolving chain ids from the existing config's RPCs")
	const chainIds = await Promise.all(
		config.chains.map(async (chain) => {
			try {
				return await withTimeout(fetchChainId(chain.rpcUrls[0]), PROBE_TIMEOUT_MS)
			} catch {
				return null
			}
		}),
	)
	spin.stop(`Resolved ${chainIds.filter((id) => id !== null).length}/${chainIds.length} chains`)

	return { config, chainIds }
}
