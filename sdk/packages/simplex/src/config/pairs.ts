import { Decimal } from "decimal.js"
import { FillerPricePolicy, type PriceCurvePoint } from "@/config/interpolated-curve"
import {
	isRegistrySymbol,
	normalizeSymbol,
	registrySymbols,
	USD_STABLE_SYMBOLS,
	type AssetDefinition,
} from "@/config/asset-registry"

/**
 * One trading pair from the top-level `[[pairs]]` TOML array:
 *
 * ```toml
 * [[pairs]]
 * token0 = "USDC"        # quote side — any symbol in the asset registry
 * token1 = "CNGN"        # base side — any symbol in the asset registry
 * maxOrderSize = "5000"  # optional per-order cap, in token0 units; omit for uncapped
 * bidPriceCurve = [ { amount = "1000", price = "1580" } ]
 * askPriceCurve = [ { amount = "1000", price = "1550" } ]
 * ```
 *
 * Curves are priced in **token1 per 1 token0** and keyed by the order's
 * token0 notional — the same unit as `maxOrderSize`, so a pair is priced,
 * capped, and sized entirely in its own quote asset with no external price
 * feed. The bid curve prices the filler *buying* token1 (user sends token1,
 * receives token0); the ask curve prices the filler *selling* token1.
 * Omitting one curve disables that direction for this pair (one-sided LP);
 * omitting both is only valid when Uniswap V4 venue pricing is configured.
 *
 * A pair may quote the **same symbol on both sides** (`token0 = token1 =
 * "USDC"`) — the same-asset cross-chain market. Such pairs are ask-only with
 * prices strictly below par; the gap to 1 is the filler's spread (e.g.
 * "0.9995" keeps 5 bps of every fill). Cross-asset books quote bid and ask
 * independently — each side fills at its own curve. A crossed book (bid at or
 * below ask) is allowed; it just means a full round trip loses money.
 *
 * Users may declare any number of pairs; a single engine serves all of them.
 */
export interface PairConfig {
	/** Quote-side symbol (e.g. "USDC", "USDT", "ZARP"). */
	token0: string
	/** Base-side symbol (e.g. "CNGN"). Any symbol in the registry. */
	token1: string
	/** Maximum token0 notional this pair fills per order. Optional for reference-only pairs. */
	maxOrderSize?: string
	/**
	 * A pure price feed: the pair contributes its FX edge to the USD anchor
	 * graph (so confirmation depth can price its assets) but never matches
	 * order legs — no market is opened. Lets an operator run e.g. a lone
	 * CNGN/CNGN transfer market anchored by a reference USDC/CNGN quote
	 * without offering to trade USDC/CNGN.
	 */
	referenceOnly?: boolean
	/** token1 per token0 when the filler buys token1. Omit for ask-only (one-sided) LP. */
	bidPriceCurve?: PriceCurvePoint[]
	/** token1 per token0 when the filler sells token1. Omit for bid-only (one-sided) LP. */
	askPriceCurve?: PriceCurvePoint[]
}

function isKnownSymbol(symbol: string, userAssets?: Record<string, AssetDefinition>): boolean {
	const normalized = normalizeSymbol(symbol)
	if (isRegistrySymbol(normalized)) return true
	return Object.keys(userAssets ?? {}).some((key) => normalizeSymbol(key) === normalized)
}

/**
 * Confirmation depth is sized in USD, and the only price feed is the operator's
 * own curves: USD stables are $1 anchors and every curve-priced cross-asset
 * pair is an FX edge between its two symbols. A token0 can therefore be priced
 * iff it is a USD stable or connected to one through curve edges — possibly
 * transitively (USDC/CNGN + ZARP/CNGN anchors ZARP through CNGN).
 *
 * Returns the token0 symbols (normalized) that cannot be priced. Same-token
 * pairs carry no FX information and venue-priced (curve-less) pairs have no
 * quote at validation time, so neither contributes an edge.
 */
export function unanchoredToken0Symbols(
	pairs: Array<{ token0: string; token1: string; hasCurve: boolean }>,
): string[] {
	const anchored = new Set(USD_STABLE_SYMBOLS)
	// Fixed-point: each pass extends reachability by at least one hop or stops.
	let grew = true
	while (grew) {
		grew = false
		for (const pair of pairs) {
			const token0 = normalizeSymbol(pair.token0)
			const token1 = normalizeSymbol(pair.token1)
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
		const token0 = normalizeSymbol(pair.token0)
		if (!anchored.has(token0)) missing.add(token0)
	}
	return [...missing]
}

function validateCurve(pairLabel: string, name: string, curve: PriceCurvePoint[] | undefined): void {
	if (curve === undefined) return
	if (!Array.isArray(curve) || curve.length < 1) {
		throw new Error(`pairs.${pairLabel}: '${name}' must be an array with at least 1 point`)
	}
	for (const point of curve) {
		if (point.amount === undefined || point.price === undefined) {
			throw new Error(`pairs.${pairLabel}: each ${name} point must have 'amount' and 'price'`)
		}
		let price: Decimal
		try {
			price = new Decimal(point.price)
		} catch {
			throw new Error(`pairs.${pairLabel}: ${name} price must be a decimal string, got '${point.price}'`)
		}
		if (!price.isFinite() || price.lte(0)) {
			throw new Error(`pairs.${pairLabel}: ${name} prices must be positive, got '${point.price}'`)
		}
	}
	// Backstop: the exact validation the engine's policy runs at boot (amounts
	// included), so the gate can never pass a curve boot would reject.
	try {
		void new FillerPricePolicy({ points: curve })
	} catch (err) {
		throw new Error(`pairs.${pairLabel}: ${name} — ${err instanceof Error ? err.message : err}`)
	}
}

/** Resolves a symbol to its contract address on a chain, or null when not deployed there. */
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

/**
 * Validates the `[[pairs]]` array against the `[assets]` table and built-in
 * symbols. Pure — throws a descriptive error on the first invalid pair.
 *
 * @param hasVenuePricing whether Uniswap V4 venue pricing is configured; a pair
 *   with no curves is only valid when it is.
 */
export function validatePairConfigs(
	pairs: PairConfig[],
	userAssets?: Record<string, AssetDefinition>,
	hasVenuePricing = false,
): void {
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
				`pairs.${label}: ${token1}/${token0} is already declared — a market has one orientation; use its bid curve for the reverse direction`,
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

		// Optional: omitting it leaves the pair uncapped, filling every order at its
		// full notional. Present and malformed is still an error.
		if (pair.maxOrderSize !== undefined) {
			let maxOrderSize: Decimal
			try {
				maxOrderSize = new Decimal(pair.maxOrderSize)
			} catch {
				throw new Error(`pairs.${label}: 'maxOrderSize' must be a decimal string, got '${pair.maxOrderSize}'`)
			}
			if (!maxOrderSize.isFinite() || maxOrderSize.lte(0)) {
				throw new Error(`pairs.${label}: 'maxOrderSize' must be a positive number, got '${pair.maxOrderSize}'`)
			}
		}

		validateCurve(label, "bidPriceCurve", pair.bidPriceCurve)
		validateCurve(label, "askPriceCurve", pair.askPriceCurve)

		// Reference-only pairs are pure price feeds for the USD anchor graph:
		// they need a curve to carry a rate, and a same-token pair has no FX
		// edge to contribute, so reference-only is cross-asset by definition.
		if (pair.referenceOnly) {
			if (token0 === token1) {
				throw new Error(
					`pairs.${label}: 'referenceOnly' applies to cross-asset pairs — a same-token pair carries no FX rate to reference`,
				)
			}
			if ((pair.bidPriceCurve?.length ?? 0) < 1 && (pair.askPriceCurve?.length ?? 0) < 1) {
				throw new Error(
					`pairs.${label}: a referenceOnly pair needs a bid and/or ask price curve — the curve IS the reference`,
				)
			}
		}

		// Same-token pairs (token0 == token1) are the same-asset cross-chain
		// market (the former "stable" strategy): both directions are one market,
		// so they are ask-only, and the ask price is the fraction of the input
		// paid back out — above par would be a guaranteed loss.
		if (token0 === token1) {
			if (pair.bidPriceCurve !== undefined) {
				throw new Error(
					`pairs.${label}: same-token pairs are ask-only — omit 'bidPriceCurve' (both directions are the same market)`,
				)
			}
			if ((pair.askPriceCurve?.length ?? 0) < 1) {
				throw new Error(
					`pairs.${label}: same-token pairs need an 'askPriceCurve' with prices below par (e.g. "0.9995" keeps a 5 bps spread)`,
				)
			}
			for (const point of pair.askPriceCurve ?? []) {
				let price: Decimal
				try {
					price = new Decimal(point.price)
				} catch {
					throw new Error(`pairs.${label}: ask price must be a decimal string, got '${point.price}'`)
				}
				if (!price.isFinite() || price.lte(0) || price.gte(1)) {
					throw new Error(
						`pairs.${label}: same-token ask prices must be strictly below 1 — '${point.price}' gives back at least what was received (zero or negative spread never fills)`,
					)
				}
			}
		}

		const hasAnyCurve = (pair.bidPriceCurve?.length ?? 0) >= 1 || (pair.askPriceCurve?.length ?? 0) >= 1
		if (!hasAnyCurve && !hasVenuePricing) {
			throw new Error(
				`pairs.${label}: provide a bid and/or ask price curve, or configure [vault.uniswapV4] positions for pool-based pricing`,
			)
		}

		// A crossed book (bid ≤ ask) is allowed: each side is quoted and filled
		// independently at its own curve — crossing only means a full round trip
		// loses money. The wizards surface it as a warning, not an error.
	}

	// Every pair's token0 sizes orders for the confirmation-depth curve, whose
	// amount axis is USD — so every token0 must be priceable in USD from the
	// declared curves alone. Fail at startup, not with miscalibrated (or
	// unpriceable) reorg protection at fill time.
	const unanchored = unanchoredToken0Symbols(
		pairs.map((p) => ({
			token0: p.token0,
			token1: p.token1,
			hasCurve: (p.bidPriceCurve?.length ?? 0) >= 1 || (p.askPriceCurve?.length ?? 0) >= 1,
		})),
	)
	if (unanchored.length > 0) {
		throw new Error(
			`pairs: no USD anchor for ${unanchored.join(", ")} — confirmation depth is sized in USD using your own curves as the price feed. Add a curve-priced pair against a USD stable (e.g. USDC/${unanchored[0]}), directly or through an already-anchored asset; declare it 'referenceOnly = true' to anchor without opening that market`,
		)
	}
}
