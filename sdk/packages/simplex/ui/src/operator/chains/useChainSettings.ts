import { useCallback, useRef, useState } from "react"
import { api } from "../../api"
import { useAction, usePolling } from "../../lib/hooks"
import type { EndpointVerificationState } from "../../components/EndpointVerificationStatus"
import type { ChainDefault, ChainsDto } from "../../types"

export interface ChainDraft {
	meta: ChainDefault
	enabled: boolean
	rpcUrls: string[]
	/** Not editable. Hyperbridge's bundler the chain fills through, empty where it runs none. */
	bundlerUrl: string
	watchOnly: boolean
	running: boolean
	verificationState?: EndpointVerificationState
	verificationMessage?: string
}

function seedDrafts(dto: ChainsDto): ChainDraft[] {
	const configured = new Map(dto.chains.map((row) => [row.chainId, row]))
	const drafts = dto.catalog.map((meta) => {
		const row = configured.get(meta.chainId)
		return {
			meta,
			enabled: Boolean(row),
			rpcUrls: row ? [...row.rpcUrls] : meta.defaultRpcUrls?.length ? [...meta.defaultRpcUrls] : [""],
			bundlerUrl: meta.hyperbridgeBundlerUrl ?? "",
			watchOnly: row?.watchOnly ?? false,
			running: row?.running ?? false,
		}
	})
	for (const row of dto.chains) {
		if (dto.catalog.some((meta) => meta.chainId === row.chainId)) continue
		drafts.push({
			meta: { chainId: row.chainId, stateMachineId: row.stateMachineId, label: row.label, network: dto.network },
			enabled: true,
			rpcUrls: [...row.rpcUrls],
			bundlerUrl: row.bundlerUrl,
			watchOnly: row.watchOnly,
			running: row.running,
		})
	}
	return drafts
}

/** Live chain editor state and endpoint mutations, isolated from its view. */
export function useChainSettings() {
	const [dto, setDto] = useState<ChainsDto>()
	const [drafts, setDrafts] = useState<ChainDraft[]>()
	const [saved, setSaved] = useState(false)
	const verifyingRef = useRef(new Set<number>())
	const { run, message, error } = useAction()

	const load = useCallback(async () => {
		const next = await api.get<ChainsDto>("/api/chains")
		setDto(next)
		setDrafts((current) => current ?? seedDrafts(next))
	}, [])
	usePolling(useCallback(() => run(load, undefined, "poll"), [run, load]))

	const patch = (chainId: number, changes: Partial<ChainDraft>) =>
		setDrafts((rows) => rows?.map((row) => (row.meta.chainId === chainId ? { ...row, ...changes } : row)))
	const chains = drafts ?? []
	const verifyChain = async (chain: ChainDraft) => {
		if (verifyingRef.current.has(chain.meta.chainId)) return
		verifyingRef.current.add(chain.meta.chainId)
		patch(chain.meta.chainId, {
			verificationState: "checking",
			verificationMessage: "Checking RPC endpoints…",
		})
		try {
			const urls = chain.rpcUrls.map((url) => url.trim()).filter(Boolean)
			try {
				const rpc = await api.post<{ ok: boolean; results: Array<{ error?: string }>; error?: string }>(
					"/api/setup/validate-rpc",
					{ urls, expectedChainId: chain.meta.chainId },
				)
				if (!rpc.ok) {
					const description =
						rpc.error ?? rpc.results.find((result) => result.error)?.error ?? "RPC check failed"
					patch(chain.meta.chainId, {
						verificationState: "error",
						verificationMessage: `RPC could not be verified: ${description}`,
					})
					return
				}
			} catch (cause) {
				const description = cause instanceof Error ? cause.message : String(cause)
				patch(chain.meta.chainId, {
					verificationState: "error",
					verificationMessage: `RPC could not be verified: ${description}`,
				})
				return
			}

			patch(chain.meta.chainId, {
				verificationState: "success",
				verificationMessage: "RPC connection is ready.",
			})
		} finally {
			verifyingRef.current.delete(chain.meta.chainId)
		}
	}

	const toggleChain = (chain: ChainDraft, enabled: boolean) => {
		if (
			!enabled &&
			chain.running &&
			!window.confirm(`Stop filling on ${chain.meta.label}? It keeps trading until you restart the filler.`)
		) {
			return
		}
		patch(chain.meta.chainId, { enabled })
	}

	const save = () => {
		setSaved(false)
		return run(async () => {
			await api.put("/api/chains", {
				chains: chains
					.filter((chain) => chain.enabled)
					.map((chain) => ({
						chainId: chain.meta.chainId,
						rpcUrls: chain.rpcUrls.map((url) => url.trim()).filter(Boolean),
						watchOnly: chain.watchOnly,
					})),
			})
			setSaved(true)
			const next = await api.get<ChainsDto>("/api/chains")
			setDto(next)
			setDrafts(seedDrafts(next))
		}, "Chains saved")
	}

	return {
		dto,
		chains,
		loaded: drafts !== undefined,
		patch,
		saved,
		message,
		error,
		verifyChain,
		toggleChain,
		save,
	}
}
