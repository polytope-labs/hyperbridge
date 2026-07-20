import { confirm, intro, log, select, spinner } from "@clack/prompts"
import { existsSync, readFileSync } from "fs"
import { resolve } from "path"
import { parse } from "toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { fetchChainId } from "@/services/FillerConfigService"
import { guard, withTimeout, PROBE_TIMEOUT_MS } from "./prompt-utils"
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
	await stepChains(state, prefill)
	await stepBundlers(state, prefill)
	await stepSigner(state, prefill)
	await stepHyperbridge(state, prefill)
	await stepStrategies(state, prefill)
	await stepFineTune(state, prefill)
	await stepWrite(state, outputPath)
}

/**
 * A config already exists at the output path: offer to start with it as-is,
 * walk the wizard with its values prefilled, or start over.
 */
async function handleExistingConfig(outputPath: string): Promise<Prefill | undefined> {
	let config: FillerTomlConfig | undefined
	let invalidReason: string | undefined
	try {
		config = parse(readFileSync(outputPath, "utf-8")) as FillerTomlConfig
		validateConfig(config)
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
