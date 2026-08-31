import { useCallback, useRef, useState } from "react"
import { toast } from "sonner"
import { api } from "../../api"
import { useAction, usePolling } from "../../lib/hooks"
import type { ChainDefault, ChainsDto } from "../../types"

interface AlchemyChainRow {
	chainId: number
	rpcUrl: string | null
	bundlerUrl: string | null
}

export interface ChainDraft {
	meta: ChainDefault
	enabled: boolean
	rpcUrls: string[]
	bundlerUrl: string
	viaAlchemy: boolean
	watchOnly: boolean
	running: boolean
	rpcStatus?: "ok" | "err" | "checking"
	rpcError?: string
	bundlerWarning?: string
	bundlerOk?: boolean
}

function seedDrafts(dto: ChainsDto): ChainDraft[] {
	const configured = new Map(dto.chains.map((row) => [row.chainId, row]))
	const drafts = dto.catalog.map((meta) => {
		const row = configured.get(meta.chainId)
		return {
			meta,
			enabled: Boolean(row),
			rpcUrls: row ? [...row.rpcUrls] : [""],
			bundlerUrl: row?.bundlerUrl ?? "",
			viaAlchemy: false,
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
			viaAlchemy: false,
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
	const [alchemyKey, setAlchemyKey] = useState("")
	const [alchemy, setAlchemy] = useState<{ status?: "ok" | "err"; error?: string; busy?: boolean }>({})
	const [saved, setSaved] = useState(false)
	const alchemyBusyRef = useRef(false)
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

	const applyAlchemyKey = async () => {
		if (alchemyBusyRef.current || !alchemyKey.trim() || !dto) return
		alchemyBusyRef.current = true
		setAlchemy({ busy: true })
		try {
			const result = await api.post<{ valid: boolean; error?: string; chains: AlchemyChainRow[] }>(
				"/api/setup/validate-alchemy-key",
				{ apiKey: alchemyKey.trim(), network: dto.network },
			)
			setAlchemy({ status: result.valid ? "ok" : "err", error: result.error })
			if (!result.valid) {
				toast.error("Alchemy key could not be validated", { description: result.error })
				return
			}
			setDrafts((rows) =>
				rows?.map((row) => {
					const filled = result.chains.find((candidate) => candidate.chainId === row.meta.chainId)
					if (!filled?.rpcUrl) return row
					return {
						...row,
						rpcUrls: [filled.rpcUrl, ...row.rpcUrls.slice(1)],
						bundlerUrl: filled.bundlerUrl ?? row.bundlerUrl,
						viaAlchemy: true,
						rpcStatus: undefined,
					}
				}),
			)
			toast.success("Provider endpoints added", {
				description: "Supported chain endpoints were filled from your Alchemy key.",
			})
		} catch (cause) {
			const description = cause instanceof Error ? cause.message : String(cause)
			setAlchemy({ status: "err", error: description })
			toast.error("Alchemy key could not be validated", { description })
		} finally {
			alchemyBusyRef.current = false
		}
	}

	const verifyChain = async (chain: ChainDraft) => {
		if (verifyingRef.current.has(chain.meta.chainId)) return
		verifyingRef.current.add(chain.meta.chainId)
		const toastId = toast.loading(`Verifying ${chain.meta.label}`, {
			description: "Checking the RPC and bundler endpoints.",
		})
		patch(chain.meta.chainId, { rpcStatus: "checking", rpcError: undefined, bundlerWarning: undefined })
		try {
			const urls = chain.rpcUrls.map((url) => url.trim()).filter(Boolean)
			try {
				const rpc = await api.post<{ ok: boolean; results: Array<{ error?: string }>; error?: string }>(
					"/api/setup/validate-rpc",
					{ urls, expectedChainId: chain.meta.chainId },
				)
				if (rpc.ok) patch(chain.meta.chainId, { rpcStatus: "ok" })
				else {
					const description =
						rpc.error ?? rpc.results.find((result) => result.error)?.error ?? "RPC check failed"
					patch(chain.meta.chainId, { rpcStatus: "err", rpcError: description })
					toast.error(`${chain.meta.label} RPC could not be verified`, { description, id: toastId })
					return
				}
			} catch (cause) {
				const description = cause instanceof Error ? cause.message : String(cause)
				patch(chain.meta.chainId, { rpcStatus: "err", rpcError: description })
				toast.error(`${chain.meta.label} RPC could not be verified`, { description, id: toastId })
				return
			}

			if (chain.bundlerUrl.trim()) {
				try {
					const bundler = await api.post<{ ok: boolean; warning?: string }>("/api/setup/validate-bundler", {
						url: chain.bundlerUrl.trim(),
						chainId: chain.meta.chainId,
					})
					patch(chain.meta.chainId, { bundlerWarning: bundler.warning, bundlerOk: !bundler.warning })
					if (bundler.warning) {
						toast.warning(`${chain.meta.label} RPC verified`, { description: bundler.warning, id: toastId })
						return
					}
				} catch (cause) {
					const description = `Bundler check failed: ${cause instanceof Error ? cause.message : cause}`
					patch(chain.meta.chainId, { bundlerWarning: description, bundlerOk: false })
					toast.error(`${chain.meta.label} bundler could not be verified`, { description, id: toastId })
					return
				}
			}
			toast.success(`${chain.meta.label} endpoints verified`, {
				description: chain.bundlerUrl.trim()
					? "RPC and bundler connections are ready."
					: "RPC connection is ready.",
				id: toastId,
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
						bundlerUrl: chain.bundlerUrl.trim(),
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
		patch,
		alchemyKey,
		setAlchemyKey,
		alchemy,
		saved,
		message,
		error,
		applyAlchemyKey,
		verifyChain,
		toggleChain,
		save,
	}
}
