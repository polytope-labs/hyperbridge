// The tokens that liquidity pools are composed from. Phantom order legs carry raw EVM addresses,
// so this registry is what turns a (chain, address) into a canonical symbol — and therefore two
// legs on different chains into the same pool. Decimals are per chain because the same symbol is
// not the same everywhere (BSC stables are 18-decimal, most others 6); every rate and depth
// normalization depends on them being exact.
//
// Addresses and decimals mirror the per-chain `assets`/`tokenDecimals` tables in
// sdk/src/configs/chain.ts. Zero-address (undeployed) entries are omitted.

export interface PoolToken {
	/** Canonical symbol casing, shared across chains (e.g. "cNGN"). */
	symbol: string
	/** ERC-20 decimals of this token on this chain. */
	decimals: number
}

export const POOL_TOKENS: Record<string, Record<string, PoolToken>> = {
	"EVM-1": {
		"0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": { symbol: "USDC", decimals: 6 },
		"0xdac17f958d2ee523a2206206994597c13d831ec7": { symbol: "USDT", decimals: 6 },
		"0x17cdb2a01e7a34cbb3dd4b83260b05d0274c8dab": { symbol: "cNGN", decimals: 6 },
	},
	"EVM-10": {
		"0x0b2c639c533813f4aa9d7837caf62653d097ff85": { symbol: "USDC", decimals: 6 },
		"0x94b008aa00579c1307b0ef2c499ad98a8ce58e58": { symbol: "USDT", decimals: 6 },
	},
	"EVM-56": {
		"0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d": { symbol: "USDC", decimals: 18 },
		"0x55d398326f99059ff775485246999027b3197955": { symbol: "USDT", decimals: 18 },
	},
	"EVM-100": {
		"0xddafbb505ad214d7b80b1f830fccc89b60fb7a83": { symbol: "USDC", decimals: 6 },
		"0x4ecaba5870353805a9f068101a40e0f32ed605c6": { symbol: "USDT", decimals: 6 },
	},
	"EVM-137": {
		"0x3c499c542cef5e3811e1192ce70d8cc03d5c3359": { symbol: "USDC", decimals: 6 },
		"0xc2132d05d31c914a87c6611c10748aeb04b58e8f": { symbol: "USDT", decimals: 6 },
		"0x52828daa48c1a9a06f37500882b42daf0be04c3b": { symbol: "cNGN", decimals: 6 },
	},
	"EVM-1868": {
		"0xba9986d2381edf1da03b0b9c1f8b00dc4aacc369": { symbol: "USDC", decimals: 6 },
	},
	"EVM-8453": {
		"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": { symbol: "USDC", decimals: 6 },
		"0xfde4c96c8593536e31f229ea8f37b2ada2699bb2": { symbol: "USDT", decimals: 6 },
		"0x46c85152bfe9f96829aa94755d9f915f9b10ef5f": { symbol: "cNGN", decimals: 6 },
	},
	"EVM-42161": {
		"0xaf88d065e77c8cc2239327c5edb3a432268e5831": { symbol: "USDC", decimals: 6 },
		"0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9": { symbol: "USDT", decimals: 6 },
	},
	"EVM-420420419": {
		"0x0000053900000000000000000000000001200000": { symbol: "USDC", decimals: 6 },
	},
	// Testnets
	"EVM-97": {
		"0xa801da100bf16d07f668f4a49e1f71fc54d05177": { symbol: "USDC", decimals: 18 },
		"0xc043f483373072f7f27420d6e7d7ad269c018e18": { symbol: "USDT", decimals: 18 },
	},
	"EVM-80002": {
		"0xbe97e73126d66188d72fbf99029126d0340a7f18": { symbol: "USDC", decimals: 18 },
	},
	"EVM-420420417": {
		"0x0dc440cf87830f0af564eb8b62b454b7e0c68a4b": { symbol: "USDC", decimals: 18 },
	},
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
 */
export function sortPoolSymbols(symbolA: string, symbolB: string): [string, string] {
	return [symbolA, symbolB].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase())) as [string, string]
}

/** Canonical pool slug for a pair: {token0}-{token1} in canonical pool order. */
export function poolSlug(symbolA: string, symbolB: string): string {
	const [first, second] = sortPoolSymbols(symbolA, symbolB)
	return `${first}-${second}`
}
