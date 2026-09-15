import { Decimal } from "decimal.js"
import { normalizeSymbol, USD_STABLE_SYMBOLS } from "@/config/asset-registry"
import type { LimitOrder } from "@/data/types"
import { ORDERBOOK_SCALE } from "./amounts"

/** One market's rate: 1 `base` is worth `rate` of `quote`. */
export interface UsdEdge {
	base: string
	quote: string
	rate: Decimal
}

/**
 * The rates the operator's own limit orders imply, one per order.
 *
 * A same-asset order carries no exchange rate and is skipped, the way the curve
 * graph skipped a same-token pair: pricing USDC against USDC would anchor
 * nothing and could only contaminate a real route.
 */
export function limitOrderUsdEdges(orders: readonly LimitOrder[]): UsdEdge[] {
	return orders
		.filter((order) => normalizeSymbol(order.base) !== normalizeSymbol(order.quote))
		.map((order) => ({
			base: normalizeSymbol(order.base),
			quote: normalizeSymbol(order.quote),
			rate: new Decimal(order.price).div(ORDERBOOK_SCALE.toString()),
		}))
		.filter((edge) => edge.rate.isFinite() && edge.rate.gt(0))
}

/**
 * USD per unit of every symbol reachable from a dollar stable through `edges`.
 *
 * Dollar stables are pinned at $1 and never re-priced, so a mis-set
 * stable-against-stable market cannot move the anchors. Everything else is
 * reached by walking outward from them: an edge with a known side prices its
 * unknown side, and the walk repeats until nothing new is learned.
 *
 * Edges are sorted before the walk and a symbol is priced once, by the first
 * edge that reaches it. Two routes to the same symbol will usually disagree
 * slightly, and taking the first in a fixed order makes the answer the same on
 * every run rather than a function of what happened to be created when.
 */
export function usdFactorsFrom(edges: readonly UsdEdge[]): Map<string, Decimal> {
	const factors = new Map<string, Decimal>()
	for (const symbol of USD_STABLE_SYMBOLS) factors.set(symbol, new Decimal(1))

	const ordered = [...edges].sort((a, b) => a.base.localeCompare(b.base) || a.quote.localeCompare(b.quote))

	let grew = true
	while (grew) {
		grew = false
		for (const edge of ordered) {
			const usdBase = factors.get(edge.base)
			const usdQuote = factors.get(edge.quote)
			if (usdBase && !usdQuote) {
				// 1 base = rate quote, so a quote unit is worth that much less.
				factors.set(edge.quote, usdBase.div(edge.rate))
				grew = true
			} else if (usdQuote && !usdBase) {
				factors.set(edge.base, usdQuote.mul(edge.rate))
				grew = true
			}
		}
	}
	return factors
}

/**
 * What `amount` of `symbol` is worth in dollars, or null when no limit order
 * connects it to one.
 *
 * Returning null is the honest answer rather than a guess: the caller sizes a
 * confirmation wait with this, and a made-up dollar value would under-wait on a
 * large order.
 */
export function usdValueOf(
	factors: Map<string, Decimal>,
	symbol: string,
	amount: Decimal,
): Decimal | null {
	const factor = factors.get(normalizeSymbol(symbol))
	return factor ? amount.mul(factor) : null
}
