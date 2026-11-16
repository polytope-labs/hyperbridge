import { useEffect, useState } from "react"
import { validateConfig } from "@/config/filler-toml"
import type { SetupDefaults } from "../types"
import { loadOrderbook } from "./orderbook"
import {
	accountIndexFormatError,
	assembleConfig,
	fillChains,
	initialState,
	privateKeyFormatError,
	secretPhraseFormatError,
	tradeableBooks,
	type WizardState,
} from "./state"
import { SIGNER_STATUS_ID, StepSigner } from "./steps/Signer"
import { StepChains } from "./steps/Chains"
import { LaunchProgress, useLaunch } from "./launch"
import hyperfxLogo from "../assets/hyperfx-logo.webp"
import { InstallAppButton } from "../components/InstallAppButton"

export interface StepProps {
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	defaults: SetupDefaults
}

export function signerRequirements(state: WizardState): string[] {
	if (state.signerType === "privateKey") {
		const formatError = privateKeyFormatError(state.signerKey)
		if (formatError) return [formatError]
		if (state.signerKeyValidation === "valid" && state.signerAddress) return []
		if (state.signerKeyValidation === "checking") return ["Checking the private key…"]
		return [state.signerKeyValidationMessage ?? "Check the private key before continuing."]
	}
	if (state.signerType === "secretPhrase") {
		const formatError =
			secretPhraseFormatError(state.signerPhrase) ?? accountIndexFormatError(state.signerAccountIndex)
		if (formatError) return [formatError]
		if (state.signerKeyValidation === "valid" && state.signerAddress) return []
		if (state.signerKeyValidation === "checking") return ["Checking the secret phrase…"]
		return [state.signerKeyValidationMessage ?? "Check the secret phrase before continuing."]
	}
	if (state.signerType === "mpcVault") {
		const fields = [
			["API token", state.mpcVault.apiToken],
			["vault UUID", state.mpcVault.vaultUuid],
			["wallet address", state.mpcVault.accountAddress],
			["callback client-signer public key", state.mpcVault.callbackClientSignerPublicKey],
		] as const
		return fields.filter(([, value]) => !value.trim()).map(([label]) => `Enter the MPCVault ${label}.`)
	}
	const fields = [
		["organization ID", state.turnkey.organizationId],
		["API public key", state.turnkey.apiPublicKey],
		["API private key", state.turnkey.apiPrivateKey],
		["wallet address", state.turnkey.signWith],
	] as const
	return fields.filter(([, value]) => !value.trim()).map(([label]) => `Enter the Turnkey ${label}.`)
}

function chainRequirements(state: WizardState, defaults: SetupDefaults): string[] {
	// Every chain is watched, so each needs an endpoint whether or not it fills.
	const missingRpc = state.chains
		.filter((chain) => !chain.rpcUrls[0]?.trim())
		.map((chain) => `Enter the RPC endpoint for ${chain.meta.label}.`)
	if (missingRpc.length > 0) return missingRpc
	const filling = fillChains(state)
	if (filling.length === 0) return ["Enable fills on at least one chain."]
	const unfunded = filling
		.filter((chain) => !chain.funding?.funded)
		.map((chain) => `Fund the wallet on ${chain.meta.label}, or turn its fills off.`)
	if (unfunded.length > 0) return unfunded
	// The markets come from the orderbook's books, so a chain selection that can
	// carry none of them would boot a filler with nothing to trade.
	if (state.orderbookError) return [state.orderbookError]
	if (!state.orderbook) return ["Reading the orderbook's markets…"]
	if (tradeableBooks(state, defaults).length === 0) {
		const books = state.orderbook.books.map((book) => `${book.base}/${book.quote}`).join(", ")
		return [`None of the orderbook's markets (${books || "none listed"}) trade on the enabled chains.`]
	}
	return []
}

/**
 * What the last step needs before it can start the solver: its chains, then the
 * whole config through the same check the server gate runs.
 */
function launchRequirements(state: WizardState, defaults: SetupDefaults): string[] {
	const chains = chainRequirements(state, defaults)
	if (chains.length > 0) return chains
	try {
		validateConfig(assembleConfig(state, defaults))
		return []
	} catch (error) {
		return [error instanceof Error ? error.message : "Check the configuration."]
	}
}

const STEPS: Array<{
	id: string
	title: string
	description: string
	component: React.ComponentType<StepProps>
	requirements: (s: WizardState, defaults: SetupDefaults) => string[]
	/** The step's own status line, while it says beside the field what is still needed. The footer then stays quiet. */
	statusId?: (s: WizardState) => string | undefined
}> = [
	{
		id: "signer",
		title: "Wallet",
		description: "Simplex fills orders with your funds, so it needs a wallet to work from.",
		component: StepSigner,
		requirements: signerRequirements,
		statusId: (s) =>
			s.signerType === "privateKey" || s.signerType === "secretPhrase" ? SIGNER_STATUS_ID : undefined,
	},
	{
		id: "chains",
		title: "Chains",
		description: "Simplex watches every chain for orders. It fills on the ones you fund.",
		component: StepChains,
		requirements: launchRequirements,
	},
]

export function Wizard(props: { defaults: SetupDefaults }) {
	const [state, setState] = useState<WizardState>(() => initialState(props.defaults))
	const [stepIndex, setStepIndex] = useState(0)

	useEffect(() => {
		void loadOrderbook(setState)
	}, [])

	const launch = useLaunch(state, props.defaults)

	const step = STEPS[stepIndex]
	const StepComponent = step.component
	const lastStep = stepIndex === STEPS.length - 1
	const requirements = step.requirements(state, props.defaults)
	const canNext = requirements.length === 0
	const statusId = step.statusId?.(state)
	const listed = statusId ? [] : requirements
	const requirementsId = `wizard-${step.id}-requirements`
	const blockedById = statusId ?? requirementsId
	const progress = ((stepIndex + 1) / STEPS.length) * 100

	return (
		<div className="wizard-shell">
			<header className="wizard-brandbar">
				<div className="wizard-brand">
					<img className="hyperfx-logo" src={hyperfxLogo} alt="HyperFX" />
					<span className="wizard-product-name">Simplex</span>
				</div>
				<div className="wizard-brand-actions">
					<InstallAppButton />
					<div className="wizard-local-status">
						<span aria-hidden="true" />
						Local setup
					</div>
				</div>
			</header>

			<div className="wizard-layout">
				<aside className="wizard-sidebar">
					<div className="wizard-intro">
						<span className="eyebrow">Solver onboarding</span>
						<h1>Configure Simplex</h1>
						<p>Set up execution, liquidity, and risk controls for your Hyperbridge solver.</p>
					</div>

					<nav className="stepper" aria-label="Setup progress">
						<ol>
							{STEPS.map((s, i) => {
								const stateName = i === stepIndex ? "active" : i < stepIndex ? "done" : "upcoming"
								return (
									<li key={s.id} className="step" data-state={stateName}>
										<span className="step-index" aria-hidden="true">
											{i < stepIndex ? "✓" : String(i + 1).padStart(2, "0")}
										</span>
										<span className="step-copy">
											<strong>{s.title}</strong>
											<small>
												{i === stepIndex
													? "In progress"
													: i < stepIndex
														? "Complete"
														: "Upcoming"}
											</small>
										</span>
									</li>
								)
							})}
						</ol>
					</nav>

					<div className="wizard-security-note">
						<span className="wizard-security-icon" aria-hidden="true">
							◆
						</span>
						<div>
							<strong>Local by design</strong>
							<p>
								Credentials remain on this machine and are written only to your protected config file.
							</p>
						</div>
					</div>
				</aside>

				<section className="wizard-main" aria-labelledby="wizard-step-title">
					<header className="wizard-step-header">
						<div className="wizard-progress-meta">
							<span className="eyebrow">
								Step {stepIndex + 1} of {STEPS.length}
							</span>
							<span className="wizard-progress-value">{Math.round(progress)}%</span>
						</div>
						<progress
							className="wizard-progress-track"
							max={100}
							value={progress}
							aria-label={`Setup ${Math.round(progress)}% complete`}
						/>
						<h2 id="wizard-step-title">{step.title}</h2>
						<p>{step.description}</p>
					</header>

					<div className="wizard-step-content">
						{launch.phase === "starting" ? (
							<LaunchProgress />
						) : (
							<StepComponent state={state} setState={setState} defaults={props.defaults} />
						)}
						{launch.phase === "failed" && (
							<div className="card" role="alert">
								<p className="error">{launch.error}</p>
								{launch.saved && (
									<p className="hint">Your configuration is saved. Correct it above, then retry.</p>
								)}
							</div>
						)}
					</div>

					{launch.phase !== "starting" && (
						<footer className="footer-nav">
							{stepIndex > 0 ? (
								<button
									type="button"
									className="secondary"
									disabled={launch.phase === "saving"}
									onClick={() => setStepIndex((i) => i - 1)}
								>
									<span aria-hidden="true">←</span> Back
								</button>
							) : (
								<span aria-hidden="true" />
							)}
							{listed.length > 0 ? (
								<div
									className="wizard-requirements"
									id={requirementsId}
									role="status"
									aria-live="polite"
								>
									<span className="wizard-requirements-icon" aria-hidden="true">
										!
									</span>
									<div>
										<strong>{lastStep ? "Required to start" : "Required to continue"}</strong>
										<ul>
											{listed.map((requirement) => (
												<li key={requirement}>{requirement}</li>
											))}
										</ul>
									</div>
								</div>
							) : null}
							{lastStep ? (
								<button
									type="button"
									className="primary"
									disabled={!canNext || launch.phase === "saving"}
									aria-describedby={!canNext ? blockedById : undefined}
									onClick={launch.start}
								>
									{launch.phase === "saving"
										? "Saving configuration…"
										: launch.phase === "failed"
											? "Retry startup"
											: "Save config & start Simplex"}
									{launch.phase !== "saving" && <span aria-hidden="true"> →</span>}
								</button>
							) : (
								<button
									type="button"
									className="primary"
									disabled={!canNext}
									aria-describedby={!canNext ? blockedById : undefined}
									onClick={() => setStepIndex((i) => i + 1)}
								>
									Continue <span aria-hidden="true">→</span>
								</button>
							)}
						</footer>
					)}
				</section>
			</div>
		</div>
	)
}
