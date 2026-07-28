import { Fragment, useState } from "react"
import { bookCrossedAt } from "@/config/interpolated-curve"
import { unanchoredToken0Symbols } from "@/config/pairs"
import { toPricePoints } from "../components/CurveEditor"
import type { SetupDefaults } from "../types"
import { curveFilled, draftHasCurve, initialState, isSameTokenDraft, normSymbol, type WizardState } from "./state"
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
			// Mirrors FillerPricePolicy / validatePairConfigs; the server gate is authoritative.
			// A market has one orientation: declaring the reverse (or a duplicate) is rejected at startup.
			const orientations = new Set<string>()
			for (const pair of enabled) {
				if (!pair.token0.trim() || !pair.token1.trim()) return false
				const key = `${normSymbol(pair.token0)}/${normSymbol(pair.token1)}`
				const reverse = `${normSymbol(pair.token1)}/${normSymbol(pair.token0)}`
				if (orientations.has(key) || orientations.has(reverse)) return false
				orientations.add(key)
			}
			const shipped = new Set(defaults.registrySymbols.map((t) => normSymbol(t)))
			// Symbols deployed on at least one ENABLED chain (mirrors boot's resolves-somewhere check).
			const availableOnEnabled = new Set(
				s.chains
					.filter((c) => c.enabled)
					.flatMap((c) => (defaults.knownTokens[c.meta.stateMachineId] ?? []).map((t) => normSymbol(t.symbol))),
			)
			const enabledChainIds = new Set(s.chains.filter((c) => c.enabled).map((c) => c.meta.stateMachineId))
			for (const pair of enabled) {
				for (const symbol of [pair.token0, pair.token1]) {
					if (shipped.has(normSymbol(symbol))) {
						// A registry symbol must never be shadowed by a custom [assets] entry...
						if (s.customAssets[symbol]) return false
						// ...and must actually be deployed on one of the selected chains.
						if (!availableOnEnabled.has(normSymbol(symbol))) return false
						continue
					}
					const customAddresses = s.customAssets[symbol]
					if (
						!customAddresses ||
						!Object.entries(customAddresses).some(([chain, a]) => enabledChainIds.has(chain) && a.trim())
					) {
						return false
					}
				}
				if (pair.referenceOnly) {
					if (!curveFilled(pair.ask)) return false
					continue
				}
				if (!(Number(pair.maxOrderSize) > 0)) return false
				if (pair.kind === "sameAsset" || isSameTokenDraft(pair)) {
					// Same-token asks must sit strictly below par — the gap is the spread.
					if (!curveFilled(pair.ask, (v) => v > 0 && v < 1)) return false
					continue
				}
				if (s.fxPricing === "curves") {
					if (!pair.bidEnabled && !pair.askEnabled) return false
					if (pair.bidEnabled && !curveFilled(pair.bid)) return false
					if (pair.askEnabled && !curveFilled(pair.ask)) return false
					// A crossed (or zero-spread) book can never fill.
					if (
						pair.bidEnabled &&
						pair.askEnabled &&
						bookCrossedAt(toPricePoints(pair.bid), toPricePoints(pair.ask)) !== null
					) {
						return false
					}
				} else if (!defaults.usdStables.includes(normSymbol(pair.token0))) {
					// Venue pricing only quotes pairs with a USD-stable token0.
					return false
				}
			}
			// Every token0 must reach a USD anchor through the declared curves —
			// including same-token markets, whose token0 must itself be anchored.
			// Rows with a symbol still being typed are gated above, not here.
			const anchorInput = enabled
				.filter((p) => p.token0.trim() && p.token1.trim())
				.map((p) => ({
					token0: p.token0,
					token1: p.token1,
					hasCurve: draftHasCurve(p, s.fxPricing),
				}))
			if (unanchoredToken0Symbols(anchorInput).length > 0) return false
			const hasCrossAsset = enabled.some((p) => p.kind === "crossAsset" && !p.referenceOnly && !isSameTokenDraft(p))
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
				// `side` is only valid when NO pair carries curves; same-token and
				// reference rows always do.
				if (s.fxSide !== "" && enabled.some((p) => draftHasCurve(p, s.fxPricing))) return false
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
