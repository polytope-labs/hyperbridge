import { confirm } from "@clack/prompts"
import { isAlchemyUrl } from "../derive/alchemy"
import { parsePimlicoUrl, derivePimlicoBundler } from "../derive/pimlico"
import { ProviderDerivation, askDerivedOrCustom } from "../derive-flow"
import { guard, why, askUrl } from "../prompt-utils"
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

	// One Pimlico key serves every chain — offer to derive the rest.
	const pimlico = new ProviderDerivation({
		detect: (url) => parsePimlicoUrl(url)?.apiKey ?? null,
		derive: derivePimlicoBundler,
		confirmMessage: (remaining) => `Pimlico key detected — derive bundler URLs for ${remaining} from the same key?`,
	})

	for (const chain of state.chains) {
		if (chain.bundlerUrl) continue

		const existing = prefillBundlerFor(chain.meta.chainId, prefill)
		const derived = pimlico.candidate(chain.meta.chainId)

		const url = derived
			? await askDerivedOrCustom(`Bundler for ${chain.meta.label}`, derived, "Use the derived Pimlico URL", () =>
					askBundlerUrl(chain.meta.label, existing),
				)
			: await askBundlerUrl(chain.meta.label, existing)
		chain.bundlerUrl = url

		const remaining = state.chains.filter((c) => !c.bundlerUrl)
		await pimlico.offer(
			url,
			remaining.map((c) => c.meta.label),
		)
	}
}

function prefillBundlerFor(chainId: number, prefill?: Prefill): string | undefined {
	if (!prefill) return undefined
	const index = prefill.chainIds.indexOf(chainId)
	if (index === -1) return undefined
	return prefill.config.chains[index]?.bundlerUrl || undefined
}

function askBundlerUrl(label: string, initial?: string): Promise<string> {
	return askUrl(`ERC-4337 bundler URL for ${label}`, {
		initial,
		placeholder: "https://api.pimlico.io/v2/<chainId>/rpc?apikey=<your-key>",
		required: "Bundler URL is required — fills are submitted through it",
	})
}
