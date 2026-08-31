import { useState } from "react"
import { isRegistrySymbol } from "@/config/asset-registry"
import { validateConfig } from "@/config/filler-toml"
import type { SetupDefaults } from "../types"
import { assembleConfig, initialState, normSymbol, type WizardState } from "./state"
import { StepSigner } from "./steps/Signer"
import { StepSubstrate } from "./steps/Substrate"
import { StepChains } from "./steps/Chains"
import { StepStrategies } from "./steps/Strategies"
import { StepTreasury } from "./steps/Treasury"
import { StepAdvanced } from "./steps/Advanced"
import { StepReview } from "./steps/Review"
import hyperbridgeLogo from "../assets/hyperbridge-logo.svg"

export interface StepProps {
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	defaults: SetupDefaults
}

function signerRequirements(state: WizardState): string[] {
	if (state.signerType === "privateKey") {
		if (!state.signerKey.trim()) return ["Enter the EVM private key."]
		return /^(0x)?[0-9a-fA-F]{64}$/.test(state.signerKey.trim())
			? []
			: ["Enter a valid 64-character hexadecimal EVM private key."]
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

function chainRequirements(state: WizardState): string[] {
	const enabled = state.chains.filter((chain) => chain.enabled)
	if (enabled.length === 0) return ["Enable fills for at least one chain."]
	return enabled.flatMap((chain) => {
		const issues: string[] = []
		if (!chain.rpcUrls[0]?.trim()) issues.push(`Enter the RPC endpoint for ${chain.meta.label}.`)
		if (!chain.bundlerUrl.trim()) issues.push(`Enter the bundler endpoint for ${chain.meta.label}.`)
		return issues
	})
}

function marketRequirements(state: WizardState, defaults: SetupDefaults): string[] {
	const enabled = state.pairs.filter((pair) => pair.enabled)
	if (enabled.length === 0) return ["Enable or add at least one market."]
	const enabledChainDrafts = state.chains.filter((chain) => chain.enabled)
	const availableOnEnabled = new Set(
		enabledChainDrafts.flatMap((chain) =>
			(defaults.knownTokens[chain.meta.stateMachineId] ?? []).map((token) => normSymbol(token.symbol)),
		),
	)
	const enabledChainIds = new Set(enabledChainDrafts.map((chain) => chain.meta.stateMachineId))
	for (const pair of enabled) {
		for (const symbol of [pair.token0, pair.token1]) {
			if (!symbol.trim()) return ["Choose both assets for every enabled market."]
			if (isRegistrySymbol(symbol)) {
				if (state.customAssets[symbol])
					return [`Select ${symbol} from the asset list instead of defining it as custom.`]
				if (!availableOnEnabled.has(normSymbol(symbol))) {
					return [`${symbol} is not available on any enabled chain.`]
				}
			} else {
				const addresses = state.customAssets[symbol]
				if (
					!addresses ||
					!Object.entries(addresses).some(([chain, address]) => enabledChainIds.has(chain) && address.trim())
				) {
					return [`Enter a contract address for custom asset ${symbol} on an enabled chain.`]
				}
			}
		}
	}
	if (
		state.fxPricing === "uniswapV4" &&
		!state.fxPositions.every((position) => /^\d+$/.test(position.tokenId.trim()))
	) {
		return ["Enter a numeric Uniswap V4 position token ID for every position."]
	}
	try {
		validateConfig(assembleConfig(state, defaults))
	} catch (error) {
		return [error instanceof Error ? error.message : "Review the enabled market settings."]
	}
	return []
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
		id: "strategies",
		title: "Markets",
		description: "Define the asset pairs, pricing curves, and liquidity sources Simplex is allowed to serve.",
		component: StepStrategies,
		requirements: marketRequirements,
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
		requirements: () => [],
	},
]

export function Wizard(props: { defaults: SetupDefaults }) {
	const [state, setState] = useState<WizardState>(() => initialState(props.defaults))
	const [stepIndex, setStepIndex] = useState(0)

	const step = STEPS[stepIndex]
	const StepComponent = step.component
	const requirements = step.requirements(state, props.defaults)
	const canNext = requirements.length === 0
	const requirementsId = `wizard-${step.id}-requirements`
	const progress = ((stepIndex + 1) / STEPS.length) * 100

	return (
		<div className="wizard-shell">
			<header className="wizard-brandbar">
				<div className="wizard-brand">
					<img src={hyperbridgeLogo} alt="Hyperbridge" />
					<span className="wizard-product-name">Simplex</span>
				</div>
				<div className="wizard-local-status">
					<span aria-hidden="true" />
					Local setup
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
						<StepComponent state={state} setState={setState} defaults={props.defaults} />
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
