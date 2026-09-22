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
			{ amount: "100000", value: 5 },
		],
	}, // Polygon (~2s blocks; milestone finality lands in ~5s, so 5 blocks ≈ 10s covers it)
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

/** Parses to Decimal, dropping unparseable/non-finite points (a wizard editor mid-typing). */
/** A confirmation curve's points, parsed once at construction. */
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

	constructor(config: CurveConfig, label = "Interpolated curve") {
		this.label = label

		if (!config.points || config.points.length < 2) {
			throw new Error(`${label}: must have at least 2 points to define a curve`)
		}

		this.points = config.points
			.map((p) => ({
				amount: Number.parseFloat(p.amount),
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

/** The canonical "EVM-<id>" state machine id `parseChainKey` accepts. */
export function formatChainKey(chainId: number | string): string {
	return `EVM-${chainId}`
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
	 * Installs a curve for a chain added at runtime. Validates before it mutates,
	 * so a rejected curve leaves the policy set untouched — a chain that started
	 * scanning without a curve would drop every cross-chain order sourced on it
	 * with a per-order throw.
	 */
	add(chainId: number, config: CurveConfig): void {
		const curve = new InterpolatedCurve(config, `Chain ${chainId} confirmation policy`)
		this.policies.set(chainId, curve)
	}

	/** Whether a chain has a confirmation curve. */
	has(chainId: number): boolean {
		return this.policies.has(chainId)
	}

	remove(chainId: number): void {
		this.policies.delete(chainId)
	}

	/**
	 * Copies one chain's curve across from another policy set.
	 *
	 * Used when a chain is added at runtime and covered by a built-in default: the
	 * defaults are only materialised inside a freshly constructed policy, so the
	 * live one adopts the entry rather than re-deriving it.
	 */
	adopt(chainId: number, from: ConfirmationPolicy): void {
		const curve = from.policies.get(chainId)
		if (curve) this.policies.set(chainId, curve)
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
