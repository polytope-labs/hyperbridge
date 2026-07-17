import { useState } from "react"
import { api } from "../../api"
import { switchNetwork } from "../state"
import type { StepProps } from "../Wizard"

export function StepSigner({ state, setState, defaults }: StepProps) {
	const [error, setError] = useState<string>()

	const deriveAddress = async () => {
		setError(undefined)
		try {
			const { address } = await api.post<{ address: string }>("/api/setup/derive-evm-address", {
				privateKey: state.signerKey.trim(),
			})
			setState((s) => ({ ...s, signerAddress: address }))
		} catch (err) {
			setState((s) => ({ ...s, signerAddress: undefined }))
			setError(err instanceof Error ? err.message : String(err))
		}
	}

	return (
		<div>
			<div className="card">
				<h2>Network</h2>
				<p className="hint">Mainnet fills real orders with real funds; testnet uses the Sepolia-family chains.</p>
				<div className="row">
					{(["mainnet", "testnet"] as const).map((network) => (
						<label key={network} className="row">
							<input
								type="radio"
								checked={state.network === network}
								onChange={() => setState((s) => switchNetwork(s, defaults, network))}
							/>
							{network}
						</label>
					))}
				</div>
			</div>

			<div className="card">
				<h2>Filler wallet</h2>
				<p className="hint">
					This wallet signs every fill and holds your stablecoin float on each chain — it is the identity of your
					filler. It needs native gas plus stablecoins on every chain you enable. The key is written only into the
					local config file (permissions 600). MPCVault and Turnkey signers are available via the terminal wizard
					(`simplex init`) or by editing the config file.
				</p>
				<label className="field">
					<span>EVM private key (0x + 64 hex chars)</span>
					<input
						type="password"
						value={state.signerKey}
						onChange={(e) => setState((s) => ({ ...s, signerKey: e.target.value, signerAddress: undefined }))}
						onBlur={deriveAddress}
						placeholder="0x…"
					/>
				</label>
				{state.signerAddress && (
					<p className="hint">
						Filler address: <span className="mono">{state.signerAddress}</span> — confirm this is the wallet you
						intend to fund.
					</p>
				)}
				{error && <p className="error">{error}</p>}
			</div>
		</div>
	)
}
