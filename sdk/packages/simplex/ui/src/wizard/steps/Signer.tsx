import { useRef, useState } from "react"
import { api } from "../../api"
import { Field } from "../../components/Field"
import { CheckIcon, CloseIcon, CopyIcon } from "../../components/InterfaceIcons"
import { PillTabs } from "../../components/PillTabs"
import {
	EVM_PRIVATE_KEY_INVALID_ERROR,
	normalizeHexKey,
	privateKeyFormatError,
	type SignerType,
	type WizardState,
} from "../state"
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

export function StepSigner({ state, setState }: StepProps) {
	const validationRequest = useRef(0)
	const [copyError, setCopyError] = useState<string>()
	const [addressCopied, setAddressCopied] = useState(false)
	const signerDescription = SIGNER_DESCRIPTIONS[state.signerType]

	const updateSignerKey = (signerKey: string) => {
		const requestId = ++validationRequest.current
		setAddressCopied(false)
		setCopyError(undefined)
		const formatError = privateKeyFormatError(signerKey)
		if (formatError) {
			setState((s) => ({
				...s,
				signerKey,
				signerAddress: undefined,
				signerKeyValidation: signerKey.trim() ? "invalid" : "empty",
				signerKeyValidationMessage: formatError,
			}))
			return
		}

		setState((s) => ({
			...s,
			signerKey,
			signerAddress: undefined,
			signerKeyValidation: "checking",
			signerKeyValidationMessage: undefined,
		}))
		void api
			.post<{ address: string }>("/api/setup/derive-evm-address", {
				privateKey: normalizeHexKey(signerKey),
			})
			.then(({ address }) => {
				if (requestId !== validationRequest.current) return
				setState((s) => ({
					...s,
					signerAddress: address,
					signerKeyValidation: "valid",
					signerKeyValidationMessage: undefined,
				}))
			})
			.catch(() => {
				if (requestId !== validationRequest.current) return
				setState((s) => ({
					...s,
					signerAddress: undefined,
					signerKeyValidation: "error",
					signerKeyValidationMessage: EVM_PRIVATE_KEY_INVALID_ERROR,
				}))
			})
	}

	const copyFillerAddress = () => {
		if (!state.signerAddress) return
		void navigator.clipboard
			.writeText(state.signerAddress)
			.then(() => {
				setAddressCopied(true)
				window.setTimeout(() => setAddressCopied(false), 1600)
			})
			.catch(() => setCopyError("Could not copy the filler wallet address. Select it and copy it manually."))
	}

	const validation = state.signerKeyValidation ?? "empty"
	const validationMessage =
		state.signerKeyValidationMessage ??
		(validation === "checking"
			? "Checking the EVM private key…"
			: validation === "valid"
				? "EVM private key is valid."
				: "Enter the EVM private key.")

	return (
		<div className="wizard-sections signer-step">
			<section className="card signer-card" aria-labelledby="signer-title">
				<div className="card-heading">
					<h2 id="signer-title">Filler wallet</h2>
				</div>
				<p className="hint">
					This wallet signs fills and holds the funds used for orders. Its credentials stay private on this
					machine.
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
							ariaInvalid={validation === "invalid" || validation === "error"}
							onChange={updateSignerKey}
						/>
						<div className={`signer-key-validation signer-key-validation-${validation}`} role="status" aria-live="polite">
							<span className="signer-key-validation-icon" aria-hidden="true">
								{validation === "valid" ? (
									<CheckIcon />
								) : validation === "checking" ? (
									<span className="signer-key-validation-spinner" />
								) : validation === "empty" ? (
									<span className="signer-key-validation-dot" />
								) : (
									<CloseIcon />
								)}
							</span>
							<span>{validationMessage}</span>
						</div>
						<p className="credential-note">
							Use a dedicated wallet and keep the generated config file protected. The key is checked by the local
							Simplex server and never persisted by validation.
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
				{copyError && <p className="error">{copyError}</p>}
			</section>
		</div>
	)
}
