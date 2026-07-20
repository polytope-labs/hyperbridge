import { useState } from "react"
import { api } from "../../api"
import { patchChain, type ChainDraft } from "../state"
import type { StepProps } from "../Wizard"

interface AlchemyChainRow {
	chainId: number
	rpcUrl: string | null
	bundlerUrl: string | null
}

export function StepChains({ state, setState }: StepProps) {
	const [busy, setBusy] = useState(false)

	const patch = (chainId: number, changes: Partial<ChainDraft>) => setState((s) => patchChain(s, chainId, changes))

	const applyAlchemyKey = async () => {
		if (!state.alchemyKey.trim()) return
		setBusy(true)
		try {
			const res = await api.post<{ valid: boolean; error?: string; chains: AlchemyChainRow[] }>(
				"/api/setup/validate-alchemy-key",
				{ apiKey: state.alchemyKey.trim(), network: state.network },
			)
			setState((s) => ({
				...s,
				alchemyStatus: res.valid ? "ok" : "err",
				alchemyError: res.error,
				chains: res.valid
					? s.chains.map((c) => {
							const row = res.chains.find((r) => r.chainId === c.meta.chainId)
							if (!row?.rpcUrl) return c
							return {
								...c,
								rpcUrls: [row.rpcUrl, ...c.rpcUrls.slice(1)],
								bundlerUrl: row.bundlerUrl ?? c.bundlerUrl,
								viaAlchemy: true,
								rpcStatus: undefined,
							}
						})
					: s.chains,
			}))
		} finally {
			setBusy(false)
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
			const bundler = await api.post<{ ok: boolean; warning?: string }>("/api/setup/validate-bundler", {
				url: chain.bundlerUrl.trim(),
				chainId: chain.meta.chainId,
			})
			patch(chain.meta.chainId, { bundlerWarning: bundler.warning, bundlerOk: !bundler.warning })
		}
	}

	return (
		<div>
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
						value={state.alchemyKey}
						onChange={(e) => setState((s) => ({ ...s, alchemyKey: e.target.value, alchemyStatus: undefined }))}
					/>
					<button type="button" onClick={applyAlchemyKey} disabled={busy || !state.alchemyKey.trim()}>
						Validate & prefill
					</button>
					{state.alchemyStatus === "ok" && <span className="badge ok">key valid — URLs prefilled</span>}
					{state.alchemyStatus === "err" && <span className="badge err">{state.alchemyError}</span>}
				</div>
			</div>

			{state.chains.map((chain) => (
				<div className="card" key={chain.meta.chainId}>
					<div className="spread">
						<h2>
							{chain.meta.label} <span className="badge">chainId {chain.meta.chainId}</span>{" "}
							{chain.viaAlchemy && <span className="badge ok">via Alchemy</span>}
						</h2>
						<label className="row">
							<input
								type="checkbox"
								checked={chain.enabled}
								onChange={(e) => patch(chain.meta.chainId, { enabled: e.target.checked })}
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
									<span>
										{index === 0
											? "RPC URL (scans order events, reads balances, simulates fills)"
											: "Additional RPC for quorum scanning (must be a different provider)"}
									</span>
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
									onClick={() => patch(chain.meta.chainId, { rpcUrls: [...chain.rpcUrls, ""] })}
								>
									+ quorum RPC
								</button>
								<span className="hint">
									2+ organisationally independent providers make event scans Byzantine-fault-tolerant: one lying
									RPC fails the batch instead of feeding you fake orders.
								</span>
							</div>

							<label className="field">
								<span>ERC-4337 bundler URL (fills are submitted through it as UserOperations)</span>
								<input
									type="text"
									value={chain.bundlerUrl}
									onChange={(e) => patch(chain.meta.chainId, { bundlerUrl: e.target.value })}
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
								<label className="row">
									<input
										type="checkbox"
										checked={chain.watchOnly}
										onChange={(e) => patch(chain.meta.chainId, { watchOnly: e.target.checked })}
									/>
									watch-only (observe orders, never fill)
								</label>
							</div>
						</div>
					)}
				</div>
			))}
		</div>
	)
}
