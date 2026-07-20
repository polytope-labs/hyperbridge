import { confirm, log, multiselect, select, spinner } from "@clack/prompts"
import { fetchChainId } from "@/services/FillerConfigService"
import { chainsForNetwork, chainByAlchemySubdomain, type InitChainMeta, type InitNetwork } from "../chains"
import { parseAlchemyUrl, deriveAlchemyRpc } from "../derive/alchemy"
import { ProviderDerivation, askDerivedOrCustom } from "../derive-flow"
import { guard, why, askUrl, withTimeout, PROBE_TIMEOUT_MS } from "../prompt-utils"
import { WHY } from "../help-text"
import type { Prefill, WizardState, WizardChain } from "../state"

export async function stepChains(state: WizardState, prefill?: Prefill): Promise<void> {
	const prefillNetwork = detectPrefillNetwork(prefill)
	const network = guard(
		await select<InitNetwork>({
			message: "Which network do you want to run on?",
			initialValue: prefillNetwork ?? "mainnet",
			options: [
				{ value: "mainnet", label: "Mainnet", hint: "real funds, real orders" },
				{ value: "testnet", label: "Testnet", hint: "Sepolia-family chains, for trying things out" },
			],
		}),
	)
	state.network = network

	const available = chainsForNetwork(network)
	why(WHY.chains)
	const prefillIds = new Set((prefill?.chainIds ?? []).filter((id): id is number => id !== null))
	const selected = guard(
		await multiselect<number>({
			message: "Select the chains to fill orders on (space to toggle, enter to confirm)",
			options: available.map((chain) => ({
				value: chain.chainId,
				label: chain.label,
				hint: chain.note,
			})),
			initialValues:
				prefillIds.size > 0
					? available.filter((c) => prefillIds.has(c.chainId)).map((c) => c.chainId)
					: available.map((c) => c.chainId),
			required: true,
		}),
	)

	state.chains = available.filter((chain) => selected.includes(chain.chainId)).map((meta) => ({ meta, rpcUrls: [] }))

	carryUnmanagedChains(state, prefill)
	await collectRpcUrls(state, prefill)
	await verifyRpcUrls(state)
}

function detectPrefillNetwork(prefill?: Prefill): InitNetwork | undefined {
	if (!prefill) return undefined
	for (const chainId of prefill.chainIds) {
		if (chainId === null) continue
		const meta = chainsForNetwork("testnet").find((c) => c.chainId === chainId)
		if (meta) return "testnet"
	}
	return "mainnet"
}

/** Chains in the existing config the wizard doesn't manage are kept verbatim. */
function carryUnmanagedChains(state: WizardState, prefill?: Prefill): void {
	if (!prefill) return
	prefill.config.chains.forEach((chain, index) => {
		const chainId = prefill.chainIds[index]
		const managed = chainId !== null && state.chains.some((c) => c.meta.chainId === chainId)
		const knownToWizard = chainId !== null && chainsForNetwork(state.network).some((c) => c.chainId === chainId)
		if (!managed && !knownToWizard) {
			state.passthroughChains.push(chain)
			log.warn(
				`Keeping unmanaged chain from the existing config unchanged (chainId ${chainId ?? "unknown"}: ${chain.rpcUrls[0]})`,
			)
		}
	})
}

function prefillRpcFor(chainId: number, prefill?: Prefill): string[] | undefined {
	if (!prefill) return undefined
	const index = prefill.chainIds.indexOf(chainId)
	if (index === -1) return undefined
	return prefill.config.chains[index]?.rpcUrls
}

function askRpcUrl(meta: InitChainMeta, initial?: string): Promise<string> {
	return askUrl(`RPC URL for ${meta.label}`, {
		initial,
		placeholder: meta.alchemySubdomain
			? `https://${meta.alchemySubdomain}.g.alchemy.com/v2/<your-key>`
			: "https://...",
		required: "RPC URL is required",
		validate: (url) => alchemyMismatch(url, meta),
	})
}

/** Catches pasting one chain's Alchemy URL into another chain's field before any network call. */
function alchemyMismatch(url: string, meta: InitChainMeta): string | undefined {
	const parsed = parseAlchemyUrl(url)
	if (!parsed) return undefined
	const impliedChain = chainByAlchemySubdomain(parsed.subdomain)
	if (impliedChain && impliedChain.chainId !== meta.chainId) {
		return `This is an Alchemy ${impliedChain.label} URL, but you're configuring ${meta.label}`
	}
	return undefined
}

async function collectRpcUrls(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.rpc)

	// One Alchemy key serves every chain Alchemy supports — offer to derive the rest.
	const alchemy = new ProviderDerivation({
		detect: (url) => parseAlchemyUrl(url)?.apiKey ?? null,
		derive: deriveAlchemyRpc,
		confirmMessage: (remaining) => `Alchemy key detected — derive RPC URLs for ${remaining} from the same key?`,
	})

	for (const chain of state.chains) {
		const existing = prefillRpcFor(chain.meta.chainId, prefill)
		const derived = alchemy.candidate(chain.meta.chainId)

		let url: string
		if (existing?.length) {
			url = await askRpcUrl(chain.meta, existing[0])
		} else if (derived) {
			url = await askDerivedOrCustom(`RPC for ${chain.meta.label}`, derived, "Use the derived Alchemy URL", () =>
				askRpcUrl(chain.meta),
			)
		} else {
			url = await askRpcUrl(chain.meta)
		}

		chain.rpcUrls = [url]
		const remaining = state.chains.filter((c) => c.rpcUrls.length === 0 && c.meta.alchemySubdomain)
		await alchemy.offer(
			url,
			remaining.map((c) => c.meta.label),
		)

		await maybeAddQuorumUrls(chain)
	}
}

async function maybeAddQuorumUrls(chain: WizardChain): Promise<void> {
	const addQuorum = guard(
		await confirm({
			message: `Add a second RPC provider for ${chain.meta.label} (quorum log scanning)?`,
			initialValue: false,
		}),
	)
	if (!addQuorum) return
	why(WHY.quorum)

	for (;;) {
		const url = await askUrl(`Additional RPC URL for ${chain.meta.label} (must be a different provider)`, {
			required: "RPC URL is required",
			validate: (candidate) => {
				const mismatch = alchemyMismatch(candidate, chain.meta)
				if (mismatch) return mismatch
				const hostname = new URL(candidate).hostname.toLowerCase()
				const clash = chain.rpcUrls.some((u) => new URL(u).hostname.toLowerCase() === hostname)
				if (clash) return "Quorum URLs must point to different hostnames"
				return undefined
			},
		})
		chain.rpcUrls.push(url)

		const more = guard(
			await confirm({ message: `Add another RPC provider for ${chain.meta.label}?`, initialValue: false }),
		)
		if (!more) return
	}
}

/** Confirms every RPC actually serves the chain it was entered for, before anything is written. */
async function verifyRpcUrls(state: WizardState): Promise<void> {
	const dropped: WizardChain[] = []

	for (const chain of state.chains) {
		let verified = false
		while (!verified) {
			const spin = spinner()
			spin.start(`Verifying ${chain.meta.label} RPC${chain.rpcUrls.length > 1 ? "s" : ""}`)

			let failure: string | undefined
			try {
				const chainIds = await Promise.all(
					chain.rpcUrls.map((url) => withTimeout(fetchChainId(url), PROBE_TIMEOUT_MS, "RPC check")),
				)
				const wrong = chainIds.findIndex((id) => id !== chain.meta.chainId)
				if (wrong !== -1) {
					failure = `${chain.rpcUrls[wrong]} reports chainId ${chainIds[wrong]}, expected ${chain.meta.chainId} (${chain.meta.label})`
				}
			} catch (error) {
				failure = error instanceof Error ? error.message : String(error)
			}

			if (!failure) {
				spin.stop(`${chain.meta.label} RPC verified (chainId ${chain.meta.chainId})`)
				verified = true
				continue
			}

			spin.stop(`${chain.meta.label} RPC check failed`)
			log.error(failure)

			const action = guard(
				await select({
					message: `How do you want to handle ${chain.meta.label}?`,
					options: [
						{ value: "reenter", label: "Re-enter the RPC URL(s)" },
						{ value: "retry", label: "Retry the check" },
						{ value: "keep", label: "Keep anyway", hint: "verified again when the filler starts" },
						{ value: "drop", label: "Drop this chain" },
					],
				}),
			)
			if (action === "retry") continue
			if (action === "keep") {
				verified = true
				continue
			}
			if (action === "drop") {
				dropped.push(chain)
				verified = true
				continue
			}
			chain.rpcUrls = [await askRpcUrl(chain.meta)]
		}
	}

	if (dropped.length > 0) {
		state.chains = state.chains.filter((chain) => !dropped.includes(chain))
		if (state.chains.length === 0 && state.passthroughChains.length === 0) {
			log.error("All chains were dropped — at least one chain is required. Let's pick RPCs again.")
			for (const chain of dropped) chain.rpcUrls = []
			state.chains = dropped
			await collectRpcUrls(state)
			await verifyRpcUrls(state)
		}
	}
}
