import { normalizeSymbol, USD_STABLE_SYMBOLS } from "@/config/asset-registry"

/**
 * The USD stable to quote a reference feed against for an unanchored symbol:
 * the first stable (registry preference order) with no existing orientation
 * against it — a pair and its reverse are the same market, so a stable that
 * already has any market against the symbol cannot also carry the feed.
 * Returns null when every stable is taken (the operator must give one of
 * those existing markets a price curve instead). Shared by both wizards.
 */
export function pickAnchorStable(pairs: Array<{ token0: string; token1: string }>, symbol: string): string | null {
	const target = normalizeSymbol(symbol)
	for (const stable of USD_STABLE_SYMBOLS) {
		const taken = pairs.some((pair) => {
			const token0 = normalizeSymbol(pair.token0)
			const token1 = normalizeSymbol(pair.token1)
			return (token0 === stable && token1 === target) || (token0 === target && token1 === stable)
		})
		if (!taken) return stable
	}
	return null
}
