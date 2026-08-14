export interface ResolvedLiquidityPool<Symbol extends string = string> {
	poolId: string
	token0Symbol: Symbol
	token1Symbol: Symbol
}

/** Canonical symbol order used by the SDK and indexer pool IDs. */
export function sortPoolSymbols<Symbol extends string>(symbolA: Symbol, symbolB: Symbol): [Symbol, Symbol] {
	return symbolA.toLowerCase() <= symbolB.toLowerCase() ? [symbolA, symbolB] : [symbolB, symbolA]
}

/** Canonical indexer pool ID for a pair of canonical token symbols. */
export function poolSlug(symbolA: string, symbolB: string): string {
	return sortPoolSymbols(symbolA, symbolB).join("-")
}

export function resolveLiquidityPool<Symbol extends string>(
	symbolA: Symbol,
	symbolB: Symbol,
): ResolvedLiquidityPool<Symbol> {
	const [token0Symbol, token1Symbol] = sortPoolSymbols(symbolA, symbolB)
	return {
		poolId: `${token0Symbol}-${token1Symbol}`,
		token0Symbol,
		token1Symbol,
	}
}
