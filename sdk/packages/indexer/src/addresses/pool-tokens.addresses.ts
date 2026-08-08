// Pool-token registry accessors. The data itself lives in pool-tokens.generated.ts, which
// scripts/generate-chain-yamls.ts rebuilds from each chain's "yieldVaults" addresses in
// src/configs/config-mainnet.json / config-testnet.json, reading symbol() and decimals() from
// the token contracts. Add tokens there, not here.

import { POOL_TOKENS } from "./pool-tokens.generated"

export { POOL_TOKENS }

export interface PoolToken {
	/** Canonical symbol casing, shared across chains (e.g. "cNGN"). */
	symbol: string
	/** ERC-20 decimals of this token on this chain. */
	decimals: number
}

/** Resolves a leg token to its registry entry; undefined when the token is not pool-tracked. */
export function getPoolToken(chain: string, address: string): PoolToken | undefined {
	return POOL_TOKENS[chain]?.[address.toLowerCase()]
}

const CANONICAL_SYMBOLS = new Map<string, string>()
for (const tokens of Object.values(POOL_TOKENS)) {
	for (const token of Object.values(tokens)) {
		CANONICAL_SYMBOLS.set(token.symbol.toLowerCase(), token.symbol)
	}
}

/** Canonical casing for a symbol known to the registry ("CNGN" -> "cNGN"); undefined otherwise. */
export function canonicalPoolSymbol(symbol: string): string | undefined {
	return CANONICAL_SYMBOLS.get(symbol.toLowerCase())
}

/**
 * The two symbols of a pair in canonical pool order: sorted case-insensitively, so both
 * directions of a pair land in the same pool (cNGN/USDC and USDC/cNGN are both cNGN-USDC).
 * Plain code-unit comparison, never locale-sensitive collation — the result is a persisted
 * primary key and must sort identically everywhere.
 */
export function sortPoolSymbols(symbolA: string, symbolB: string): [string, string] {
	const [a, b] = [symbolA.toLowerCase(), symbolB.toLowerCase()]
	return a <= b ? [symbolA, symbolB] : [symbolB, symbolA]
}

/** Canonical pool slug for a pair: {token0}-{token1} in canonical pool order. */
export function poolSlug(symbolA: string, symbolB: string): string {
	const [first, second] = sortPoolSymbols(symbolA, symbolB)
	return `${first}-${second}`
}
