import { Fragment, useState } from "react"
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

export interface StepProps {
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	defaults: SetupDefaults
}

const STEPS: Array<{
	id: string
	title: string
	component: React.ComponentType<StepProps>
	valid: (s: WizardState, defaults: SetupDefaults) => boolean
}> = [
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
		valid: (s, defaults) => {
			const enabled = s.pairs.filter((p) => p.enabled)
			if (enabled.length === 0) return false
			// Wizard-only knowledge the offline config gate does not have: which
			// symbols are deployed on the ENABLED chains (the server re-runs this
			// with the posted chain ids; boot enforces it against the real RPCs).
			const enabledChainDrafts = s.chains.filter((c) => c.enabled)
			const availableOnEnabled = new Set(
				enabledChainDrafts.flatMap((c) =>
					(defaults.knownTokens[c.meta.stateMachineId] ?? []).map((t) => normSymbol(t.symbol)),
				),
			)
			const enabledChainIds = new Set(enabledChainDrafts.map((c) => c.meta.stateMachineId))
			for (const pair of enabled) {
				for (const symbol of [pair.token0, pair.token1]) {
					if (!symbol.trim()) return false
					if (isRegistrySymbol(symbol)) {
						// A registry symbol must never be shadowed by a custom [assets] entry...
						if (s.customAssets[symbol]) return false
						// ...and must actually be deployed on one of the selected chains.
						if (!availableOnEnabled.has(normSymbol(symbol))) return false
					} else {
						const customAddresses = s.customAssets[symbol]
						if (
							!customAddresses ||
							!Object.entries(customAddresses).some(([chain, a]) => enabledChainIds.has(chain) && a.trim())
						) {
							return false
						}
					}
				}
			}
			// The gate only checks a position's token id is non-empty; the wizard knows it must be numeric.
			if (s.fxPricing === "uniswapV4" && !s.fxPositions.every((p) => /^\d+$/.test(p.tokenId.trim()))) return false
			// Every engine rule — orientation, curve completeness, crossed books,
			// below-par asks, USD anchors, venue rules — is the engine's own gate,
			// run on the assembled config exactly as the server and boot run it.
			try {
				validateConfig(assembleConfig(s, defaults))
			} catch {
				return false
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
	const canNext = step.valid(state, props.defaults)

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
