import { useState } from "react"
import { api } from "../../api"
import { Field } from "../../components/Field"
import { PillTabs } from "../../components/PillTabs"
import { normalizeHexKey, switchNetwork, type SignerType, type WizardState } from "../state"
import type { StepProps } from "../Wizard"

const SIGNER_TABS = [
	{ value: "privateKey", label: "Private key" },
	{ value: "mpcVault", label: "MPCVault" },
	{ value: "turnkey", label: "Turnkey" },
] as const

const MPC_FIELDS: ReadonlyArray<{ key: keyof WizardState["mpcVault"]; label: string; type?: "password" }> = [
	{ key: "apiToken", label: "API token", type: "password" },
	{ key: "vaultUuid", label: "Vault UUID" },
	{ key: "accountAddress", label: "Wallet address in the vault (0x…)" },
	{ key: "callbackClientSignerPublicKey", label: "Callback client-signer public key (ssh-ed25519 …)" },
	{ key: "grpcTarget", label: "gRPC target (optional, defaults to api.mpcvault.com:443)" },
]

const TURNKEY_FIELDS: ReadonlyArray<{ key: keyof WizardState["turnkey"]; label: string; type?: "password" }> = [
	{ key: "organizationId", label: "Organization ID" },
	{ key: "apiPublicKey", label: "API public key" },
	{ key: "apiPrivateKey", label: "API private key", type: "password" },
	{ key: "signWith", label: "Wallet address to sign with (0x…)" },
]

export function StepSigner({ state, setState, defaults }: StepProps) {
	const [error, setError] = useState<string>()

	const deriveAddress = async () => {
		setError(undefined)
		try {
			const { address } = await api.post<{ address: string }>("/api/setup/derive-evm-address", {
				privateKey: normalizeHexKey(state.signerKey),
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
					filler. Holding USDC/USDT is enough: gas is covered by the paymaster. Credentials are written only into
					the local config file (permissions 600).
				</p>
				<PillTabs
					options={SIGNER_TABS}
					value={state.signerType}
					onChange={(signerType: SignerType) => setState((s) => ({ ...s, signerType }))}
				/>

				{state.signerType === "privateKey" && (
					<div>
						<Field
							label="EVM private key (64 hex chars, 0x optional) — simplest; guard the config file"
							type="password"
							value={state.signerKey}
							placeholder="0x…"
							onChange={(signerKey) => setState((s) => ({ ...s, signerKey, signerAddress: undefined }))}
							onBlur={deriveAddress}
						/>
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
						{MPC_FIELDS.map((field) => (
							<Field
								key={field.key}
								label={field.label}
								type={field.type}
								value={state.mpcVault[field.key]}
								onChange={(value) => setState((s) => ({ ...s, mpcVault: { ...s.mpcVault, [field.key]: value } }))}
							/>
						))}
					</div>
				)}

				{state.signerType === "turnkey" && (
					<div>
						<p className="hint">Hosted key management: create the API keypair in the Turnkey dashboard.</p>
						{TURNKEY_FIELDS.map((field) => (
							<Field
								key={field.key}
								label={field.label}
								type={field.type}
								value={state.turnkey[field.key]}
								onChange={(value) => setState((s) => ({ ...s, turnkey: { ...s.turnkey, [field.key]: value } }))}
							/>
						))}
					</div>
				)}
				{error && <p className="error">{error}</p>}
			</div>
		</div>
	)
}
