import { enabledChains, patchAt, removeAt } from "../state"
import type { StepProps } from "../Wizard"

export function StepTreasury({ state, setState, defaults }: StepProps) {
	const chains = enabledChains(state)

	return (
		<div>
			<p className="hint">Everything on this page is optional — skip it for a minimal setup.</p>

			<div className="card">
				<h2>ERC-4626 treasury vaults</h2>
				<p className="hint">
					When the wallet holds more than the sweep threshold, the excess is deposited into the vault to earn
					yield, keeping min balance liquid for gas and small fills; fills pull funds back out when needed.
					Amounts are in USD. One vault per asset per chain.
				</p>

				{chains.some((c) => (defaults.knownVaults[c.meta.stateMachineId] ?? []).length > 0) && (
					<div style={{ marginBottom: "0.8rem" }}>
						<p className="hint">Known vaults — tick to add:</p>
						{chains.map((chain) =>
							(defaults.knownVaults[chain.meta.stateMachineId] ?? []).map((known) => {
								const selected = state.vaults.some(
									(v) =>
										v.chain === chain.meta.stateMachineId &&
										v.vault.toLowerCase() === known.address.toLowerCase(),
								)
								return (
									<label className="row" key={`${chain.meta.stateMachineId}-${known.address}`}>
										<input
											type="checkbox"
											checked={selected}
											onChange={(e) =>
												setState((s) => ({
													...s,
													vaults: e.target.checked
														? [
																...s.vaults,
																{
																	chain: chain.meta.stateMachineId,
																	vault: known.address,
																	threshold: "5000",
																	minBalance: "3000",
																	redeemOnShutdown: false,
																},
															]
														: s.vaults.filter(
																(v) =>
																	!(
																		v.chain === chain.meta.stateMachineId &&
																		v.vault.toLowerCase() === known.address.toLowerCase()
																	),
															),
												}))
											}
										/>
										{chain.meta.label}: {known.label} ({known.asset}){" "}
										<span className="mono">{known.address.slice(0, 10)}…</span>
									</label>
								)
							}),
						)}
					</div>
				)}

				{state.vaults.map((vault, index) => (
					// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
					<div className="row" key={index} style={{ marginBottom: "0.5rem", alignItems: "flex-end" }}>
						<label className="field" style={{ margin: 0 }}>
							<span>Chain</span>
							<select
								value={vault.chain}
								onChange={(e) =>
									setState((s) => ({
										...s,
										vaults: patchAt(s.vaults, index, { chain: e.target.value }),
									}))
								}
							>
								{chains.map((c) => (
									<option key={c.meta.stateMachineId} value={c.meta.stateMachineId}>
										{c.meta.label}
									</option>
								))}
							</select>
						</label>
						<label className="field" style={{ flex: 1, margin: 0 }}>
							<span>Vault address</span>
							<input
								type="text"
								placeholder="0x…"
								value={vault.vault}
								onChange={(e) =>
									setState((s) => ({
										...s,
										vaults: patchAt(s.vaults, index, { vault: e.target.value }),
									}))
								}
							/>
						</label>
						<label className="field" style={{ maxWidth: "9rem", margin: 0 }}>
							<span>Sweep threshold ($)</span>
							<input
								type="text"
								value={vault.threshold}
								onChange={(e) =>
									setState((s) => ({
										...s,
										vaults: patchAt(s.vaults, index, { threshold: e.target.value }),
									}))
								}
							/>
						</label>
						<label className="field" style={{ maxWidth: "9rem", margin: 0 }}>
							<span>Min balance ($)</span>
							<input
								type="text"
								value={vault.minBalance}
								onChange={(e) =>
									setState((s) => ({
										...s,
										vaults: patchAt(s.vaults, index, { minBalance: e.target.value }),
									}))
								}
							/>
						</label>
						<label
							className="row"
							style={{ whiteSpace: "nowrap" }}
							title="Redeem this position to the wallet on graceful shutdown"
						>
							<input
								type="checkbox"
								checked={vault.redeemOnShutdown}
								onChange={(e) =>
									setState((s) => ({
										...s,
										vaults: patchAt(s.vaults, index, { redeemOnShutdown: e.target.checked }),
									}))
								}
							/>
							redeem on shutdown
						</label>
						<button
							type="button"
							onClick={() => setState((s) => ({ ...s, vaults: removeAt(s.vaults, index) }))}
						>
							✕
						</button>
					</div>
				))}
				<button
					type="button"
					onClick={() =>
						setState((s) => ({
							...s,
							vaults: [
								...s.vaults,
								{
									chain: chains[0]?.meta.stateMachineId ?? "",
									vault: "",
									threshold: "5000",
									minBalance: "3000",
									redeemOnShutdown: false,
								},
							],
						}))
					}
				>
					+ Add vault
				</button>
			</div>
		</div>
	)
}
