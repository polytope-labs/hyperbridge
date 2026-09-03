import * as Collapsible from "@radix-ui/react-collapsible"
import { ChainLogo } from "../components/ChainLogo"
import { useChainSettings } from "./chains/useChainSettings"

export function Chains() {
	const model = useChainSettings()
	const {
		dto,
		chains,
		loaded,
		patch,
		alchemyKey,
		updateAlchemyKey,
		alchemy,
		saved,
		message,
		error,
		applyAlchemyKey,
		verifyChain,
		toggleChain,
		save,
	} = model

	return (
		<div className="wizard-sections chains-step operator-chain-settings">
			<div className="card">
				<h2>Provider key</h2>
				<p className="hint">
					Use one Alchemy key to fill supported RPC and bundler endpoints, or enter providers manually below.
					Premium endpoints with archive access are recommended for reliable event scanning.
				</p>
				<div className="chain-provider-controls">
					<input
						type="password"
						aria-label="Alchemy API key"
						placeholder="Alchemy API key (optional)"
						value={alchemyKey}
						onChange={(e) => updateAlchemyKey(e.target.value)}
					/>
					<button type="button" onClick={applyAlchemyKey} disabled={alchemy.busy || !alchemyKey.trim()}>
						{alchemy.busy ? "Checking…" : "Validate & prefill"}
					</button>
				</div>
			</div>

			{chains.map((chain) => (
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
								{chain.viaAlchemy ? (
									<span className="chain-source">Configured with Alchemy</span>
								) : null}
								{chain.enabled && !chain.running ? (
									<span className="chain-source">Applies after restart</span>
								) : null}
							</div>
						</div>
						<label className="chain-enable-toggle">
							<input
								type="checkbox"
								checked={chain.enabled}
								onChange={(e) => toggleChain(chain, e.target.checked)}
							/>
							<span className="chain-enable-switch" aria-hidden="true" />
							<span>Enable fills</span>
						</label>
					</div>
					{chain.meta.note ? (
						<div className="chain-warning" role="note" aria-label="Warning">
							<span className="chain-warning-icon" aria-hidden="true">
								!
							</span>
							<span>
								<strong>Native gas required</strong>
								<small>{chain.meta.note}</small>
							</span>
						</div>
					) : null}
					<Collapsible.Content className="chain-collapsible-content">
						<div className="chain-configuration-fields">
							{chain.rpcUrls.map((url, index) => (
								<label className="field" key={index}>
									<span className="field-label">
										{index === 0 ? "RPC endpoint" : "Backup RPC endpoint"}
										{index === 0 ? <span className="field-required">Required</span> : null}
									</span>
									<small>
										{index === 0
											? "Used to read the chain and find orders."
											: "A second provider protects against unavailable or incorrect data."}
									</small>
									<div className="row">
										<input
											type="text"
											value={url}
											required={index === 0}
											onChange={(e) =>
												patch(chain.meta.chainId, {
													rpcUrls: chain.rpcUrls.map((item, itemIndex) =>
														itemIndex === index ? e.target.value : item,
													),
													rpcStatus: undefined,
													viaAlchemy: index === 0 ? false : chain.viaAlchemy,
												})
											}
										/>
										{index > 0 ? (
											<button
												type="button"
												aria-label="Remove backup RPC"
												onClick={() =>
													patch(chain.meta.chainId, {
														rpcUrls: chain.rpcUrls.filter(
															(_, itemIndex) => itemIndex !== index,
														),
													})
												}
											>
												✕
											</button>
										) : null}
									</div>
								</label>
							))}
							<div className="chain-backup-rpc">
								<button
									className="chain-add-backup-button"
									type="button"
									onClick={() => patch(chain.meta.chainId, { rpcUrls: [...chain.rpcUrls, ""] })}
								>
									<span aria-hidden="true">+</span> Add backup RPC
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
										patch(chain.meta.chainId, { bundlerUrl: e.target.value, bundlerOk: false })
									}
									placeholder="https://api.pimlico.io/v2/<chainId>/rpc?apikey=…"
								/>
							</label>
							<div className="chain-configuration-actions">
								<button
									type="button"
									disabled={!chain.rpcUrls[0]?.trim() || chain.rpcStatus === "checking"}
									onClick={() => verifyChain(chain)}
								>
									{chain.rpcStatus === "checking" ? "Verifying…" : "Verify endpoints"}
								</button>
								<label
									className="chain-watch-toggle"
									title={
										dto?.globalWatchOnly
											? "Observe-only mode is set globally in the config file."
											: undefined
									}
								>
									<input
										type="checkbox"
										checked={chain.watchOnly}
										disabled={dto?.globalWatchOnly}
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

			<div className="operator-chain-save">
				<div className="row">
					<button type="button" className="primary" disabled={!loaded} onClick={save}>
						Save chain settings
					</button>
					{saved ? <span className="badge warn">Restart Simplex to apply</span> : null}
					{message ? <span className="badge ok">{message}</span> : null}
					{error ? <span className="badge err">{error}</span> : null}
				</div>
				<p className="hint">
					Changes are validated and written to the local config. A newly enabled chain must be funded before
					it can fill.
				</p>
			</div>
		</div>
	)
}
