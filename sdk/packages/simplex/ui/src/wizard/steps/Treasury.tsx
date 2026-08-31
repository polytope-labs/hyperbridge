import { VaultRowsEditor } from "../../components/VaultRowsEditor"
import { enabledChains } from "../state"
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

				<VaultRowsEditor
					chains={chains.map((c) => ({ key: c.meta.stateMachineId, label: c.meta.label }))}
					knownVaults={defaults.knownVaults}
					rows={state.vaults}
					onChange={(vaults) => setState((s) => ({ ...s, vaults }))}
				/>
			</div>
		</div>
	)
}
