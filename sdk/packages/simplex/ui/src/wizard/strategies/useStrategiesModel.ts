import { useEffect } from "react"
import { unanchoredToken0Symbols } from "@/config/pairs"
import type { EditorPoint } from "../../components/curveModel"
import type { SetupDefaults } from "../../types"
import {
	draftHasCurve,
	enabledChains,
	newCrossAssetDraft,
	normSymbol,
	patchAt,
	type PairDraft,
	type WizardState,
} from "../state"

function curvesAreUntouched(points: EditorPoint[]): boolean {
	return points.every((point) => !point.value.trim())
}

export function prefillCurves(draft: PairDraft, usdStables: string[]): Partial<PairDraft> {
	const bothStable = usdStables.includes(normSymbol(draft.token0)) && usdStables.includes(normSymbol(draft.token1))
	if (!bothStable) return {}
	const patch: Partial<PairDraft> = {}
	if (curvesAreUntouched(draft.bid)) patch.bid = [{ amount: "0", value: "1.001" }]
	if (curvesAreUntouched(draft.ask)) patch.ask = [{ amount: "0", value: "0.999" }]
	return patch
}

/** Derived market state and initialization rules for the strategy step. */
export function useStrategiesModel(options: {
	state: WizardState
	setState: React.Dispatch<React.SetStateAction<WizardState>>
	defaults: SetupDefaults
}) {
	const { state, setState, defaults } = options
	const chains = enabledChains(state)
	const availableSymbols = [
		...new Set(
			chains.flatMap((chain) =>
				(defaults.knownTokens[chain.meta.stateMachineId] ?? []).map((token) => token.symbol),
			),
		),
	].sort((a, b) => Number(defaults.usdStables.includes(b)) - Number(defaults.usdStables.includes(a)))
	const rows = state.pairs.map((pair, index) => ({ pair, index }))
	const marketRows = rows
	const enabled = state.pairs.filter((pair) => pair.enabled)
	const duplicateKeys = new Set<string>()
	const seen = new Set<string>()
	for (const pair of enabled) {
		const key = `${normSymbol(pair.token0)}/${normSymbol(pair.token1)}`
		const reverse = `${normSymbol(pair.token1)}/${normSymbol(pair.token0)}`
		if (seen.has(key) || seen.has(reverse)) duplicateKeys.add(key)
		seen.add(key)
	}
	const unanchored = unanchoredToken0Symbols(
		enabled
			.filter((pair) => pair.token0.trim() && pair.token1.trim())
			.map((pair) => ({
				token0: pair.token0,
				token1: pair.token1,
				hasCurve: draftHasCurve(pair, state.fxPricing),
			})),
	)
	const defaultToken1 = availableSymbols.find((symbol) => !defaults.usdStables.includes(symbol)) ?? "USDT"

	useEffect(() => {
		setState((current) => {
			if (current.fxSeeded) return current
			const draft = newCrossAssetDraft(defaultToken1)
			return {
				...current,
				fxSeeded: true,
				pairs: [
					...current.pairs,
					{
						...draft,
						...prefillCurves(draft, defaults.usdStables),
					},
				],
			}
		})
	}, [defaultToken1, defaults.usdStables, setState])

	const patchPair = (index: number, patch: Partial<PairDraft>) =>
		setState((current) => ({ ...current, pairs: patchAt(current.pairs, index, patch) }))

	return {
		chains,
		availableSymbols,
		marketRows,
		enabled,
		duplicateKeys,
		unanchored,
		defaultToken1,
		patchPair,
	}
}
