import { isAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"

/**
 * A user-supplied asset entry in the `[assets]` TOML table: symbol → chain
 * state machine id → contract address.
 *
 * ```toml
 * [assets.BRZ]
 * "EVM-1"   = "0x..."
 * "EVM-137" = "0x..."
 * ```
 */
export type AssetDefinition = Record<string, HexString>

/**
 * The subset of `FillerConfigService` used to resolve built-in symbols to
 * per-chain addresses from the SDK chain registry.
 */
export interface BuiltinAssetResolver {
	getUsdcAsset(chain: string): HexString
	getUsdtAsset(chain: string): HexString
	getDaiAsset(chain: string): HexString
	getCNgnAsset(chain: string): HexString | undefined
	/** Generic per-chain lookup into the SDK chain registry's asset table. */
	getAssetBySymbol(chain: string, symbol: string): HexString | undefined
}

interface BuiltinSpec {
	resolve: (resolver: BuiltinAssetResolver, chain: string) => HexString | undefined
}

/**
 * Symbols resolved per chain from the SDK chain registry — the single source
 * of truth for token addresses (`chain.ts`), shared with the rest of the SDK
 * so a new asset is added there once and never in a parallel table here.
 * ZARP/EURC/XSGD/TRYB/USDR are curated stablecoin deployments whose addresses
 * were taken from the issuer's official documentation and verified on-chain
 * (`symbol()`/`decimals()`) before inclusion in the SDK registry.
 */
const BUILTIN_ASSETS: Record<string, BuiltinSpec> = {
	USDC: { resolve: (r, chain) => r.getUsdcAsset(chain) },
	USDT: { resolve: (r, chain) => r.getUsdtAsset(chain) },
	DAI: { resolve: (r, chain) => r.getDaiAsset(chain) },
	CNGN: { resolve: (r, chain) => r.getCNgnAsset(chain) },
	USDR: { resolve: (r, chain) => r.getAssetBySymbol(chain, "USDR") },
	ZARP: { resolve: (r, chain) => r.getAssetBySymbol(chain, "ZARP") },
	EURC: { resolve: (r, chain) => r.getAssetBySymbol(chain, "EURC") },
	XSGD: { resolve: (r, chain) => r.getAssetBySymbol(chain, "XSGD") },
	TRYB: { resolve: (r, chain) => r.getAssetBySymbol(chain, "TRYB") },
}

/**
 * Symbols pegged to 1 USD. Two roles: gating Uniswap venue pricing (a pool's
 * USD-per-token quote only inverts into a pair rate when token0 is a dollar),
 * and seeding the USD anchor graph at $1 — the roots from which every pair's
 * token0 must be reachable so confirmation depth can be sized in USD (see
 * `unanchoredToken0Symbols` and `FXFiller.usdFactors`). Trade pricing never
 * uses this as a price.
 */
export const USD_STABLE_SYMBOLS: ReadonlySet<string> = new Set(["USDC", "USDT", "DAI"])


/** Normalises a symbol for lookups: trimmed, uppercased. "cNGN" ≡ "CNGN". */
export function normalizeSymbol(symbol: string): string {
	return symbol.trim().toUpperCase()
}

const ZERO_ADDRESS = "0x0000000000000000000000000000000000000000"

/**
 * Whether a value from an address source is a real deployment. The SDK chain
 * registry returns `"0x"` for chains it has no entry for and stores a literal
 * zero address for assets not deployed on a chain — both are truthy, and the
 * zero address doubles as the NATIVE-token sentinel in the fill path, so
 * letting either through would price native currency as if it were the asset.
 */
function isRealAddress(address: string | undefined): address is HexString {
	return !!address && isAddress(address, { strict: false }) && address.toLowerCase() !== ZERO_ADDRESS
}

/** Every symbol the registry ships without user configuration. */
export function registrySymbols(): string[] {
	return Object.keys(BUILTIN_ASSETS)
}

/** Whether `symbol` ships with the registry (built-in or curated). Pure — no chain resolution. */
export function isRegistrySymbol(symbol: string): boolean {
	const normalized = normalizeSymbol(symbol)
	return normalized in BUILTIN_ASSETS
}

/**
 * Validates the `[assets]` table; throws a descriptive error on the first
 * invalid entry. Side-effect free — safe to call at config-parse time and again
 * at registry construction.
 */
export function validateAssetDefinitions(assets: Record<string, AssetDefinition>): void {
	const seen = new Set<string>()
	for (const [symbol, definition] of Object.entries(assets)) {
		const normalized = normalizeSymbol(symbol)
		if (normalized.length === 0) {
			throw new Error("assets: symbol must be a non-empty string")
		}
		if (seen.has(normalized)) {
			throw new Error(`assets: symbol '${normalized}' is defined twice (symbols are case-insensitive)`)
		}
		seen.add(normalized)

		const entries = Object.entries(definition ?? {})
		if (entries.length === 0) {
			throw new Error(`assets.${symbol}: entry must map at least one chain to a token address`)
		}
		for (const [chain, address] of entries) {
			// Lookups are exact-string matches against resolved chain names
			// ("EVM-<id>"), so a typo'd key would silently mean "not deployed
			// on this chain" and the pair would never trade there.
			if (!/^EVM-\d+$/.test(chain)) {
				throw new Error(
					`assets.${symbol}: chain key '${chain}' must be "EVM-<chainId>" (e.g. "EVM-137")`,
				)
			}
			if (!isAddress(address) || address.toLowerCase() === ZERO_ADDRESS) {
				throw new Error(`assets.${symbol}: invalid address '${address}' for chain '${chain}'`)
			}
		}
	}
}

/**
 * Symbol → contract address registry backing the `[[pairs]]` configuration.
 *
 * Resolution merges two layers, most specific winning:
 *  1. the user's `[assets]` table — an *escape hatch* for assets the registry
 *     doesn't ship (or per-deployment overrides), never required for shipped
 *     symbols;
 *  2. shipped symbols (USDC, USDT, DAI, CNGN, USDR, ZARP, EURC, XSGD, TRYB)
 *     resolved per chain from the SDK chain registry (`chain.ts`) — the single
 *     source of truth shared with the rest of the SDK, so an address
 *     correction there is never shadowed by a parallel table here.
 *
 * Address lookups are per `(symbol, chain)` — a chain where no layer knows the
 * asset simply doesn't trade pairs involving it. The registry holds addresses
 * only; all pricing (including risk sizing) is derived from the pair curves.
 */
export class AssetRegistry {
	private readonly resolver: BuiltinAssetResolver
	private readonly userAssets: Map<string, AssetDefinition>
	private readonly addressCache = new Map<string, HexString | null>()

	constructor(resolver: BuiltinAssetResolver, userAssets?: Record<string, AssetDefinition>) {
		this.resolver = resolver
		if (userAssets) validateAssetDefinitions(userAssets)
		this.userAssets = new Map(
			Object.entries(userAssets ?? {}).map(([symbol, definition]) => [normalizeSymbol(symbol), definition]),
		)
	}

	/** Whether `symbol` is known (user-defined, curated, or built-in). */
	hasSymbol(symbol: string): boolean {
		const normalized = normalizeSymbol(symbol)
		return this.userAssets.has(normalized) || isRegistrySymbol(normalized)
	}

	/**
	 * Registers new user-defined assets on the live registry (runtime market
	 * adds). Existing symbols may not be redefined — an address swap under a
	 * running market would silently repoint its fills.
	 */
	addAssets(defs: Record<string, AssetDefinition>): void {
		validateAssetDefinitions(defs)
		for (const symbol of Object.keys(defs)) {
			if (this.hasSymbol(symbol)) {
				throw new Error(`assets: '${symbol}' is already defined and cannot be redefined at runtime`)
			}
		}
		for (const [symbol, definition] of Object.entries(defs)) {
			const normalized = normalizeSymbol(symbol)
			this.userAssets.set(normalized, definition)
			// The cache stores null for previously-unknown symbols; drop those
			// entries or the new token stays unresolvable.
			for (const key of this.addressCache.keys()) {
				if (key.startsWith(`${normalized}:`)) this.addressCache.delete(key)
			}
		}
	}

	/**
	 * Contract address of `symbol` on `chain`, or `null` when the asset is not
	 * deployed/known there. User `[assets]` addresses win over the SDK chain
	 * registry, per chain.
	 */
	getAddress(symbol: string, chain: string): HexString | null {
		const normalized = normalizeSymbol(symbol)
		const cacheKey = `${normalized}:${chain}`
		const cached = this.addressCache.get(cacheKey)
		if (cached !== undefined) return cached

		let address: string | undefined = this.userAssets.get(normalized)?.[chain]
		if (!address) {
			const builtin = BUILTIN_ASSETS[normalized]
			if (builtin) {
				try {
					address = builtin.resolve(this.resolver, chain)
				} catch {
					address = undefined
				}
			}
		}

		// The SDK registry signals absence with sentinels ("0x", the zero
		// address) rather than throwing — filter them so callers get a clean
		// "not deployed here" instead of a poisonous pseudo-address.
		const result = isRealAddress(address) ? address : null
		this.addressCache.set(cacheKey, result)
		return result
	}
}
