import { useRef, useState } from "react"
import { ApiError, api } from "../../api"
import { Field } from "../../components/Field"
import { CheckIcon, CloseIcon, CopyIcon, LockIcon, PenIcon, WalletIcon } from "../../components/InterfaceIcons"
import { PillTabs } from "../../components/PillTabs"
import {
	accountIndexFormatError,
	EVM_PRIVATE_KEY_INVALID_ERROR,
	normalizeHexKey,
	privateKeyFormatError,
	type SignerType,
	secretPhraseCredentials,
	secretPhraseFormatError,
	switchSignerType,
	type WizardState,
} from "../state"
import { StepPoints, type StepPoint } from "../StepPoints"
import type { StepProps } from "../Wizard"

const SIGNER_TABS = [
	{ value: "privateKey", label: "Private key" },
	{ value: "secretPhrase", label: "Secret phrase" },
	{ value: "mpcVault", label: "MPCVault" },
	{ value: "turnkey", label: "Turnkey" },
] as const

/** The step's own status line. The footer leaves the reason to it while a key or phrase is asked for. */
export const SIGNER_STATUS_ID = "signer-status"

/** Why the wizard asks for a wallet, said once above the form. */
const WALLET_USES: ReadonlyArray<StepPoint> = [
	{ icon: WalletIcon, title: "Holds your funds", detail: "Simplex fills orders from this wallet's balance." },
	{ icon: PenIcon, title: "Signs each fill", detail: "Every bid and fill is signed with it." },
	{ icon: LockIcon, title: "Stays on this machine", detail: "What you enter is saved here and nowhere else." },
]

const SIGNER_NOTES: Record<SignerType, string> = {
	privateKey: "Use a wallet made for this solver, not your main one.",
	secretPhrase: "Use a phrase made for this solver, not your main one.",
	mpcVault: "MPCVault signs for you, so no wallet key is kept here.",
	turnkey: "Turnkey signs for you, so no wallet key is kept here.",
}

const STORED_IN_CONFIG = "Your config file keeps it in plain text, and only your user account can read that file."

interface CredentialField<Key> {
	key: Key
	label: string
	type?: "password"
	placeholder?: string
	optional?: boolean
}

const MPC_FIELDS: ReadonlyArray<CredentialField<keyof WizardState["mpcVault"]>> = [
	{ key: "apiToken", label: "API token", type: "password" },
	{ key: "vaultUuid", label: "Vault UUID" },
	{ key: "accountAddress", label: "Wallet address in the vault", placeholder: "0x…" },
	{ key: "callbackClientSignerPublicKey", label: "Callback client-signer public key", placeholder: "ssh-ed25519 …" },
	{ key: "grpcTarget", label: "gRPC target (optional)", placeholder: "api.mpcvault.com:443", optional: true },
]

const TURNKEY_FIELDS: ReadonlyArray<CredentialField<keyof WizardState["turnkey"]>> = [
	{ key: "organizationId", label: "Organization ID" },
	{ key: "apiPublicKey", label: "API public key" },
	{ key: "apiPrivateKey", label: "API private key", type: "password" },
	{ key: "signWith", label: "Wallet address to sign with", placeholder: "0x…" },
]

const SECRET_PHRASE_UNCHECKED_ERROR = "Could not check the secret phrase. Try again."

export function StepSigner({ state, setState }: StepProps) {
	const validationRequest = useRef(0)
	const [accountIndexOpen] = useState(() => state.signerAccountIndex.trim() !== "")
	const [copyError, setCopyError] = useState<string>()
	const [addressCopied, setAddressCopied] = useState(false)

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

	const updateSecretPhrase = (signerPhrase: string, signerAccountIndex: string) => {
		const requestId = ++validationRequest.current
		setAddressCopied(false)
		setCopyError(undefined)
		const formatError = secretPhraseFormatError(signerPhrase) ?? accountIndexFormatError(signerAccountIndex)
		if (formatError) {
			setState((s) => ({
				...s,
				signerPhrase,
				signerAccountIndex,
				signerAddress: undefined,
				signerKeyValidation: signerPhrase.trim() ? "invalid" : "empty",
				signerKeyValidationMessage: formatError,
			}))
			return
		}

		setState((s) => ({
			...s,
			signerPhrase,
			signerAccountIndex,
			signerAddress: undefined,
			signerKeyValidation: "checking",
			signerKeyValidationMessage: undefined,
		}))
		void api
			.post<{ address: string }>(
				"/api/setup/derive-evm-address",
				secretPhraseCredentials(signerPhrase, signerAccountIndex),
			)
			.then(({ address }) => {
				if (requestId !== validationRequest.current) return
				setState((s) => ({
					...s,
					signerAddress: address,
					signerKeyValidation: "valid",
					signerKeyValidationMessage: undefined,
				}))
			})
			.catch((err) => {
				if (requestId !== validationRequest.current) return
				setState((s) => ({
					...s,
					signerAddress: undefined,
					signerKeyValidation: "error",
					signerKeyValidationMessage: err instanceof ApiError ? err.message : SECRET_PHRASE_UNCHECKED_ERROR,
				}))
			})
	}

	const changeSignerType = (signerType: SignerType) => {
		validationRequest.current++
		setAddressCopied(false)
		setCopyError(undefined)
		setState((s) => switchSignerType(s, signerType))
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
	const rejected = validation === "invalid" || validation === "error"

	const fillerAddress = state.signerAddress && (
		<aside className="filler-address-callout" aria-labelledby="filler-address-title">
			<div className="filler-address-heading">
				<span className="filler-address-status" aria-hidden="true">
					<CheckIcon />
				</span>
				<div>
					<strong id="filler-address-title">This is your filler wallet</strong>
					<p>Check it is the address you expect. You fund it in the next step.</p>
				</div>
			</div>
			<div className="filler-address-value">
				<code>{state.signerAddress}</code>
				<button type="button" onClick={copyFillerAddress} aria-live="polite">
					{addressCopied ? <CheckIcon aria-hidden="true" /> : <CopyIcon aria-hidden="true" />}
					{addressCopied ? "Copied" : "Copy"}
				</button>
			</div>
		</aside>
	)

	// One live region for every outcome, so each is announced: checking, the reason, or the address.
	const status = (
		<div id={SIGNER_STATUS_ID} className="signer-status" role="status" aria-live="polite">
			{validation === "checking" && (
				<p className="signer-key-validation signer-key-validation-checking">
					<span className="signer-key-validation-icon" aria-hidden="true">
						<span className="signer-key-validation-spinner" />
					</span>
					Checking…
				</p>
			)}
			{rejected && (
				<p className="signer-key-validation signer-key-validation-invalid">
					<span className="signer-key-validation-icon" aria-hidden="true">
						<CloseIcon />
					</span>
					{state.signerKeyValidationMessage}
				</p>
			)}
			{fillerAddress}
		</div>
	)

	return (
		<div className="wizard-sections signer-step">
			<section className="card signer-card" aria-labelledby="signer-title">
				<StepPoints label="What Simplex uses this wallet for" points={WALLET_USES} />

				<h2 id="signer-title">How should Simplex sign?</h2>
				<PillTabs
					options={SIGNER_TABS}
					value={state.signerType}
					onChange={changeSignerType}
					className="signer-tabs"
					ariaLabel="Filler wallet signing method"
				/>
				<p className="signer-method-note">{SIGNER_NOTES[state.signerType]}</p>

				{state.signerType === "privateKey" && (
					<div className="signer-credentials">
						<Field
							label="Private key"
							type="password"
							required
							value={state.signerKey}
							placeholder="0x…"
							ariaInvalid={rejected}
							onChange={updateSignerKey}
						/>
						{!rejected && <p className="credential-note">{STORED_IN_CONFIG}</p>}
						{status}
					</div>
				)}

				{state.signerType === "secretPhrase" && (
					<div className="signer-credentials">
						<label className="field">
							<span className="field-label">Secret phrase</span>
							<input
								type="password"
								value={state.signerPhrase}
								required
								autoComplete="off"
								autoCorrect="off"
								autoCapitalize="off"
								spellCheck={false}
								aria-invalid={rejected}
								onChange={(e) => updateSecretPhrase(e.target.value, state.signerAccountIndex)}
							/>
						</label>
						{!rejected && (
							<p className="credential-note">12 to 24 words, separated by spaces. {STORED_IN_CONFIG}</p>
						)}
						<details className="credential-note" open={accountIndexOpen}>
							<summary style={{ cursor: "pointer" }}>Advanced: account index</summary>
							<label className="field">
								<span className="field-label">Account index under the phrase (empty for 0)</span>
								<input
									type="text"
									inputMode="numeric"
									value={state.signerAccountIndex}
									placeholder="0"
									autoComplete="off"
									aria-invalid={accountIndexFormatError(state.signerAccountIndex) !== undefined}
									onChange={(e) => updateSecretPhrase(state.signerPhrase, e.target.value)}
								/>
							</label>
							<p>The filler wallet is the account at m/44'/60'/0'/0/index under the phrase.</p>
						</details>
						{status}
					</div>
				)}

				{state.signerType === "mpcVault" && (
					<div className="signer-credentials">
						{MPC_FIELDS.map((field) => (
							<Field
								key={field.key}
								label={field.label}
								type={field.type}
								placeholder={field.placeholder}
								required={!field.optional}
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
								placeholder={field.placeholder}
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
