import type { AdminStrategyDto } from "../types"

/**
 * HyperFX "solver links": a swap page locked to one solver, one token pair and
 * fixed rates, in the form the app's parser expects (`wl=1&wlv=1`, then
 * `source`/`destination` chain ids, `from`/`to` symbols which must equal
 * `rate_base`/`rate_quote`, `rate` in `to` per `from`, an optional
 * `reverse_rate` for the other direction in the same unit, the solver's
 * address and a display name of at most 24 characters).
 */
export const HYPERFX_APP_URL = "https://app.hyperfx.finance"
export const SOLVER_NAME_MAX = 24

export interface SolverLinkInput {
	chainId: number
	from: string
	to: string
	/** `to` per one `from`. */
	rate: string
	/** For the `to` → `from` direction, still expressed as `to` per one `from`. */
	reverseRate?: string
	solverAddress: string
	solverName: string
}

export function buildSolverLink(input: SolverLinkInput): string {
	const url = new URL("/swap", HYPERFX_APP_URL)
	const params = url.searchParams
	params.set("wl", "1")
	params.set("wlv", "1")
	params.set("source", String(input.chainId))
	params.set("destination", String(input.chainId))
	params.set("from", input.from.toUpperCase())
	params.set("to", input.to.toUpperCase())
	params.set("rate_base", input.from.toUpperCase())
	params.set("rate_quote", input.to.toUpperCase())
	params.set("rate", input.rate)
	if (input.reverseRate) params.set("reverse_rate", input.reverseRate)
	params.set("solver", input.solverAddress)
	params.set("solver_name", input.solverName.trim().slice(0, SOLVER_NAME_MAX))
	return url.toString()
}

export function isAddress(value: string | undefined): value is string {
	return !!value && /^0x[0-9a-fA-F]{40}$/.test(value)
}

function firstPrice(points: AdminStrategyDto["bid"]): string | undefined {
	return points?.find((point) => Number(point.price) > 0 && Number(point.amount) >= 0)?.price.trim()
}

/** Formats a reciprocal to six significant digits without float noise: 1/1373 → "0.000728332". */
function reciprocal(price: string): string {
	return (1 / Number(price)).toPrecision(6).replace(/\.?0+$/, "")
}

export type SolverLinkPlan =
	| { ok: true; from: string; to: string; rate: string; reverseRate?: string; link: string }
	| { ok: false; reason: string }

/**
 * A solver link for one Simplex market. Curves price token1 per token0: the ask
 * is what the solver sells token1 for, so a link that sells `token0` → `token1`
 * quotes the ask as `rate` and the bid (solver buying token1 back) as the
 * reverse. A bid-only market flips the direction, with the reciprocal bid.
 */
export function planSolverLink(
	strategy: AdminStrategyDto,
	chainId: number,
	solverAddress: string | undefined,
	solverName: string,
): SolverLinkPlan {
	if (!isAddress(solverAddress)) return { ok: false, reason: "The filler's EVM address is not known yet." }
	if (strategy.sameToken || strategy.referenceOnly) {
		return { ok: false, reason: "Solver links are for FX pairs, not same-asset or reference markets." }
	}
	if (strategy.pricingMode !== "static") {
		return { ok: false, reason: "This market is priced by a venue; a link needs fixed curve prices." }
	}
	const ask = firstPrice(strategy.ask)
	const bid = firstPrice(strategy.bid)
	if (!ask && !bid) return { ok: false, reason: "This market has no configured price yet." }
	const base = { chainId, solverAddress, solverName }
	if (ask) {
		const plan = { from: strategy.token0, to: strategy.token1, rate: ask, reverseRate: bid }
		return { ok: true, ...plan, link: buildSolverLink({ ...base, ...plan }) }
	}
	// Bid only: the solver buys token1, so the link sells token1 → token0 at the reciprocal.
	const plan = { from: strategy.token1, to: strategy.token0, rate: reciprocal(bid as string) }
	return { ok: true, ...plan, link: buildSolverLink({ ...base, ...plan }) }
}
