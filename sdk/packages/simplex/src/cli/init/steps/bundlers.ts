import { confirm, select, text } from "@clack/prompts"
import { isAlchemyUrl } from "../derive/alchemy"
import { parsePimlicoUrl, derivePimlicoBundler } from "../derive/pimlico"
import { guard, why, isValidUrl } from "../prompt-utils"
import { WHY } from "../help-text"
import type { Prefill, WizardState } from "../state"

export async function stepBundlers(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.bundler)

	// Alchemy RPCs double as ERC-4337 bundlers — one confirm covers all of them.
	const alchemyChains = state.chains.filter((chain) => !chain.bundlerUrl && isAlchemyUrl(chain.rpcUrls[0]))
	if (alchemyChains.length > 0) {
		const reuse = guard(
			await confirm({
				message: `Reuse the Alchemy RPC as the ERC-4337 bundler for ${alchemyChains.map((c) => c.meta.label).join(", ")}?`,
				initialValue: true,
			}),
		)
		if (reuse) {
			for (const chain of alchemyChains) chain.bundlerUrl = chain.rpcUrls[0]
		}
	}

	let pimlicoKey: string | undefined
	let pimlicoConfirmed = false

	for (const chain of state.chains) {
		if (chain.bundlerUrl) continue

		const existing = prefillBundlerFor(chain.meta.chainId, prefill)
		const derived = pimlicoKey && pimlicoConfirmed ? derivePimlicoBundler(pimlicoKey, chain.meta.chainId) : null

		let url: string
		if (derived) {
			const choice = guard(
				await select({
					message: `Bundler for ${chain.meta.label}`,
					options: [
						{ value: "derived", label: "Use the derived Pimlico URL", hint: derived },
						{ value: "custom", label: "Enter a different URL" },
					],
				}),
			)
			url = choice === "derived" ? derived : guard(await askBundlerUrl(chain.meta.label, existing))
		} else {
			url = guard(await askBundlerUrl(chain.meta.label, existing))
		}
		chain.bundlerUrl = url.trim()

		// One Pimlico key serves every chain — offer to derive the rest.
		if (pimlicoKey === undefined) {
			const parsed = parsePimlicoUrl(url.trim())
			const remaining = state.chains.filter((c) => !c.bundlerUrl)
			if (parsed && remaining.length > 0) {
				pimlicoConfirmed = guard(
					await confirm({
						message: `Pimlico key detected — derive bundler URLs for ${remaining.map((c) => c.meta.label).join(", ")} from the same key?`,
						initialValue: true,
					}),
				)
				pimlicoKey = parsed.apiKey
			}
		}
	}
}

function prefillBundlerFor(chainId: number, prefill?: Prefill): string | undefined {
	if (!prefill) return undefined
	const index = prefill.chainIds.indexOf(chainId)
	if (index === -1) return undefined
	return prefill.config.chains[index]?.bundlerUrl || undefined
}

function askBundlerUrl(label: string, initialValue?: string) {
	return text({
		message: `ERC-4337 bundler URL for ${label}`,
		placeholder: "https://api.pimlico.io/v2/<chainId>/rpc?apikey=<your-key>",
		initialValue,
		validate: (value) => {
			const trimmed = (value ?? "").trim()
			if (!trimmed) return "Bundler URL is required — fills are submitted through it"
			if (!isValidUrl(trimmed)) return "Enter a valid http(s) URL"
			return undefined
		},
	})
}
