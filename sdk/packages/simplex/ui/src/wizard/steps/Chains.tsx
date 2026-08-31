import { useState } from "react"
import * as Collapsible from "@radix-ui/react-collapsible"
import { toast } from "sonner"
import { api } from "../../api"
import { ChainLogo } from "../../components/ChainLogo"
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
			if (res.valid) {
				toast.success("Provider endpoints added", {
					description: "Supported chain endpoints were filled from your Alchemy key.",
				})
			} else {
				toast.error("Alchemy key could not be validated", { description: res.error })
			}
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err)
			setState((s) => ({
				...s,
				alchemyStatus: "err",
				alchemyError: message,
			}))
			toast.error("Alchemy key could not be validated", { description: message })
		} finally {
			setBusy(false)
		}
	}

	const verifyChain = async (chain: ChainDraft) => {
		const toastId = toast.loading(`Verifying ${chain.meta.label}`, {
			description: "Checking the RPC and bundler endpoints.",
		})
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
				toast.error(`${chain.meta.label} RPC could not be verified`, {
					description: firstError,
					id: toastId,
				})
				return
			}
			patch(chain.meta.chainId, { rpcStatus: "ok" })
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err)
			patch(chain.meta.chainId, { rpcStatus: "err", rpcError: message })
			toast.error(`${chain.meta.label} RPC could not be verified`, {
				description: message,
				id: toastId,
			})
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
					toast.warning(`${chain.meta.label} RPC verified`, {
						description: bundler.warning,
						id: toastId,
					})
					return
				}
			} catch (err) {
				const message = `Bundler check failed: ${err instanceof Error ? err.message : err}`
				patch(chain.meta.chainId, {
					bundlerWarning: message,
					bundlerOk: false,
				})
				toast.error(`${chain.meta.label} bundler could not be verified`, {
					description: message,
					id: toastId,
				})
				return
			}
		}

		toast.success(`${chain.meta.label} endpoints verified`, {
			description: chain.bundlerUrl.trim()
				? "RPC and bundler connections are ready."
				: "RPC connection is ready.",
			id: toastId,
		})
	}

	return (
		<div className="wizard-sections chains-step">
			<div className="card">
				<h2>Provider key</h2>
				<p className="hint">
					One Alchemy API key can fill in the RPC and bundler URL for every supported chain — Alchemy serves
					ERC-4337 bundler methods on the same endpoint. Use premium endpoints with archive access; free tiers
					rate-limit and break event scanning. Every field stays editable if you prefer other providers (e.g.
					a Pimlico bundler).
				</p>
				<div className="chain-provider-controls">
					<input
						type="password"
						aria-label="Alchemy API key"
						style={{ maxWidth: "24rem" }}
						placeholder="Alchemy API key (optional)"
						value={state.alchemyKey}
						onChange={(e) =>
							setState((s) => ({ ...s, alchemyKey: e.target.value, alchemyStatus: undefined }))
						}
					/>
					<button type="button" onClick={applyAlchemyKey} disabled={busy || !state.alchemyKey.trim()}>
						Validate & prefill
					</button>
				</div>
			</div>

			{state.chains.map((chain) => (
				<Collapsible.Root
					className="card chain-configuration"
					data-enabled={chain.enabled}
					key={chain.meta.chainId}
					open={chain.enabled}
				>
					<div className="chain-configuration-header">
						<div className="chain-identity">
							<ChainLogo label={chain.meta.label} />
							<div>
								<h2>{chain.meta.label}</h2>
								{chain.viaAlchemy && <span className="chain-source">Configured with Alchemy</span>}
							</div>
						</div>
						<label className="chain-enable-toggle">
							<input
								type="checkbox"
								checked={chain.enabled}
								onChange={(e) => patch(chain.meta.chainId, { enabled: e.target.checked })}
							/>
							<span className="chain-enable-switch" aria-hidden="true" />
							<span>Enable fills</span>
						</label>
					</div>
					{chain.meta.note && (
						<div className="chain-warning" role="note" aria-label="Warning">
							<span className="chain-warning-icon" aria-hidden="true">
								!
							</span>
							<span>
								<strong>Native gas required</strong>
								<small>{chain.meta.note}</small>
							</span>
						</div>
					)}
					<Collapsible.Content className="chain-collapsible-content">
						<div className="chain-configuration-fields">
							{chain.rpcUrls.map((url, index) => (
								<label className="field" key={index}>
									<span className="field-label">
										{index === 0 ? "RPC endpoint" : "Backup RPC endpoint"}
										{index === 0 ? <span className="field-required">Required</span> : null}
									</span>
									{index === 0 && <small>Used to read the chain and find orders.</small>}
									{index > 0 && (
										<small>A second provider helps protect against bad or unavailable data.</small>
									)}
									<div className="row">
										<input
											type="text"
											style={{ flex: 1 }}
											value={url}
											required={index === 0}
											onChange={(e) =>
												patch(chain.meta.chainId, {
													rpcUrls: chain.rpcUrls.map((u, i) =>
														i === index ? e.target.value : u,
													),
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
							<div className="chain-backup-rpc">
								<button
									className="chain-add-backup-button"
									type="button"
									title="A backup RPC lets Simplex compare independent providers before it acts on chain data."
									onClick={() => patch(chain.meta.chainId, { rpcUrls: [...chain.rpcUrls, ""] })}
								>
									<span aria-hidden="true">+</span>
									Add backup RPC
								</button>
								<p className="chain-info-text">
									<span className="chain-info-icon" aria-hidden="true">
										i
									</span>
									<span>A backup lets Simplex compare providers before it acts on chain data.</span>
								</p>
							</div>

							<label className="field">
								<span className="field-label">
									Bundler endpoint <span className="field-required">Required</span>
								</span>
								<small>Used to submit sponsored fills on this chain.</small>
								<input
									type="text"
									value={chain.bundlerUrl}
									required
									onChange={(e) => patch(chain.meta.chainId, { bundlerUrl: e.target.value })}
									placeholder="https://api.pimlico.io/v2/<chainId>/rpc?apikey=…"
								/>
							</label>

							<div className="chain-configuration-actions">
								<button
									type="button"
									disabled={!chain.rpcUrls[0]?.trim() || chain.rpcStatus === "checking"}
									onClick={() => verifyChain(chain)}
								>
									{chain.rpcStatus === "checking" ? "Verifying…" : "Verify"}
								</button>
								<label className="chain-watch-toggle">
									<input
										type="checkbox"
										checked={chain.watchOnly}
										onChange={(e) => patch(chain.meta.chainId, { watchOnly: e.target.checked })}
									/>
									<span>
										<strong>Observe only</strong>
										<small>Monitor orders without filling them.</small>
									</span>
								</label>
							</div>
						</div>
					</Collapsible.Content>
				</Collapsible.Root>
			))}
		</div>
	)
}
