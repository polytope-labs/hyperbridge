import type { ProfitBucket, ProfitBucketFigures, ProfitFigures, ProfitPeriod } from "../../types"

/** The periods the page offers, shortest first. Each is bucketed at its own size by the server. */
export const PERIODS: ReadonlyArray<{ value: ProfitPeriod; label: string }> = [
	{ value: "7d", label: "7 days" },
	{ value: "30d", label: "30 days" },
	{ value: "12w", label: "12 weeks" },
	{ value: "12m", label: "12 months" },
	{ value: "all", label: "All time" },
]

/** A bucket as a word: the heading over the table of them, and what its pager counts. */
export const BUCKET_NOUNS: Record<ProfitBucket, { one: string; many: string }> = {
	day: { one: "day", many: "days" },
	week: { one: "week", many: "weeks" },
	month: { one: "month", many: "months" },
	year: { one: "year", many: "years" },
}

export type Tone = "" | "ok" | "err"

/** Green for a gain and red for a loss. Nothing, and exactly nothing, stay neutral. */
export function toneOf(value: number | null): Tone {
	if (value === null || value === 0) return ""
	return value > 0 ? "ok" : "err"
}

/**
 * "$932,400", "$63.20", "+$5,409", "-$59.10". Cents are kept under a thousand dollars, where they
 * are a real part of the figure, and dropped above it. `signed` marks a gain as well as a loss,
 * for a figure that could be either.
 */
export function formatUsd(value: number, options: { signed?: boolean } = {}): string {
	const size = Math.abs(value)
	const digits = size < 1000 ? 2 : 0
	const figure = size.toLocaleString(undefined, { minimumFractionDigits: digits, maximumFractionDigits: digits })
	// A loss too small to show is not a loss worth a sign.
	const shown = Number(size.toFixed(digits)) !== 0
	const sign = !shown ? "" : value < 0 ? "-" : options.signed ? "+" : ""
	return `${sign}$${figure}`
}

/** "$2K", "-$150", "$1.2M": a dollar figure short enough for an axis. */
export function formatUsdShort(value: number): string {
	const figure = Math.abs(value).toLocaleString(undefined, { notation: "compact", maximumFractionDigits: 1 })
	return `${value < 0 ? "-" : ""}$${figure}`
}

/**
 * A rate in quote per base, to the places that tell two rates apart: "1,579.88", "1.0842",
 * "0.00063291". An average is not a rate anyone typed, so it has no natural length of its own.
 */
export function formatRate(value: number): string {
	if (value >= 100) return value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })
	if (value >= 1) return value.toLocaleString(undefined, { maximumFractionDigits: 4 })
	return value.toLocaleString(undefined, { maximumSignificantDigits: 5 })
}

/** "+0.58%", "-0.21%", and a dash when there is no spread to state. */
export function formatPercent(value: number | null): string {
	if (value === null) return "—"
	const figure = Math.abs(value).toFixed(2)
	const sign = Number(figure) === 0 ? "" : value < 0 ? "-" : "+"
	return `${sign}${figure}%`
}

/**
 * Why a stretch has no return. A return needs a profit and an inventory to compare it with: no
 * fills leaves nothing to compare, and without a record of what was held when the stretch began
 * there is nothing to compare it with.
 */
export function describeNoReturn(figures: Pick<ProfitFigures, "buys" | "sells" | "startInventoryUsd">): string {
	if (figures.buys === 0 && figures.sells === 0) return "No fills yet"
	if (figures.startInventoryUsd === null) return "Starting inventory not on record"
	return "Nothing was held at the start"
}

/** "+12,609 USDC", "-19,921,000 cNGN": a change in what is held, signed, and a dash for none. */
export function formatChange(amount: number, symbol: string): string {
	const figure = Math.abs(amount).toLocaleString(undefined, { maximumFractionDigits: 4 })
	if (Number(figure.replace(/[^0-9.]/g, "")) === 0) return "—"
	return `${amount < 0 ? "-" : "+"}${figure} ${symbol}`
}

/** "43,450 USDC, 6,210 USDT and 21,480,000 cNGN": what an inventory is made of, largest first. */
export function describeHoldings(tokens: ReadonlyArray<{ symbol: string; amount: number }>): string {
	const parts = tokens.map(
		(token) => `${token.amount.toLocaleString(undefined, { maximumFractionDigits: 2 })} ${token.symbol}`,
	)
	if (parts.length <= 1) return parts[0] ?? ""
	return `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}`
}

/**
 * A bucket named for a table row or, shorter, for a chart's axis: "Oct 5", "Week of Oct 5" or
 * "Oct 5", "Oct 2026" or "Oct", "2026". In the viewer's locale and on their clock, which is the
 * clock the server cut the buckets on.
 */
export function bucketLabel(start: number, bucket: ProfitBucket, style: "row" | "axis" = "row"): string {
	const date = new Date(start)
	if (bucket === "year") return String(date.getFullYear())
	if (bucket === "month") {
		return date.toLocaleDateString(
			undefined,
			style === "axis" ? { month: "short" } : { month: "short", year: "numeric" },
		)
	}
	const day = date.toLocaleDateString(undefined, { day: "numeric", month: "short" })
	return bucket === "week" && style === "row" ? `Week of ${day}` : day
}

/** The profit to date at the end of each bucket. */
export function runningTotal(series: ReadonlyArray<Pick<ProfitBucketFigures, "profitUsd">>): number[] {
	let total = 0
	return series.map((entry) => {
		total += entry.profitUsd
		return total
	})
}

/**
 * Round values to rule a chart at: about `count` of them, covering `min` to `max` and always
 * zero, at a step of 1, 2 or 5 times a power of ten. A flat series still gets an axis to sit on.
 */
export function niceTicks(min: number, max: number, count = 4): number[] {
	const low = Math.min(0, min)
	const high = Math.max(0, max)
	if (high === low) return [0, 1]

	const rough = (high - low) / count
	const magnitude = 10 ** Math.floor(Math.log10(rough))
	const step = [1, 2, 5, 10].map((factor) => factor * magnitude).find((candidate) => candidate >= rough) ?? rough

	const ticks: number[] = []
	for (let index = Math.floor(low / step); index * step < high + step; index++) {
		// Multiplying a whole index keeps 0.1 + 0.2 out of the labels.
		ticks.push(Number((index * step).toPrecision(12)))
		if (index * step >= high) break
	}
	return ticks
}

/**
 * Which buckets carry a label under the chart, given room for `fit` of them: every nth, counted
 * back from the latest so the one the eye lands on is always named.
 */
export function labelledBuckets(count: number, fit: number): Set<number> {
	const every = Math.max(1, Math.ceil(count / Math.max(1, fit)))
	const labelled = new Set<number>()
	for (let index = count - 1; index >= 0; index -= every) labelled.add(index)
	return labelled
}
