import {
	isRegistrySymbol,
	normalizeSymbol,
	registrySymbols,
	USD_STABLE_SYMBOLS,
	type AssetDefinition,
} from "@/config/asset-registry"

/**
 * One market from the top-level `[[pairs]]` TOML array:
 *
 * ```toml
 * [[pairs]]
 * token0 = "USDC"   # quote side — any symbol in the asset registry
 * token1 = "CNGN"   # base side — any symbol in the asset registry
 * ```
 *
 * A pair declares that a market exists and nothing more. What the filler will
 * pay on it comes from the operator's limit orders, which carry the rate and the
 * size they are good for.
 *
 * One orientation only: declaring both `A/B` and `B/A` would make matching
 * depend on declaration order. `token0 == token1` is the same-asset cross-chain
 * market.
 */
export interface PairConfig {
	/** Quote-side symbol (e.g. "USDC", "USDT", "ZARP"). */
	token0: string
	/** Base-side symbol (e.g. "CNGN"). Any symbol in the registry. */
	token1: string
}

function isKnownSymbol(symbol: string, userAssets?: Record<string, AssetDefinition>): boolean {
	const normalized = normalizeSymbol(symbol)
	if (isRegistrySymbol(normalized)) return true
	return Object.keys(userAssets ?? {}).some((key) => normalizeSymbol(key) === normalized)
}

export interface PairAddressResolver {
	getAddress(symbol: string, chain: string): string | null
}

/**
 * Boot-parity resolution checks over `[[pairs]]` symbols, shared by the boot
 * path and the wizard gates so a config is rejected where it is written, not
 * first on `simplex run`:
 *  1. every symbol must resolve to a real deployment on at least one of the
 *     given chains — the SDK stores zero-address sentinels for undeployed
 *     assets, and the zero address doubles as the native-token sentinel in the
 *     fill path;
 *  2. no two distinct symbols may resolve to the SAME contract on a chain
 *     (an [assets] alias of USDC's address): aliasing collapses a cross-asset
 *     pair into a same-asset market — bypassing the same-token safeguards —
 *     and makes leg matching order-dependent.
 */
export function assertPairSymbolsResolve(
	pairs: PairConfig[],
	registry: PairAddressResolver,
	chainNames: string[],
): void {
	const pairSymbols = new Set(pairs.flatMap((pair) => [pair.token0, pair.token1]))
	for (const pair of pairs) {
		for (const symbol of [pair.token0, pair.token1]) {
			const resolvesSomewhere = chainNames.some((chainName) => registry.getAddress(symbol, chainName) !== null)
			if (!resolvesSomewhere) {
				throw new Error(
					`pairs.${pair.token0}/${pair.token1}: '${symbol}' does not resolve to a deployed contract on any configured chain`,
				)
			}
		}
	}

	for (const chainName of chainNames) {
		const addressOwner = new Map<string, string>()
		for (const symbol of pairSymbols) {
			const address = registry.getAddress(symbol, chainName)?.toLowerCase()
			if (!address) continue
			const owner = addressOwner.get(address)
			if (owner && normalizeSymbol(owner) !== normalizeSymbol(symbol)) {
				throw new Error(
					`assets: '${symbol}' and '${owner}' both resolve to ${address} on ${chainName} — symbols must map to distinct contracts`,
				)
			}
			addressOwner.set(address, symbol)
		}
	}
}

/**
 * The USD stable to quote a reference feed against for an unanchored symbol:
 * the first stable (registry preference order) with no existing orientation
 * against it — a pair and its reverse are the same market, so a stable that
 * already has any market against the symbol cannot also carry the feed.
 * Returns null when every stable is taken. Shared by both wizards.
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

/**
 * Validates the `[[pairs]]` array against the `[assets]` table and built-in
 * symbols. Pure — throws a descriptive error on the first invalid pair.
 */
export function validatePairConfigs(pairs: PairConfig[], userAssets?: Record<string, AssetDefinition>): void {
	if (!Array.isArray(pairs) || pairs.length === 0) {
		throw new Error("pairs: at least one [[pairs]] entry is required")
	}

	const seen = new Set<string>()
	for (const pair of pairs) {
		if (!pair.token0 || !pair.token1) {
			throw new Error("pairs: each entry needs 'token0' and 'token1' symbols")
		}
		const token0 = normalizeSymbol(pair.token0)
		const token1 = normalizeSymbol(pair.token1)
		const label = `${token0}/${token1}`

		if (seen.has(label)) {
			throw new Error(`pairs.${label}: pair is declared twice`)
		}
		// The reverse orientation is the same market seen from the other side —
		// declaring both would make leg matching declaration-order dependent
		// (and price the reversed legs in the wrong unit). One orientation only.
		if (seen.has(`${token1}/${token0}`)) {
			throw new Error(
				`pairs.${label}: ${token1}/${token0} is already declared — a market has one orientation, and a limit order on either side trades it`,
			)
		}
		seen.add(label)

		for (const symbol of [token0, token1]) {
			if (!isKnownSymbol(symbol, userAssets)) {
				throw new Error(
					`pairs.${label}: unknown symbol '${symbol}' — the registry ships ${registrySymbols().join(", ")}; anything else needs an [assets.${symbol}] entry`,
				)
			}
		}

	}
}
