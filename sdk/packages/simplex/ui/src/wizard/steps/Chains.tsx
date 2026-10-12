import * as Collapsible from "@radix-ui/react-collapsible"
import { api } from "../../api"
import { ChainCollapseTrigger, isHeaderControl, useChainPanels } from "../../components/ChainPanel"
import { ChainLogo } from "../../components/ChainLogo"
import { EndpointVerificationStatus } from "../../components/EndpointVerificationStatus"
import { loadOrderbook } from "../orderbook"
import { patchChain, type ChainDraft } from "../state"
import type { StepProps } from "../Wizard"

export function StepChains({ state, setState }: StepProps) {
	const panels = useChainPanels()

	const patch = (chainId: number, changes: Partial<ChainDraft>) => setState((s) => patchChain(s, chainId, changes))

	const verifyChain = async (chain: ChainDraft) => {
		patch(chain.meta.chainId, {
			verificationState: "checking",
			verificationMessage: "Checking RPC endpoints…",
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

		patch(chain.meta.chainId, {
			verificationState: "success",
			verificationMessage: "RPC connection is ready.",
		})
	}

	return (
		<div className="wizard-sections chains-step">
			{state.orderbookError ? (
				<div className="card">
					<p className="error">{state.orderbookError}</p>
					<button type="button" onClick={() => void loadOrderbook(setState)}>
						Retry
					</button>
				</div>
			) : null}

			{state.chains.map((chain) => (
				<Collapsible.Root
					className="card chain-configuration"
					data-enabled={chain.enabled}
					key={chain.meta.chainId}
					open={panels.isOpen(chain.meta.chainId, chain.enabled)}
					onOpenChange={(open) => panels.setOpen(chain.meta.chainId, open)}
				>
					<div
						className="chain-configuration-header"
						onClick={(e) => {
							if (isHeaderControl(e)) return
							panels.setOpen(chain.meta.chainId, !panels.isOpen(chain.meta.chainId, chain.enabled))
						}}
					>
						<div className="chain-identity">
							<ChainLogo label={chain.meta.label} />
							<div>
								<h2>{chain.meta.label}</h2>
							</div>
						</div>
						<div className="chain-header-controls">
							<label className="chain-enable-toggle">
								<input
									type="checkbox"
									checked={chain.enabled}
									onChange={(e) => {
										patch(chain.meta.chainId, { enabled: e.target.checked })
										panels.setOpen(chain.meta.chainId, e.target.checked)
									}}
								/>
								<span className="chain-enable-switch" aria-hidden="true" />
								<span>Enable fills</span>
							</label>
							<ChainCollapseTrigger label={chain.meta.label} />
						</div>
					</div>
					<Collapsible.Content className="chain-collapsible-content">
						<div className="chain-configuration-fields">
							{chain.rpcUrls.map((url, index) => (
								<label className="field" key={index}>
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
