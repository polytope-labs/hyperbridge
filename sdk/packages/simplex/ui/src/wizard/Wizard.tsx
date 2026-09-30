import { useEffect, useState } from "react"
import { validateConfig } from "@/config/filler-toml"
import type { SetupDefaults } from "../types"
import { loadOrderbook } from "./orderbook"
import { assembleConfig, initialState, orderbookPairs, privateKeyFormatError, type WizardState } from "./state"
import { StepSigner } from "./steps/Signer"
import { StepSubstrate } from "./steps/Substrate"
import { StepChains } from "./steps/Chains"
import { StepTreasury } from "./steps/Treasury"
import { StepAdvanced } from "./steps/Advanced"
import { StepReview } from "./steps/Review"
import hyperfxLogo from "../assets/hyperfx-logo.webp"
import { InstallAppButton } from "../components/InstallAppButton"

export interface StepProps {
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	defaults: SetupDefaults
	/** Jumps to another step by id (e.g. "chains"); unknown ids are ignored. */
	goToStep: (id: string) => void
}

function signerRequirements(state: WizardState): string[] {
	if (state.signerType === "privateKey") {
		const formatError = privateKeyFormatError(state.signerKey)
		if (formatError) return [formatError]
		if (state.signerKeyValidation === "valid" && state.signerAddress) return []
		if (state.signerKeyValidation === "checking") return ["Checking the EVM private key…"]
		return [state.signerKeyValidationMessage ?? "Verify the EVM private key before continuing."]
	}
	if (state.signerType === "mpcVault") {
		const fields = [
			["API token", state.mpcVault.apiToken],
			["Vault UUID", state.mpcVault.vaultUuid],
			["Wallet address", state.mpcVault.accountAddress],
			["Callback client-signer public key", state.mpcVault.callbackClientSignerPublicKey],
		] as const
		return fields
			.filter(([, value]) => !value.trim())
			.map(([label]) => `Enter the MPCVault ${label.toLowerCase()}.`)
	}
	const fields = [
		["Organization ID", state.turnkey.organizationId],
		["API public key", state.turnkey.apiPublicKey],
		["API private key", state.turnkey.apiPrivateKey],
		["Wallet address", state.turnkey.signWith],
	] as const
	return fields.filter(([, value]) => !value.trim()).map(([label]) => `Enter the Turnkey ${label.toLowerCase()}.`)
}

function substrateRequirements(state: WizardState): string[] {
	const issues: string[] = []
	if (!state.substrateKey.trim()) issues.push("Generate an account or enter an existing recovery phrase or hex seed.")
	if (!state.hyperbridgeWsUrl.trim().startsWith("ws")) {
		issues.push("Enter a Hyperbridge WebSocket URL beginning with ws:// or wss://.")
	}
	return issues
}

function chainRequirements(state: WizardState, defaults: SetupDefaults): string[] {
	const enabled = state.chains.filter((chain) => chain.enabled)
	if (enabled.length === 0) return ["Enable fills for at least one chain."]
	const issues = enabled.flatMap((chain) => {
		const missing: string[] = []
		if (!chain.rpcUrls[0]?.trim()) missing.push(`Enter the RPC endpoint for ${chain.meta.label}.`)
		if (!chain.bundlerUrl.trim()) missing.push(`Enter the bundler endpoint for ${chain.meta.label}.`)
		return missing
	})
	if (issues.length > 0) return issues
	// The markets come from the orderbook's books, so a chain selection that can
	// carry none of them would boot a filler with nothing to trade.
	if (state.orderbookError) return [state.orderbookError]
	if (!state.orderbook) return ["Reading the orderbook's markets…"]
	if (orderbookPairs(state, defaults).length === 0) {
		const books = state.orderbook.books.map((book) => `${book.base}/${book.quote}`).join(", ")
		return [`None of the orderbook's markets (${books || "none listed"}) trade on the enabled chains.`]
	}
	return []
}

/** The last word before launch: the whole config, through the same check the server gate runs. */
function reviewRequirements(state: WizardState, defaults: SetupDefaults): string[] {
	try {
		validateConfig(assembleConfig(state, defaults))
		return []
	} catch (error) {
		return [error instanceof Error ? error.message : "Review the configuration."]
	}
}

const STEPS: Array<{
	id: string
	title: string
	description: string
	component: React.ComponentType<StepProps>
	requirements: (s: WizardState, defaults: SetupDefaults) => string[]
}> = [
	{
		id: "signer",
		title: "Signer",
		description: "Choose the wallet infrastructure that will identify Simplex and authorize every fill.",
		component: StepSigner,
		requirements: signerRequirements,
	},
	{
		id: "substrate",
		title: "Hyperbridge account",
		description: "Connect the account Simplex uses to submit bids and settle execution fees on Hyperbridge.",
		component: StepSubstrate,
		requirements: substrateRequirements,
	},
	{
		id: "chains",
		title: "Chains",
		description: "Select execution networks and verify the RPC and bundler infrastructure behind each one.",
		component: StepChains,
		requirements: chainRequirements,
	},
	{
		id: "treasury",
		title: "Treasury",
		description: "Optionally connect vaults that keep idle liquidity productive and available for fills.",
		component: StepTreasury,
		requirements: () => [],
	},
	{
		id: "advanced",
		title: "Advanced",
		description: "Tune concurrency, logging, and access policy—or keep the safe defaults.",
		component: StepAdvanced,
		requirements: () => [],
	},
	{
		id: "review",
		title: "Review & launch",
		description: "Confirm the accounts and generated configuration before starting the solver.",
		component: StepReview,
		requirements: reviewRequirements,
	},
]

export function Wizard(props: { defaults: SetupDefaults }) {
	const [state, setState] = useState<WizardState>(() => initialState(props.defaults))
	const [stepIndex, setStepIndex] = useState(0)

	useEffect(() => {
		void loadOrderbook(setState)
	}, [])

	const step = STEPS[stepIndex]
	const StepComponent = step.component
	const goToStep = (id: string) => {
		const index = STEPS.findIndex((candidate) => candidate.id === id)
		if (index >= 0) setStepIndex(index)
	}
	const requirements = step.requirements(state, props.defaults)
	const canNext = requirements.length === 0
	const requirementsId = `wizard-${step.id}-requirements`
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
						<StepComponent state={state} setState={setState} defaults={props.defaults} goToStep={goToStep} />
					</div>

					<footer className="footer-nav">
						{stepIndex > 0 ? (
							<button type="button" className="secondary" onClick={() => setStepIndex((i) => i - 1)}>
								<span aria-hidden="true">←</span> Back
							</button>
						) : (
							<span aria-hidden="true" />
						)}
						{stepIndex < STEPS.length - 1 && requirements.length > 0 ? (
							<div className="wizard-requirements" id={requirementsId} role="status" aria-live="polite">
								<span className="wizard-requirements-icon" aria-hidden="true">
									!
								</span>
								<div>
									<strong>Required to continue</strong>
									<ul>
										{requirements.map((requirement) => (
											<li key={requirement}>{requirement}</li>
										))}
									</ul>
								</div>
							</div>
						) : null}
						{stepIndex < STEPS.length - 1 && (
							<button
								type="button"
								className="primary"
								disabled={!canNext}
								aria-describedby={!canNext ? requirementsId : undefined}
								onClick={() => setStepIndex((i) => i + 1)}
							>
								Continue <span aria-hidden="true">→</span>
							</button>
						)}
					</footer>
				</section>
			</div>
		</div>
	)
}
