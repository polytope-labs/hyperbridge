import { useState } from "react"
import type { SetupDefaults } from "../types"
import { initialState, type WizardState } from "./state"
import { StepSigner } from "./steps/Signer"
import { StepSubstrate } from "./steps/Substrate"
import { StepChains } from "./steps/Chains"
import { StepStrategies } from "./steps/Strategies"
import { StepTreasury } from "./steps/Treasury"
import { StepAdvanced } from "./steps/Advanced"
import { StepReview } from "./steps/Review"

export interface StepProps {
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	defaults: SetupDefaults
}

const STEPS: Array<{ id: string; title: string; component: React.ComponentType<StepProps>; valid: (s: WizardState) => boolean }> = [
	{
		id: "signer",
		title: "Signer",
		component: StepSigner,
		valid: (s) => /^0x[0-9a-fA-F]{64}$/.test(s.signerKey.trim()),
	},
	{
		id: "substrate",
		title: "Hyperbridge account",
		component: StepSubstrate,
		valid: (s) => s.substrateKey.trim().length > 0 && s.hyperbridgeWsUrl.trim().startsWith("ws"),
	},
	{
		id: "chains",
		title: "Chains",
		component: StepChains,
		valid: (s) =>
			s.chains.some((c) => c.enabled) &&
			s.chains.filter((c) => c.enabled).every((c) => c.rpcUrls[0]?.trim() && c.bundlerUrl.trim()),
	},
	{
		id: "strategies",
		title: "Strategies",
		component: StepStrategies,
		valid: (s) => {
			if (!s.stableEnabled && !s.fxEnabled) return false
			if (s.stableEnabled && s.stableBps.filter((p) => p.amount.trim() && p.value.trim()).length < 2) return false
			if (s.fxEnabled) {
				const hasToken = s.chains.some((c) => c.enabled && c.token1.trim())
				const hasCurve =
					(s.fxBidEnabled && s.fxBid.some((p) => p.amount.trim() && p.value.trim())) ||
					(s.fxAskEnabled && s.fxAsk.some((p) => p.amount.trim() && p.value.trim()))
				if (!hasToken || !hasCurve || !(Number(s.fxMaxOrderUsd) > 0)) return false
			}
			return true
		},
	},
	{ id: "treasury", title: "Treasury", component: StepTreasury, valid: () => true },
	{ id: "advanced", title: "Advanced", component: StepAdvanced, valid: () => true },
	{ id: "review", title: "Review & launch", component: StepReview, valid: () => true },
]

export function Wizard(props: { defaults: SetupDefaults }) {
	const [state, setState] = useState<WizardState>(() => initialState(props.defaults))
	const [stepIndex, setStepIndex] = useState(0)

	const step = STEPS[stepIndex]
	const StepComponent = step.component
	const canNext = step.valid(state)

	return (
		<div>
			<h1>Simplex setup</h1>
			<p className="hint">
				This wizard asks for the minimum a filler needs to run, explains why each value matters, and writes a
				commented filler-config.toml before starting. Nothing leaves this machine.
			</p>
			<div className="steps">
				{STEPS.map((s, i) => (
					<span key={s.id} className={`step ${i === stepIndex ? "active" : i < stepIndex ? "done" : ""}`}>
						{i + 1}. {s.title}
					</span>
				))}
			</div>

			<StepComponent state={state} setState={setState} defaults={props.defaults} />

			<div className="footer-nav">
				<button type="button" disabled={stepIndex === 0} onClick={() => setStepIndex((i) => i - 1)}>
					Back
				</button>
				{stepIndex < STEPS.length - 1 && (
					<button type="button" className="primary" disabled={!canNext} onClick={() => setStepIndex((i) => i + 1)}>
						Next
					</button>
				)}
			</div>
		</div>
	)
}
