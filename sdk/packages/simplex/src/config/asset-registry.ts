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
}

interface BuiltinSpec {
	resolve: (resolver: BuiltinAssetResolver, chain: string) => HexString | undefined
}

/**
 * Symbols resolved per chain from the SDK chain registry (which maintains their
 * addresses across mainnets and testnets).
 */
const BUILTIN_ASSETS: Record<string, BuiltinSpec> = {
	USDC: { resolve: (r, chain) => r.getUsdcAsset(chain) },
	USDT: { resolve: (r, chain) => r.getUsdtAsset(chain) },
	DAI: { resolve: (r, chain) => r.getDaiAsset(chain) },
	CNGN: { resolve: (r, chain) => r.getCNgnAsset(chain) },
}

/**
 * Symbols pegged to 1 USD, used to gate Uniswap venue pricing (a pool's
 * USD-per-token quote only inverts into a pair rate when token0 is a dollar).
 * Trade pricing never uses this as a price.
 */
export const USD_STABLE_SYMBOLS: ReadonlySet<string> = new Set(["USDC", "USDT", "DAI"])

/**
 * Curated registry of additional stablecoin deployments on the supported
 * mainnets, maintained here so operators reference assets purely by symbol —
 * no address configuration required. Every address below was taken from the
 * issuer's official documentation (Circle, ZARP Stablecoin, StraitsX, BiLira)
 * and verified on-chain (`symbol()` + `decimals()`) before inclusion.
 */
export const KNOWN_ASSETS: Record<string, AssetDefinition> = {
	// South African rand — same contract address on every EVM deployment.
	ZARP: {
		"EVM-1": "0xb755506531786C8aC63B756BaB1ac387bACB0C04",
		"EVM-137": "0xb755506531786C8aC63B756BaB1ac387bACB0C04",
		"EVM-8453": "0xb755506531786C8aC63B756BaB1ac387bACB0C04",
	},
	// Circle euro stablecoin.
	EURC: {
		"EVM-1": "0x1aBaEA1f7C830bD89Acc67eC4af516284b1bC33c",
		"EVM-8453": "0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1adb42",
	},
	// StraitsX Singapore dollar stablecoin.
	XSGD: {
		"EVM-1": "0x70e8dE73cE538DA2bEEd35d14187F6959a8ecA96",
		"EVM-137": "0xDC3326e71D45186F113a2F448984CA0e8D201995",
	},
	// BiLira Turkish lira stablecoin.
	TRYB: {
		"EVM-1": "0x2C537E5624e4af88A7ae4060C022609376C8D0EB",
	},
}
// Frozen: the registry is shared, and `AssetRegistry` caches resolutions
// permanently — a runtime mutation would leave lookups incoherent.
for (const definition of Object.values(KNOWN_ASSETS)) Object.freeze(definition)
Object.freeze(KNOWN_ASSETS)

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
	return [...new Set([...Object.keys(BUILTIN_ASSETS), ...Object.keys(KNOWN_ASSETS)])]
}

/** Whether `symbol` ships with the registry (built-in or curated). Pure — no chain resolution. */
export function isRegistrySymbol(symbol: string): boolean {
	const normalized = normalizeSymbol(symbol)
	return normalized in BUILTIN_ASSETS || normalized in KNOWN_ASSETS
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
			if (!isAddress(address) || address.toLowerCase() === ZERO_ADDRESS) {
				throw new Error(`assets.${symbol}: invalid address '${address}' for chain '${chain}'`)
			}
		}
	}
}

/**
 * Symbol → contract address registry backing the `[[pairs]]` configuration.
 *
 * Resolution merges three layers, most specific winning:
 *  1. the user's `[assets]` table — an *escape hatch* for assets the registry
 *     doesn't ship (or per-deployment overrides), never required for shipped
 *     symbols;
 *  2. the curated {@link KNOWN_ASSETS} table maintained in this repository;
 *  3. built-in symbols (USDC, USDT, DAI, CNGN) resolved per chain from the SDK
 *     chain registry (which also covers testnets).
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
	 * Contract address of `symbol` on `chain`, or `null` when the asset is not
	 * deployed/known there. User `[assets]` addresses win over the curated
	 * table, which wins over the built-in SDK registry, per chain.
	 */
	getAddress(symbol: string, chain: string): HexString | null {
		const normalized = normalizeSymbol(symbol)
		const cacheKey = `${normalized}:${chain}`
		const cached = this.addressCache.get(cacheKey)
		if (cached !== undefined) return cached

		let address: string | undefined =
			this.userAssets.get(normalized)?.[chain] ?? KNOWN_ASSETS[normalized]?.[chain]
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
