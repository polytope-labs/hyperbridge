import { useState } from "react"
import * as Collapsible from "@radix-ui/react-collapsible"
import { toast } from "sonner"
import { api } from "../../api"
import { ChainLogo } from "../../components/ChainLogo"
import { EndpointVerificationStatus } from "../../components/EndpointVerificationStatus"
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
				{ apiKey: state.alchemyKey.trim() },
			)
			setState((s) => ({
				...s,
				alchemyStatus: res.valid ? "ok" : "err",
				alchemyError: res.error,
				chains: res.valid
					? s.chains.map((c) => {
							const row = res.chains.find((r) => r.chainId === c.meta.chainId)
							if (!row?.rpcUrl) return c
							// Bundler only. The scan reads from the public quorum, which
							// costs nothing and spreads across providers; sending it to
							// Alchemy instead would burn the key's quota on polling and
							// leave the chain on a single provider.
							return {
								...c,
								bundlerUrl: row.bundlerUrl ?? row.rpcUrl,
								viaAlchemy: true,
								verificationState: undefined,
								verificationMessage: undefined,
							}
						})
					: s.chains,
			}))
			if (res.valid) {
				toast.success("Bundlers configured", {
					description: "Every supported chain now submits fills through your Alchemy key.",
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
		patch(chain.meta.chainId, {
			verificationState: "checking",
			verificationMessage: "Checking RPC and bundler endpoints…",
		})
		const urls = chain.rpcUrls.map((u) => u.trim()).filter(Boolean)
		try {
			const rpc = await api.post<{ ok: boolean; results: Array<{ error?: string }>; error?: string }>(
				"/api/setup/validate-rpc",
				{ urls, expectedChainId: chain.meta.chainId },
			)
			if (!rpc.ok) {
				const firstError = rpc.error ?? rpc.results.find((r) => r.error)?.error ?? "RPC check failed"
				patch(chain.meta.chainId, {
					verificationState: "error",
					verificationMessage: `RPC could not be verified: ${firstError}`,
				})
				return
			}
		} catch (err) {
			const message = err instanceof Error ? err.message : String(err)
			patch(chain.meta.chainId, {
				verificationState: "error",
				verificationMessage: `RPC could not be verified: ${message}`,
			})
			return
		}

		if (chain.bundlerUrl.trim()) {
			try {
				const bundler = await api.post<{ ok: boolean; warning?: string }>("/api/setup/validate-bundler", {
					url: chain.bundlerUrl.trim(),
					chainId: chain.meta.chainId,
				})
				if (bundler.warning) {
					patch(chain.meta.chainId, {
						verificationState: "warning",
						verificationMessage: `RPC verified. Bundler warning: ${bundler.warning}`,
					})
					return
				}
			} catch (err) {
				const message = `Bundler check failed: ${err instanceof Error ? err.message : err}`
				patch(chain.meta.chainId, {
					verificationState: "error",
					verificationMessage: message,
				})
				return
			}
		}

		patch(chain.meta.chainId, {
			verificationState: "success",
			verificationMessage: chain.bundlerUrl.trim()
				? "RPC and bundler connections are ready."
				: "RPC connection is ready.",
		})
	}

	return (
		<div className="wizard-sections chains-step">
			<div className="card">
				<h2>Bundler key</h2>
				<p className="hint">
					One Alchemy API key sets up the bundler on every chain, and the RPC endpoints below are already
					filled in.
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
								{chain.viaAlchemy && <span className="chain-source">Bundler via Alchemy</span>}
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
					<Collapsible.Content className="chain-collapsible-content">
						<div className="chain-configuration-fields">
							{chain.rpcUrls.map((url, index) => (
								<label className="field chain-rpc-field" key={index}>
									<span className="field-label">
										{index === 0 ? "RPC endpoint" : `RPC endpoint ${index + 1}`}
										{index === 0 ? <span className="field-required">Required</span> : null}
									</span>
									{index === 0 && <small>Used to read the chain and find orders.</small>}
									{index === 1 && (
										<small>
											Reads are agreed across every endpoint listed, so a wrong or unavailable
											answer from any one of them cannot mislead the filler.
										</small>
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
													verificationState: undefined,
													verificationMessage: undefined,
												})
											}
										/>
										{index > 0 && (
											<button
												type="button"
												onClick={() =>
													patch(chain.meta.chainId, {
														rpcUrls: chain.rpcUrls.filter((_, i) => i !== index),
														verificationState: undefined,
														verificationMessage: undefined,
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
									onClick={() =>
										patch(chain.meta.chainId, {
											rpcUrls: [...chain.rpcUrls, ""],
											verificationState: undefined,
											verificationMessage: undefined,
										})
									}
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
									onChange={(e) =>
										patch(chain.meta.chainId, {
											bundlerUrl: e.target.value,
											verificationState: undefined,
											verificationMessage: undefined,
										})
									}
									placeholder="https://api.pimlico.io/v2/<chainId>/rpc?apikey=…"
								/>
							</label>

						<div className="chain-configuration-actions">
							<div className="chain-verification-control">
								<button
									type="button"
									disabled={
										!chain.rpcUrls[0]?.trim() || chain.verificationState === "checking"
									}
									onClick={() => verifyChain(chain)}
								>
									{chain.verificationState === "checking" ? "Verifying…" : "Verify"}
								</button>
								<EndpointVerificationStatus
									state={chain.verificationState}
									message={chain.verificationMessage}
								/>
							</div>
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
