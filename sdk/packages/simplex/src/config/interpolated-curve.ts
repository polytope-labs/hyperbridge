import Decimal from "decimal.js"

/**
 * A coordinate point on a curve
 * @property amount - The input threshold (e.g., USD amount)
 * @property value - The output value at this threshold (e.g., confirmations, bps)
 */
export interface CurvePoint {
	amount: string
	value: number
}

/**
 * Configuration for a curve
 */
export interface CurveConfig {
	points: CurvePoint[]
}

/**
 * Built-in per-chain confirmation curves for the supported mainnets, merged
 * under any user-supplied `[confirmationPolicies]` entries at startup. The
 * curve amount axis is the order's USD value (derived from the pair curves
 * via the USD anchors); the value is the confirmation depth in blocks.
 */
export const DEFAULT_CONFIRMATION_POLICIES: Record<string, CurveConfig> = {
	"1": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 15 },
		],
	}, // Ethereum (~12s blocks, ~24s–3min)
	"56": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 3 },
		],
	}, // BNB Chain (~3s blocks, fast finality)
	"137": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 32 },
		],
	}, // Polygon (~2s blocks, milestone finality)
	"8453": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 90 },
		],
	}, // Base (~2s blocks, L2)
	"42161": {
		points: [
			{ amount: "1000", value: 8 },
			{ amount: "100000", value: 720 },
		],
	}, // Arbitrum (~0.25s blocks, L2)
	"130": {
		points: [
			{ amount: "1000", value: 2 },
			{ amount: "100000", value: 180 },
		],
	}, // Unichain (~1s blocks, OP-stack L2 — time-equivalent to Base's curve)
}

/**
 * A coordinate point on a price curve
 * @property amount - The input threshold (e.g., USD amount)
 * @property price - The exotic tokens per 1 USD at this threshold
 */
export interface PriceCurvePoint {
	amount: string
	price: string
}

/**
 * Configuration for a price curve
 */
export interface PriceCurveConfig {
	points: PriceCurvePoint[]
}

interface ParsedPoint {
	amount: number
	value: number
}

/**
 * A curve that interpolates values based on input amounts.
 * Uses piecewise linear interpolation between provided points.
 *
 * This is a generic utility used for:
 * - Confirmation blocks based on order value
 * - BPS (basis points) based on order value
 */
export class InterpolatedCurve {
	private points: ParsedPoint[]
	private label: string

	constructor(config: CurveConfig, label: string = "Interpolated curve") {
		this.label = label

		if (!config.points || config.points.length < 2) {
			throw new Error(`${label}: must have at least 2 points to define a curve`)
		}

		this.points = config.points
			.map((p) => ({
				amount: parseFloat(p.amount),
				value: p.value,
			}))
			.sort((a, b) => a.amount - b.amount)

		for (const point of this.points) {
			if (isNaN(point.amount) || point.amount < 0) {
				throw new Error(`${label}: invalid amount`)
			}
			if (!Number.isInteger(point.value) || point.value < 0) {
				throw new Error(`${label}: value must be a non-negative integer`)
			}
		}
	}

	getValue(inputAmount: Decimal | number): number {
		const amount = inputAmount instanceof Decimal ? inputAmount.toNumber() : inputAmount

		if (amount <= this.points[0].amount) {
			return this.points[0].value
		}
		if (amount >= this.points[this.points.length - 1].amount) {
			return this.points[this.points.length - 1].value
		}

		const result = this.linearInterpolate(amount)
		return Math.round(result)
	}

	/**
	 * Piecewise linear interpolation.
	 * Finds the two points the amount falls between and linearly interpolates.
	 */
	private linearInterpolate(amount: number): number {
		for (let i = 0; i < this.points.length - 1; i++) {
			const p1 = this.points[i]
			const p2 = this.points[i + 1]

			if (amount >= p1.amount && amount <= p2.amount) {
				const t = (amount - p1.amount) / (p2.amount - p1.amount)
				return p1.value + t * (p2.value - p1.value)
			}
		}

		return this.points[this.points.length - 1].value
	}
}

/**
 * Manages confirmation block requirements per chain.
 * Each chain has its own curve mapping order value to required confirmations.
 */
/**
 * Parses a per-chain config key. Every other table in the config writes chains
 * as state machine ids ("EVM-1"), so both "EVM-1" and bare "1" are accepted;
 * anything else returns null and callers MUST fail loudly — a silently
 * discarded chain key has meant committing capital on a chain configured as
 * watch-only, or hardening a confirmation curve that never applied.
 */
export function parseChainKey(key: string): number | null {
	const normalized = key.trim().replace(/^EVM-/i, "")
	if (!/^\d+$/.test(normalized)) return null
	return Number(normalized)
}

export class ConfirmationPolicy {
	private policies: Map<number, InterpolatedCurve>

	constructor(policyConfig: Record<string, CurveConfig>) {
		this.policies = new Map()

		Object.entries(policyConfig).forEach(([chainId, config]) => {
			const parsed = parseChainKey(chainId)
			if (parsed === null) {
				throw new Error(
					`Confirmation policy key '${chainId}' is not a chain id — write [confirmationPolicies."EVM-<id>"] or [confirmationPolicies."<id>"]`,
				)
			}
			const curve = new InterpolatedCurve(config, `Chain ${parsed} confirmation policy`)
			// Later entries override earlier ones after normalization, so a user
			// key "EVM-1" replaces the built-in default keyed "1" instead of
			// sitting beside it as a dead entry.
			this.policies.set(parsed, curve)
		})
	}

	getConfirmationBlocks(chainId: number, amountUsd: Decimal): number {
		const curve = this.policies.get(chainId)
		if (!curve) throw new Error(`No confirmation policy found for chainId ${chainId}`)
		return curve.getValue(amountUsd)
	}

	/**
	 * Startup guard: every configured chain must have a confirmation curve.
	 * Without this, a missing policy only surfaces as a per-order throw at
	 * fill time — cross-chain orders sourced on that chain silently dropped.
	 */
	assertCovers(chainIds: number[]): void {
		const missing = chainIds.filter((id) => !this.policies.has(id))
		if (missing.length > 0) {
			throw new Error(
				`No confirmation policy for chain(s) ${missing.join(", ")} — add [confirmationPolicies."<chainId>"] entries for them (built-in defaults cover Ethereum, BSC, Polygon, Base, Arbitrum and Unichain)`,
			)
		}
	}
}

/**
 * Manages filler basis points based on order value.
 * Uses linear interpolation to determine BPS for any order size.
 */
export class FillerBpsPolicy {
	private curve: InterpolatedCurve

	constructor(config: CurveConfig) {
		this.curve = new InterpolatedCurve(config, "Filler BPS policy")
	}

	getBps(orderValueUsd: Decimal): bigint {
		return BigInt(this.curve.getValue(orderValueUsd))
	}
}

/**
 * Manages cNGN prices (in USD) based on order value.
 * Uses piecewise linear interpolation between configured USD thresholds.
 */
export class FillerPricePolicy {
	private points: { amount: Decimal; price: Decimal }[]

	constructor(config: PriceCurveConfig) {
		this.points = FillerPricePolicy.parsePoints(config)
	}

	/** Validates and normalizes curve points; throws without side effects on invalid input. */
	private static parsePoints(config: PriceCurveConfig): { amount: Decimal; price: Decimal }[] {
		if (!config.points || config.points.length < 1) {
			throw new Error("Filler price policy: must have at least 1 point to define a curve")
		}

		const points = config.points
			.map((p) => ({
				amount: new Decimal(p.amount),
				price: new Decimal(p.price),
			}))
			.sort((a, b) => a.amount.comparedTo(b.amount))

		for (const point of points) {
			if (!point.amount.isFinite() || point.amount.isNegative()) {
				throw new Error("Filler price policy: invalid amount")
			}
			if (!point.price.isFinite() || !point.price.isPositive()) {
				throw new Error("Filler price policy: price must be a positive number")
			}
		}

		return points
	}

	/**
	 * Replaces the curve points in place, so every holder of this policy prices
	 * with the new curve from the next `getPrice` call. Validates the full point
	 * set before applying; throws on invalid input leaving the curve unchanged.
	 */
	replacePoints(config: PriceCurveConfig): void {
		this.points = FillerPricePolicy.parsePoints(config)
	}

	/** Replaces all curve points with a single flat price. */
	updatePrice(newPrice: Decimal): void {
		this.replacePoints({ points: [{ amount: "0", price: newPrice.toString() }] })
	}

	/**
	 * Startup guard for two-sided books: the bid must exceed the ask at every
	 * point of both curves' amount domains. A crossed or zero-spread book can
	 * never fill — the per-leg FX margin marks every fill at the opposite
	 * curve, so bid ≤ ask makes every round trip non-positive and the profit
	 * gate rejects all orders. Reject the config loudly instead of letting the
	 * pair go silently dead. Both curves are piecewise-linear, so checking
	 * every breakpoint of the combined amount grid covers the whole domain
	 * (clamped tails included).
	 */
	static assertBookNotCrossed(label: string, bid: FillerPricePolicy, ask: FillerPricePolicy): void {
		const amounts = [...new Set([...bid.getPoints(), ...ask.getPoints()].map((p) => p.amount))]
		for (const amount of amounts) {
			const at = new Decimal(amount)
			const bidPrice = bid.getPrice(at)
			const askPrice = ask.getPrice(at)
			if (bidPrice.lte(askPrice)) {
				throw new Error(
					`${label}: book is crossed at amount ${amount} — bid ${bidPrice.toString()} ≤ ask ${askPrice.toString()}. ` +
						`A non-positive spread can never fill (every fill marks as a loss at the opposite curve); ` +
						`skew both curves without crossing them, or use a one-sided pair for a deliberate directional position`,
				)
			}
		}
	}

	/** Current curve points, sorted by amount. */
	getPoints(): PriceCurvePoint[] {
		return this.points.map((p) => ({ amount: p.amount.toString(), price: p.price.toString() }))
	}

	getPrice(orderValueUsd: Decimal): Decimal {
		const amount = orderValueUsd

		// Below minimum configured amount, use the first point
		if (amount.lte(this.points[0].amount)) {
			return this.points[0].price
		}

		// Above maximum configured amount, use the last point
		const lastPoint = this.points[this.points.length - 1]
		if (amount.gte(lastPoint.amount)) {
			return lastPoint.price
		}

		// Piecewise linear interpolation between surrounding points
		for (let i = 0; i < this.points.length - 1; i++) {
			const p1 = this.points[i]
			const p2 = this.points[i + 1]

			if (amount.gte(p1.amount) && amount.lte(p2.amount)) {
				const t = amount.minus(p1.amount).div(p2.amount.minus(p1.amount))
				return p1.price.plus(t.mul(p2.price.minus(p1.price)))
			}
		}

		// Fallback (should not be reached due to earlier checks)
		return lastPoint.price
	}
}
