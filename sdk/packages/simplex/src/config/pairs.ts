import { Decimal } from "decimal.js"
import type { PriceCurvePoint } from "@/config/interpolated-curve"
import { isRegistrySymbol, normalizeSymbol, registrySymbols, type AssetDefinition } from "@/config/asset-registry"

/**
 * One trading pair from the top-level `[[pairs]]` TOML array:
 *
 * ```toml
 * [[pairs]]
 * token0 = "USDC"        # quote side — any symbol in the asset registry
 * token1 = "CNGN"        # base side — any symbol in the asset registry
 * maxOrderSize = "5000"  # per-order cap, in token0 units
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
 * Users may declare any number of pairs; a single FX engine serves all of them.
 */
export interface PairConfig {
	/** Quote-side symbol (e.g. "USDC", "USDT", "ZARP"). */
	token0: string
	/** Base-side symbol (e.g. "CNGN"). Any symbol in the registry. */
	token1: string
	/** Maximum token0 notional this pair fills per order. */
	maxOrderSize: string
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

function validateCurve(pairLabel: string, name: string, curve: PriceCurvePoint[] | undefined): void {
	if (curve === undefined) return
	if (!Array.isArray(curve) || curve.length < 1) {
		throw new Error(`pairs.${pairLabel}: '${name}' must be an array with at least 1 point`)
	}
	for (const point of curve) {
		if (point.amount === undefined || point.price === undefined) {
			throw new Error(`pairs.${pairLabel}: each ${name} point must have 'amount' and 'price'`)
		}
	}
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

		if (token0 === token1) {
			throw new Error(`pairs.${label}: token0 and token1 must differ`)
		}
		if (seen.has(label)) {
			throw new Error(`pairs.${label}: pair is declared twice`)
		}
		seen.add(label)

		for (const symbol of [token0, token1]) {
			if (!isKnownSymbol(symbol, userAssets)) {
				throw new Error(
					`pairs.${label}: unknown symbol '${symbol}' — the registry ships ${registrySymbols().join(", ")}; anything else needs an [assets.${symbol}] entry`,
				)
			}
		}

		if (pair.maxOrderSize === undefined) {
			throw new Error(`pairs.${label}: 'maxOrderSize' is required (per-order cap in ${token0} units)`)
		}
		let maxOrderSize: Decimal
		try {
			maxOrderSize = new Decimal(pair.maxOrderSize)
		} catch {
			throw new Error(`pairs.${label}: 'maxOrderSize' must be a decimal string, got '${pair.maxOrderSize}'`)
		}
		if (!maxOrderSize.isFinite() || maxOrderSize.lte(0)) {
			throw new Error(`pairs.${label}: 'maxOrderSize' must be a positive number, got '${pair.maxOrderSize}'`)
		}

		validateCurve(label, "bidPriceCurve", pair.bidPriceCurve)
		validateCurve(label, "askPriceCurve", pair.askPriceCurve)

		const hasAnyCurve = (pair.bidPriceCurve?.length ?? 0) >= 1 || (pair.askPriceCurve?.length ?? 0) >= 1
		if (!hasAnyCurve && !hasVenuePricing) {
			throw new Error(
				`pairs.${label}: provide a bid and/or ask price curve, or configure [strategies.vault.uniswapV4] positions for pool-based pricing`,
			)
		}
	}
}
