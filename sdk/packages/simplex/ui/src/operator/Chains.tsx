import { useCallback, useState } from "react"
import { api } from "../api"
import { useAction, usePolling } from "../lib/hooks"
import type { ChainDefault, ChainsDto } from "../types"

interface AlchemyChainRow {
	chainId: number
	rpcUrl: string | null
	bundlerUrl: string | null
}

/**
 * Same draft shape the setup wizard edits chains with, plus `running` — false
 * for a chain the operator selected since boot, which the filler only picks up
 * on the next start.
 */
interface ChainDraft {
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

/**
 * Seeds one draft per catalog chain, enabled where the running config already
 * has it. A configured chain outside the catalog (hand-written chain id) keeps
 * its own card so an edit here can never silently drop it.
 */
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

/**
 * Chain selection and endpoints for a running filler — the setup wizard's chain
 * step, against the live config. Chain topology is wired at boot (clients, event
 * monitors, balance tracking), so saves are validated and persisted here and
 * take effect on the next start.
 */
export function Chains() {
	const [dto, setDto] = useState<ChainsDto>()
	const [drafts, setDrafts] = useState<ChainDraft[]>()
	const [alchemyKey, setAlchemyKey] = useState("")
	const [alchemy, setAlchemy] = useState<{ status?: "ok" | "err"; error?: string; busy?: boolean }>({})
	const [saved, setSaved] = useState(false)
	const { run, message, error } = useAction()

	const load = useCallback(async () => {
		const next = await api.get<ChainsDto>("/api/chains")
		setDto(next)
		// Seed the editor once; later edits stay local until saved.
		setDrafts((current) => current ?? seedDrafts(next))
	}, [])
	usePolling(useCallback(() => run(load), [run, load]))

	const patch = (chainId: number, changes: Partial<ChainDraft>) =>
		setDrafts((rows) => rows?.map((row) => (row.meta.chainId === chainId ? { ...row, ...changes } : row)))

	const chains = drafts ?? []

	const applyAlchemyKey = async () => {
		if (!alchemyKey.trim() || !dto) return
		setAlchemy({ busy: true })
		try {
			const res = await api.post<{ valid: boolean; error?: string; chains: AlchemyChainRow[] }>(
				"/api/setup/validate-alchemy-key",
				{ apiKey: alchemyKey.trim(), network: dto.network },
			)
			setAlchemy({ status: res.valid ? "ok" : "err", error: res.error })
			if (!res.valid) return
			setDrafts((rows) =>
				rows?.map((row) => {
					const filled = res.chains.find((r) => r.chainId === row.meta.chainId)
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
		} catch (err) {
			setAlchemy({ status: "err", error: err instanceof Error ? err.message : String(err) })
		}
	}

	const verifyChain = async (chain: ChainDraft) => {
		patch(chain.meta.chainId, { rpcStatus: "checking", rpcError: undefined, bundlerWarning: undefined })
		const urls = chain.rpcUrls.map((u) => u.trim()).filter(Boolean)
		try {
			const rpc = await api.post<{ ok: boolean; results: Array<{ error?: string }>; error?: string }>(
				"/api/setup/validate-rpc",
				{ urls, expectedChainId: chain.meta.chainId },
			)
			if (!rpc.ok) {
				const firstError = rpc.error ?? rpc.results.find((r) => r.error)?.error ?? "RPC check failed"
				patch(chain.meta.chainId, { rpcStatus: "err", rpcError: firstError })
				return
			}
			patch(chain.meta.chainId, { rpcStatus: "ok" })
		} catch (err) {
			patch(chain.meta.chainId, { rpcStatus: "err", rpcError: err instanceof Error ? err.message : String(err) })
			return
		}

		if (chain.bundlerUrl.trim()) {
			try {
				const bundler = await api.post<{ ok: boolean; warning?: string }>("/api/setup/validate-bundler", {
					url: chain.bundlerUrl.trim(),
					chainId: chain.meta.chainId,
				})
				patch(chain.meta.chainId, { bundlerWarning: bundler.warning, bundlerOk: !bundler.warning })
			} catch (err) {
				patch(chain.meta.chainId, {
					bundlerWarning: `Bundler check failed: ${err instanceof Error ? err.message : err}`,
					bundlerOk: false,
				})
			}
		}
	}

	/** Dropping a chain the filler is trading is worth a beat the wizard doesn't need. */
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

	return (
		<div>
			<h2 style={{ marginTop: "1.5rem" }}>Chains & endpoints</h2>
			<p className="hint">
				The chains this filler watches and fills on, and the endpoints it reaches them through — the setup
				wizard's chain step against the running config.
			</p>

			<div className="card">
				<h2>Provider key</h2>
				<p className="hint">
					One Alchemy API key can fill in the RPC and bundler URL for every supported chain — Alchemy serves
					ERC-4337 bundler methods on the same endpoint. Use premium endpoints with archive access; free tiers
					rate-limit and break event scanning. Every field stays editable if you prefer other providers (e.g. a
					Pimlico bundler).
				</p>
				<div className="row">
					<input
						type="password"
						style={{ maxWidth: "24rem" }}
						placeholder="Alchemy API key (optional)"
						value={alchemyKey}
						onChange={(e) => {
							setAlchemyKey(e.target.value)
							setAlchemy({})
						}}
					/>
					<button type="button" onClick={applyAlchemyKey} disabled={alchemy.busy || !alchemyKey.trim()}>
						Validate & prefill
					</button>
					{alchemy.status === "ok" && <span className="badge ok">key valid — URLs prefilled</span>}
					{alchemy.status === "err" && <span className="badge err">{alchemy.error}</span>}
				</div>
			</div>

			{chains.map((chain) => (
				<div className="card" key={chain.meta.chainId}>
					<div className="spread">
						<h2>
							{chain.meta.label} <span className="badge">chainId {chain.meta.chainId}</span>{" "}
							{chain.viaAlchemy && <span className="badge ok">via Alchemy</span>}{" "}
							{chain.enabled && !chain.running && <span className="badge warn">pending restart</span>}
						</h2>
						<label className="row">
							<input
								type="checkbox"
								checked={chain.enabled}
								onChange={(e) => toggleChain(chain, e.target.checked)}
							/>
							fill on this chain
						</label>
					</div>
					{chain.meta.note && <p className="hint">Note: {chain.meta.note}</p>}
					{chain.enabled && (
						<div>
							{chain.rpcUrls.map((url, index) => (
								// biome-ignore lint/suspicious/noArrayIndexKey: positional quorum rows
								<label className="field" key={index}>
									<span>{index === 0 ? "RPC URL" : "Additional RPC (different provider)"}</span>
									<div className="row">
										<input
											type="text"
											style={{ flex: 1 }}
											value={url}
											onChange={(e) =>
												patch(chain.meta.chainId, {
													rpcUrls: chain.rpcUrls.map((u, i) => (i === index ? e.target.value : u)),
													rpcStatus: undefined,
													viaAlchemy: index === 0 ? false : chain.viaAlchemy,
												})
											}
										/>
										{index > 0 && (
											<button
												type="button"
												onClick={() =>
													patch(chain.meta.chainId, {
														rpcUrls: chain.rpcUrls.filter((_, i) => i !== index),
													})
												}
											>
												✕
											</button>
										)}
									</div>
								</label>
							))}
							<div className="row">
								<button
									type="button"
									title="Order scans require a quorum of providers to agree, so one lying RPC can't feed you fake orders. The quorum is exactly the URLs you list here — add at least two organisationally independent providers; four or more to tolerate a faulty one."
									onClick={() => patch(chain.meta.chainId, { rpcUrls: [...chain.rpcUrls, ""] })}
								>
									+ quorum RPC
								</button>
							</div>

							<label className="field">
								<span>Bundler URL</span>
								<input
									type="text"
									value={chain.bundlerUrl}
									onChange={(e) => patch(chain.meta.chainId, { bundlerUrl: e.target.value, bundlerOk: false })}
									placeholder="https://api.pimlico.io/v2/<chainId>/rpc?apikey=…"
								/>
							</label>

							<div className="row">
								<button
									type="button"
									disabled={!chain.rpcUrls[0]?.trim() || chain.rpcStatus === "checking"}
									onClick={() => verifyChain(chain)}
								>
									{chain.rpcStatus === "checking" ? "Verifying…" : "Verify"}
								</button>
								{chain.rpcStatus === "ok" && <span className="badge ok">RPC verified</span>}
								{chain.rpcStatus === "err" && <span className="badge err">{chain.rpcError}</span>}
								{chain.bundlerOk && <span className="badge ok">bundler ok</span>}
								{chain.bundlerWarning && <span className="badge warn">{chain.bundlerWarning}</span>}
								<label
									className="row"
									title={
										dto?.globalWatchOnly
											? "[simplex].watchOnly is set globally — edit the config to change it"
											: undefined
									}
								>
									<input
										type="checkbox"
										checked={chain.watchOnly}
										disabled={dto?.globalWatchOnly}
										onChange={(e) => patch(chain.meta.chainId, { watchOnly: e.target.checked })}
									/>
									watch-only (observe orders, never fill)
								</label>
							</div>
						</div>
					)}
				</div>
			))}

			<div className="card">
				<div className="row">
					<button type="button" className="primary" disabled={drafts === undefined} onClick={save}>
						Save chains
					</button>
					{saved && <span className="badge warn">restart the filler to apply</span>}
					{message && <span className="badge ok">{message}</span>}
					{error && <span className="badge err">{error}</span>}
				</div>
				<p className="hint">
					Saving validates every new endpoint against the chain it claims to serve and writes the config; the
					filler picks the changes up on the next start. Dropping a chain is refused while a market symbol or a
					funding venue still depends on it. A newly selected chain needs funding — the filler wallet must hold
					the assets it quotes there before it can fill.
				</p>
			</div>
		</div>
	)
}
