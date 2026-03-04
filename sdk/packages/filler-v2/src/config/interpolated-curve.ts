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
 * A coordinate point on a price curve
 * @property amount - The input threshold (e.g., USD amount)
 * @property priceUsd - The cNGN price in USD at this threshold
 */
export interface PriceCurvePoint {
	amount: string
	priceUsd: string
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
export class ConfirmationPolicy {
	private policies: Map<number, InterpolatedCurve>

	constructor(policyConfig: Record<string, CurveConfig>) {
		this.policies = new Map()

		Object.entries(policyConfig).forEach(([chainId, config]) => {
			const curve = new InterpolatedCurve(config, `Chain ${chainId} confirmation policy`)
			this.policies.set(Number(chainId), curve)
		})
	}

	getConfirmationBlocks(chainId: number, amountUsd: Decimal): number {
		const curve = this.policies.get(chainId)
		if (!curve) throw new Error(`No confirmation policy found for chainId ${chainId}`)
		return curve.getValue(amountUsd)
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
	private points: { amount: Decimal; priceUsd: Decimal }[]

	constructor(config: PriceCurveConfig) {
		if (!config.points || config.points.length < 1) {
			throw new Error("Filler price policy: must have at least 1 point to define a curve")
		}

		this.points = config.points
			.map((p) => ({
				amount: new Decimal(p.amount),
				priceUsd: new Decimal(p.priceUsd),
			}))
			.sort((a, b) => a.amount.comparedTo(b.amount))

		for (const point of this.points) {
			if (!point.amount.isFinite() || point.amount.isNegative()) {
				throw new Error("Filler price policy: invalid amount")
			}
			if (!point.priceUsd.isFinite() || !point.priceUsd.isPositive()) {
				throw new Error("Filler price policy: price must be a positive number")
			}
		}
	}

	getPrice(orderValueUsd: Decimal): Decimal {
		const amount = orderValueUsd

		// Below minimum configured amount, use the first point
		if (amount.lte(this.points[0].amount)) {
			return this.points[0].priceUsd
		}

		// Above maximum configured amount, use the last point
		const lastPoint = this.points[this.points.length - 1]
		if (amount.gte(lastPoint.amount)) {
			return lastPoint.priceUsd
		}

		// Piecewise linear interpolation between surrounding points
		for (let i = 0; i < this.points.length - 1; i++) {
			const p1 = this.points[i]
			const p2 = this.points[i + 1]

			if (amount.gte(p1.amount) && amount.lte(p2.amount)) {
				const t = amount.minus(p1.amount).div(p2.amount.minus(p1.amount))
				return p1.priceUsd.plus(t.mul(p2.priceUsd.minus(p1.priceUsd)))
			}
		}

		// Fallback (should not be reached due to earlier checks)
		return lastPoint.priceUsd
	}
}
