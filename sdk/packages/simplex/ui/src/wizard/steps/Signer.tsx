import { useState } from "react"
import { api } from "../../api"
import { Field } from "../../components/Field"
import { CheckIcon, CopyIcon } from "../../components/InterfaceIcons"
import { PillTabs } from "../../components/PillTabs"
import { normalizeHexKey, switchNetwork, type SignerType, type WizardState } from "../state"
import type { StepProps } from "../Wizard"

const SIGNER_TABS = [
	{ value: "privateKey", label: "Private key" },
	{ value: "mpcVault", label: "MPCVault" },
	{ value: "turnkey", label: "Turnkey" },
] as const

const SIGNER_DESCRIPTIONS: Record<SignerType, { title: string; description: string }> = {
	privateKey: {
		title: "Direct wallet control",
		description:
			"Use a dedicated EVM wallet. The encrypted local config is the only place this credential is stored.",
	},
	mpcVault: {
		title: "Institutional MPC custody",
		description:
			"Connect your MPCVault account and registered client signer to authorize fills without exposing a private key.",
	},
	turnkey: {
		title: "Hosted key management",
		description: "Connect an API keypair created in your Turnkey dashboard and select the wallet that signs fills.",
	},
}

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
	const [addressCopied, setAddressCopied] = useState(false)
	const signerDescription = SIGNER_DESCRIPTIONS[state.signerType]

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

	const copyFillerAddress = () => {
		if (!state.signerAddress) return
		void navigator.clipboard
			.writeText(state.signerAddress)
			.then(() => {
				setAddressCopied(true)
				window.setTimeout(() => setAddressCopied(false), 1600)
			})
			.catch(() => setError("Could not copy the filler wallet address. Select it and copy it manually."))
	}

	return (
		<div className="wizard-sections signer-step">
			<section className="card network-card" aria-labelledby="network-title">
				<div className="card-heading">
					<h2 id="network-title">Network</h2>
				</div>
				<p className="hint">
					Mainnet fills real orders with real funds; testnet uses the Sepolia-family chains.
				</p>
				<div className="network-options">
					{(["mainnet", "testnet"] as const).map((network) => (
						<label key={network} className="network-option" data-active={state.network === network}>
							<input
								type="radio"
								name="network"
								checked={state.network === network}
								onChange={() => setState((s) => switchNetwork(s, defaults, network))}
							/>
							<span className="network-option-radio" aria-hidden="true" />
							<span className="network-option-copy">
								<strong>{network === "mainnet" ? "Mainnet" : "Testnet"}</strong>
								<span>
									{network === "mainnet" ? "Live funds and orders" : "Sepolia-family networks"}
								</span>
							</span>
							<span className={`badge ${network === "mainnet" ? "ok" : ""}`}>
								{network === "mainnet" ? "Live" : "Sandbox"}
							</span>
						</label>
					))}
				</div>
			</section>

			<section className="card signer-card" aria-labelledby="signer-title">
				<div className="card-heading">
					<h2 id="signer-title">Filler wallet</h2>
				</div>
				<p className="hint">
					This wallet signs every fill and holds your stablecoin float on each chain — it is the identity of
					your filler. Holding USDC/USDT is enough: gas is covered by the paymaster. Credentials are written
					only into the local config file (permissions 600).
				</p>
				<PillTabs
					options={SIGNER_TABS}
					value={state.signerType}
					onChange={(signerType: SignerType) => setState((s) => ({ ...s, signerType }))}
					className="signer-tabs"
					ariaLabel="Filler wallet signing method"
				/>
				<div className="signer-method-summary">
					<div>
						<strong>{signerDescription.title}</strong>
						<p>{signerDescription.description}</p>
					</div>
				</div>

				{state.signerType === "privateKey" && (
					<div className="signer-credentials">
						<Field
							label="EVM private key (64 hex characters; 0x optional)"
							type="password"
							required
							value={state.signerKey}
							placeholder="0x…"
							onChange={(signerKey) => {
								setAddressCopied(false)
								setState((s) => ({ ...s, signerKey, signerAddress: undefined }))
							}}
							onBlur={deriveAddress}
						/>
						<p className="credential-note">
							Use a dedicated wallet and keep the generated config file protected.
						</p>
						{state.signerAddress && (
							<aside className="filler-address-callout" aria-labelledby="filler-address-title">
								<div className="filler-address-heading">
									<span className="filler-address-status" aria-hidden="true">
										<CheckIcon />
									</span>
									<div>
										<strong id="filler-address-title">Filler wallet address</strong>
										<p>Confirm this is the wallet you intend to fund.</p>
									</div>
								</div>
								<div className="filler-address-value">
									<code>{state.signerAddress}</code>
									<button type="button" onClick={copyFillerAddress} aria-live="polite">
										{addressCopied ? (
											<CheckIcon aria-hidden="true" />
										) : (
											<CopyIcon aria-hidden="true" />
										)}
										{addressCopied ? "Copied" : "Copy"}
									</button>
								</div>
							</aside>
						)}
					</div>
				)}

				{state.signerType === "mpcVault" && (
					<div className="signer-credentials">
						{MPC_FIELDS.map((field) => (
							<Field
								key={field.key}
								label={field.label}
								type={field.type}
								required={field.key !== "grpcTarget"}
								value={state.mpcVault[field.key]}
								onChange={(value) =>
									setState((s) => ({ ...s, mpcVault: { ...s.mpcVault, [field.key]: value } }))
								}
							/>
						))}
					</div>
				)}

				{state.signerType === "turnkey" && (
					<div className="signer-credentials">
						{TURNKEY_FIELDS.map((field) => (
							<Field
								key={field.key}
								label={field.label}
								type={field.type}
								required
								value={state.turnkey[field.key]}
								onChange={(value) =>
									setState((s) => ({ ...s, turnkey: { ...s.turnkey, [field.key]: value } }))
								}
							/>
						))}
					</div>
				)}
				{error && <p className="error">{error}</p>}
			</section>
		</div>
	)
}
