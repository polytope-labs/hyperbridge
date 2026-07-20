import { enabledChains, patchAt, removeAt } from "../state"
import type { StepProps } from "../Wizard"

export function StepTreasury({ state, setState }: StepProps) {
	const chains = enabledChains(state)

	return (
		<div>
			<p className="hint">Everything on this page is optional — skip it for a minimal setup.</p>

			<div className="card">
				<div className="spread">
					<h2>Rebalancing</h2>
					<label className="row">
						<input
							type="checkbox"
							checked={state.rebalancingEnabled}
							onChange={(e) => setState((s) => ({ ...s, rebalancingEnabled: e.target.checked }))}
						/>
						enabled
					</label>
				</div>
				<p className="hint">
					Automatically tops up a chain's stablecoin balance from richer chains when it falls below a fraction of
					its base level.
				</p>
				{state.rebalancingEnabled && (
					<div>
						<label className="field" style={{ maxWidth: "16rem" }}>
							<span>Trigger fraction (0.5 = act when a chain drops to 50% of base)</span>
							<input
								type="text"
								value={state.rebalancingTrigger}
								onChange={(e) => setState((s) => ({ ...s, rebalancingTrigger: e.target.value }))}
							/>
						</label>
						<p className="hint">USDC base balance per chain (USD; leave empty to skip a chain):</p>
						<div className="row">
							{chains.map((chain) => (
								<label className="field" key={chain.meta.chainId} style={{ maxWidth: "11rem" }}>
									<span>{chain.meta.label}</span>
									<input
										type="text"
										placeholder="10000"
										value={state.rebalancingUsdc[String(chain.meta.chainId)] ?? ""}
										onChange={(e) =>
											setState((s) => ({
												...s,
												rebalancingUsdc: { ...s.rebalancingUsdc, [String(chain.meta.chainId)]: e.target.value },
											}))
										}
									/>
								</label>
							))}
						</div>
						<label className="field" style={{ maxWidth: "20rem" }}>
							<span>Binance API key (optional CEX rebalancing leg)</span>
							<input
								type="password"
								value={state.binanceKey}
								onChange={(e) => setState((s) => ({ ...s, binanceKey: e.target.value }))}
							/>
						</label>
						<label className="field" style={{ maxWidth: "20rem" }}>
							<span>Binance API secret</span>
							<input
								type="password"
								value={state.binanceSecret}
								onChange={(e) => setState((s) => ({ ...s, binanceSecret: e.target.value }))}
							/>
						</label>
					</div>
				)}
			</div>

			<div className="card">
				<h2>ERC-4626 treasury vaults</h2>
				<p className="hint">
					Fills can source missing stablecoin balance from a vault position atomically, and idle wallet balance
					above a threshold is swept in to earn yield (e.g. Aave stataUSDC). The underlying asset is resolved
					on-chain from the vault address.
				</p>
				{state.vaults.map((vault, index) => (
					// biome-ignore lint/suspicious/noArrayIndexKey: positional rows
					<div className="row" key={index} style={{ marginBottom: "0.5rem" }}>
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
						<input
							type="text"
							placeholder="vault address 0x…"
							style={{ flex: 1 }}
							value={vault.vault}
							onChange={(e) =>
								setState((s) => ({
									...s,
									vaults: patchAt(s.vaults, index, { vault: e.target.value }),
								}))
							}
						/>
						<input
							type="text"
							placeholder="sweep threshold"
							style={{ maxWidth: "9rem" }}
							value={vault.threshold}
							onChange={(e) =>
								setState((s) => ({
									...s,
									vaults: patchAt(s.vaults, index, { threshold: e.target.value }),
								}))
							}
						/>
						<input
							type="text"
							placeholder="min balance"
							style={{ maxWidth: "9rem" }}
							value={vault.minBalance}
							onChange={(e) =>
								setState((s) => ({
									...s,
									vaults: patchAt(s.vaults, index, { minBalance: e.target.value }),
								}))
							}
						/>
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
									threshold: "",
									minBalance: "",
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
