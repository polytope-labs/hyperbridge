import { VaultRowsEditor } from "../../components/VaultRowsEditor"
import { enabledChains } from "../state"
import type { StepProps } from "../Wizard"

export function StepTreasury({ state, setState, defaults, goToStep }: StepProps) {
	const chains = enabledChains(state)

	return (
		<div className="wizard-sections treasury-step">
			<section className="card treasury-section">
				<div className="treasury-heading">
					<div>
						<span className="market-flow-step">Optional · Treasury automation</span>
						<h2>Put idle liquidity to work</h2>
						<p className="hint">Connect ERC-4626 vaults without taking working capital away from fills.</p>
					</div>
					{state.vaults.length > 0 && (
						<span className="treasury-selected-count">
							{state.vaults.length} {state.vaults.length === 1 ? "vault" : "vaults"} connected
						</span>
					)}
				</div>

				<div className="treasury-flow" role="note" aria-label="How treasury automation works">
					<div>
						<span>01</span>
						<strong>Keep a reserve</strong>
						<small>The minimum balance remains immediately available.</small>
					</div>
					<div>
						<span>02</span>
						<strong>Sweep the excess</strong>
						<small>Funds above your threshold move into the vault.</small>
					</div>
					<div>
						<span>03</span>
						<strong>Redeem for fills</strong>
						<small>Liquidity returns automatically when an order needs it.</small>
					</div>
				</div>

				<VaultRowsEditor
					chains={chains.map((c) => ({ key: c.meta.stateMachineId, label: c.meta.label }))}
					knownVaults={defaults.knownVaults}
					network={state.network}
					onEnableChain={() => goToStep("chains")}
					rows={state.vaults}
					onChange={(vaults) => setState((s) => ({ ...s, vaults }))}
				/>
			</section>
		</div>
	)
}
