/**
 * Canonical symbol order used by the SDK and indexer pool IDs.
 *
 * Plain code-unit comparison, never locale-sensitive collation — the result is
 * a persisted primary key and must sort identically everywhere.
 */
export function sortPoolSymbols<Symbol extends string>(symbolA: Symbol, symbolB: Symbol): [Symbol, Symbol] {
	return symbolA.toLowerCase() <= symbolB.toLowerCase() ? [symbolA, symbolB] : [symbolB, symbolA]
}

/** Canonical indexer pool ID for a pair of canonical token symbols. */
export function poolSlug(symbolA: string, symbolB: string): string {
	return sortPoolSymbols(symbolA, symbolB).join("-")
}
