import { Fragment, useState } from "react"
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
		valid: (s) => {
			if (s.signerType === "privateKey") return /^(0x)?[0-9a-fA-F]{64}$/.test(s.signerKey.trim())
			if (s.signerType === "mpcVault") {
				const v = s.mpcVault
				return Boolean(
					v.apiToken.trim() && v.vaultUuid.trim() && v.accountAddress.trim() && v.callbackClientSignerPublicKey.trim(),
				)
			}
			const t = s.turnkey
			return Boolean(t.organizationId.trim() && t.apiPublicKey.trim() && t.apiPrivateKey.trim() && t.signWith.trim())
		},
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
		title: "Markets",
		component: StepStrategies,
		valid: (s) => {
			const enabled = s.pairs.filter((p) => p.enabled)
			if (enabled.length === 0) return false
			// Mirrors FillerPricePolicy / validatePairConfigs; the server gate is authoritative.
			const validCurve = (points: (typeof s.pairs)[number]["ask"], check: (v: number) => boolean) => {
				const filled = points.filter((p) => p.amount.trim() && p.value.trim())
				return filled.length > 0 && filled.every((p) => Number(p.amount) >= 0 && check(Number(p.value)))
			}
			for (const pair of enabled) {
				if (!(Number(pair.maxOrderSize) > 0)) return false
				if (pair.kind === "sameAsset") {
					// Same-token asks must sit strictly below par — the gap is the spread.
					if (!validCurve(pair.ask, (v) => v > 0 && v < 1)) return false
					continue
				}
				if (!pair.token1.trim()) return false
				const customAddresses = s.customAssets[pair.token1]
				if (customAddresses && !Object.values(customAddresses).some((a) => a.trim())) return false
				if (s.fxPricing === "curves") {
					if (!pair.bidEnabled && !pair.askEnabled) return false
					if (pair.bidEnabled && !validCurve(pair.bid, (v) => v > 0)) return false
					if (pair.askEnabled && !validCurve(pair.ask, (v) => v > 0)) return false
				}
			}
			const hasCrossAsset = enabled.some((p) => p.kind === "crossAsset")
			if (hasCrossAsset && s.fxPricing === "uniswapV4") {
				const positionsOk =
					s.fxPositions.length > 0 &&
					s.fxPositions.every(
						(p) =>
							/^\d+$/.test(p.tokenId.trim()) &&
							// price guard fields must be set together
							Boolean(p.referencePrice.trim()) === Boolean(p.maxDeviationBps.trim()),
					)
				if (!positionsOk) return false
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
			<div className="stepper">
				{STEPS.map((s, i) => (
					<Fragment key={s.id}>
						{i > 0 && <span className={`connector ${i <= stepIndex ? "done" : ""}`} />}
						<span className={`step ${i === stepIndex ? "active" : i < stepIndex ? "done" : ""}`}>
							{i + 1}. {s.title}
						</span>
					</Fragment>
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
