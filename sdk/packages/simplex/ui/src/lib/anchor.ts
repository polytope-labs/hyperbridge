// Hand-mirrored from src/config/pairs.ts `unanchoredToken0Symbols` — the ui
// bundle cannot import src modules. Keep the two in sync; the server preview
// gate re-runs the real check, so drift shows up as a late error, not a wrong fill.

export interface AnchorPair {
	token0: string
	token1: string
	hasCurve: boolean
}

const norm = (symbol: string) => symbol.trim().toUpperCase()

/**
 * Confirmation depth is sized in USD and the operator's curves are the only
 * price feed: USD stables are $1 anchors and every curve-priced cross-asset
 * pair is an FX edge between its symbols. Returns the token0 symbols
 * (normalized) that cannot be priced from the given pairs.
 */
export function unanchoredToken0Symbols(pairs: AnchorPair[], usdStables: string[]): string[] {
	const anchored = new Set(usdStables.map(norm))
	let grew = true
	while (grew) {
		grew = false
		for (const pair of pairs) {
			const token0 = norm(pair.token0)
			const token1 = norm(pair.token1)
			if (!pair.hasCurve || token0 === token1) continue
			if (anchored.has(token0) && !anchored.has(token1)) {
				anchored.add(token1)
				grew = true
			} else if (anchored.has(token1) && !anchored.has(token0)) {
				anchored.add(token0)
				grew = true
			}
		}
	}

	const missing = new Set<string>()
	for (const pair of pairs) {
		const token0 = norm(pair.token0)
		if (token0 && !anchored.has(token0)) missing.add(token0)
	}
	return [...missing]
}
