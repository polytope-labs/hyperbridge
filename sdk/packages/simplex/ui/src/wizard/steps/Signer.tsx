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
					filler. It needs native gas plus stablecoins on every chain you enable. Credentials are written only
					into the local config file (permissions 600).
				</p>
				<div className="steps">
					{(
						[
							["privateKey", "Private key"],
							["mpcVault", "MPCVault"],
							["turnkey", "Turnkey"],
						] as const
					).map(([type, label]) => (
						<button
							key={type}
							type="button"
							className={`step ${state.signerType === type ? "active" : ""}`}
							style={{ cursor: "pointer" }}
							onClick={() => setState((s) => ({ ...s, signerType: type }))}
						>
							{label}
						</button>
					))}
				</div>

				{state.signerType === "privateKey" && (
					<div>
						<label className="field">
							<span>EVM private key (0x + 64 hex chars) — simplest; guard the config file</span>
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
								Filler address: <span className="mono">{state.signerAddress}</span> — confirm this is the wallet
								you intend to fund.
							</p>
						)}
					</div>
				)}

				{state.signerType === "mpcVault" && (
					<div>
						<p className="hint">
							Institutional MPC custody: needs a vault, an Ed25519 keypair registered in the MPCVault console,
							and a running client-signer container.
						</p>
						<label className="field">
							<span>API token</span>
							<input
								type="password"
								value={state.mpcVault.apiToken}
								onChange={(e) => setState((s) => ({ ...s, mpcVault: { ...s.mpcVault, apiToken: e.target.value } }))}
							/>
						</label>
						<label className="field">
							<span>Vault UUID</span>
							<input
								type="text"
								value={state.mpcVault.vaultUuid}
								onChange={(e) => setState((s) => ({ ...s, mpcVault: { ...s.mpcVault, vaultUuid: e.target.value } }))}
							/>
						</label>
						<label className="field">
							<span>Wallet address in the vault (0x…)</span>
							<input
								type="text"
								value={state.mpcVault.accountAddress}
								onChange={(e) =>
									setState((s) => ({ ...s, mpcVault: { ...s.mpcVault, accountAddress: e.target.value } }))
								}
							/>
						</label>
						<label className="field">
							<span>Callback client-signer public key (ssh-ed25519 …)</span>
							<input
								type="text"
								value={state.mpcVault.callbackClientSignerPublicKey}
								onChange={(e) =>
									setState((s) => ({
										...s,
										mpcVault: { ...s.mpcVault, callbackClientSignerPublicKey: e.target.value },
									}))
								}
							/>
						</label>
						<label className="field">
							<span>gRPC target (optional, defaults to api.mpcvault.com:443)</span>
							<input
								type="text"
								value={state.mpcVault.grpcTarget}
								onChange={(e) => setState((s) => ({ ...s, mpcVault: { ...s.mpcVault, grpcTarget: e.target.value } }))}
							/>
						</label>
					</div>
				)}

				{state.signerType === "turnkey" && (
					<div>
						<p className="hint">Hosted key management: create the API keypair in the Turnkey dashboard.</p>
						<label className="field">
							<span>Organization ID</span>
							<input
								type="text"
								value={state.turnkey.organizationId}
								onChange={(e) =>
									setState((s) => ({ ...s, turnkey: { ...s.turnkey, organizationId: e.target.value } }))
								}
							/>
						</label>
						<label className="field">
							<span>API public key</span>
							<input
								type="text"
								value={state.turnkey.apiPublicKey}
								onChange={(e) => setState((s) => ({ ...s, turnkey: { ...s.turnkey, apiPublicKey: e.target.value } }))}
							/>
						</label>
						<label className="field">
							<span>API private key</span>
							<input
								type="password"
								value={state.turnkey.apiPrivateKey}
								onChange={(e) =>
									setState((s) => ({ ...s, turnkey: { ...s.turnkey, apiPrivateKey: e.target.value } }))
								}
							/>
						</label>
						<label className="field">
							<span>Wallet address to sign with (0x…)</span>
							<input
								type="text"
								value={state.turnkey.signWith}
								onChange={(e) => setState((s) => ({ ...s, turnkey: { ...s.turnkey, signWith: e.target.value } }))}
							/>
						</label>
					</div>
				)}
				{error && <p className="error">{error}</p>}
			</div>
		</div>
	)
}
