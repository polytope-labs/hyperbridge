import { confirm, log, multiselect, select, spinner, text } from "@clack/prompts"
import { fetchChainId } from "@/services/FillerConfigService"
import { chainsForNetwork, chainByAlchemySubdomain, type InitChainMeta, type InitNetwork } from "../chains"
import { parseAlchemyUrl, deriveAlchemyRpc } from "../derive/alchemy"
import { guard, why, isValidUrl } from "../prompt-utils"
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
			initialValues: prefillIds.size > 0 ? available.filter((c) => prefillIds.has(c.chainId)).map((c) => c.chainId) : available.map((c) => c.chainId),
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

async function collectRpcUrls(state: WizardState, prefill?: Prefill): Promise<void> {
	why(WHY.rpc)

	let derivedKey: string | undefined
	let deriveConfirmed = false

	for (const chain of state.chains) {
		const existing = prefillRpcFor(chain.meta.chainId, prefill)
		const derived = derivedKey ? deriveAlchemyRpc(derivedKey, chain.meta.chainId) : null

		let url: string
		if (existing?.length) {
			url = guard(
				await text({
					message: `RPC URL for ${chain.meta.label}`,
					initialValue: existing[0],
					validate: (value) => validateRpcInput(value, chain.meta),
				}),
			)
		} else if (derived && deriveConfirmed) {
			const choice = guard(
				await select({
					message: `RPC for ${chain.meta.label}`,
					options: [
						{ value: "derived", label: `Use the derived Alchemy URL`, hint: derived },
						{ value: "custom", label: "Enter a different URL" },
					],
				}),
			)
			url =
				choice === "derived"
					? derived
					: guard(
							await text({
								message: `RPC URL for ${chain.meta.label}`,
								validate: (value) => validateRpcInput(value, chain.meta),
							}),
						)
		} else {
			url = guard(
				await text({
					message: `RPC URL for ${chain.meta.label}`,
					placeholder: chain.meta.alchemySubdomain
						? `https://${chain.meta.alchemySubdomain}.g.alchemy.com/v2/<your-key>`
						: "https://...",
					validate: (value) => validateRpcInput(value, chain.meta),
				}),
			)
		}

		chain.rpcUrls = [url.trim()]

		// One Alchemy key serves every chain Alchemy supports — offer to derive the rest.
		if (derivedKey === undefined) {
			const parsed = parseAlchemyUrl(url.trim())
			const remaining = state.chains.filter((c) => c.rpcUrls.length === 0 && c.meta.alchemySubdomain)
			if (parsed && remaining.length > 0) {
				const useKey = guard(
					await confirm({
						message: `Alchemy key detected — derive RPC URLs for ${remaining.map((c) => c.meta.label).join(", ")} from the same key?`,
						initialValue: true,
					}),
				)
				derivedKey = parsed.apiKey
				deriveConfirmed = useKey
				if (!useKey) derivedKey = undefined
			}
		}

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
		const url = guard(
			await text({
				message: `Additional RPC URL for ${chain.meta.label} (must be a different provider)`,
				validate: (value) => {
					const base = validateRpcInput(value ?? "", chain.meta)
					if (base) return base
					const hostname = new URL(value!.trim()).hostname.toLowerCase()
					const clash = chain.rpcUrls.some((u) => new URL(u).hostname.toLowerCase() === hostname)
					if (clash) return "Quorum URLs must point to different hostnames"
					return undefined
				},
			}),
		)
		chain.rpcUrls.push(url.trim())

		const more = guard(
			await confirm({ message: `Add another RPC provider for ${chain.meta.label}?`, initialValue: false }),
		)
		if (!more) return
	}
}

function validateRpcInput(value: string | undefined, meta: InitChainMeta): string | undefined {
	const trimmed = (value ?? "").trim()
	if (!trimmed) return "RPC URL is required"
	if (!isValidUrl(trimmed)) return "Enter a valid http(s) URL"
	const parsed = parseAlchemyUrl(trimmed)
	if (parsed) {
		const impliedChain = chainByAlchemySubdomain(parsed.subdomain)
		if (impliedChain && impliedChain.chainId !== meta.chainId) {
			return `This is an Alchemy ${impliedChain.label} URL, but you're configuring ${meta.label}`
		}
	}
	return undefined
}

const RPC_TIMEOUT_MS = 10_000

function fetchChainIdWithTimeout(url: string): Promise<number> {
	return Promise.race([
		fetchChainId(url),
		new Promise<never>((_, reject) => {
			const timer = setTimeout(
				() => reject(new Error(`timed out after ${RPC_TIMEOUT_MS / 1000}s`)),
				RPC_TIMEOUT_MS,
			)
			timer.unref?.()
		}),
	])
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
				const chainIds = await Promise.all(chain.rpcUrls.map((url) => fetchChainIdWithTimeout(url)))
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
			chain.rpcUrls = []
			const url = guard(
				await text({
					message: `RPC URL for ${chain.meta.label}`,
					validate: (value) => validateRpcInput(value, chain.meta),
				}),
			)
			chain.rpcUrls = [url.trim()]
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
