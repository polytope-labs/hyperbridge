import { describe, expect, it } from "vitest"
import {
	bucketLabel,
	describeNoSpread,
	formatPercent,
	formatRate,
	formatUsd,
	formatUsdShort,
	labelledBuckets,
	niceTicks,
	runningTotal,
	toneOf,
} from "./analyticsModel"

describe("figures", () => {
	it("keeps cents only where they are a real part of the figure", () => {
		expect(formatUsd(932_400)).toBe("$932,400")
		expect(formatUsd(63.2)).toBe("$63.20")
		expect(formatUsd(999.994)).toBe("$999.99")
		expect(formatUsd(0)).toBe("$0.00")
	})

	it("marks a gain as well as a loss when a figure could be either", () => {
		expect(formatUsd(5409.4, { signed: true })).toBe("+$5,409")
		expect(formatUsd(-59.1, { signed: true })).toBe("-$59.10")
		expect(formatUsd(-59.1)).toBe("-$59.10")
		// Nothing, and a loss too small to show, carry no sign.
		expect(formatUsd(0, { signed: true })).toBe("$0.00")
		expect(formatUsd(-0.001, { signed: true })).toBe("$0.00")
	})

	it("shortens a figure for an axis", () => {
		expect(formatUsdShort(2000)).toBe("$2K")
		expect(formatUsdShort(-150)).toBe("-$150")
		expect(formatUsdShort(1_250_000)).toBe("$1.3M")
		expect(formatUsdShort(0)).toBe("$0")
	})

	it("states a rate to the places that tell two rates apart", () => {
		expect(formatRate(1579.8821)).toBe("1,579.88")
		expect(formatRate(1590)).toBe("1,590.00")
		expect(formatRate(1.08423)).toBe("1.0842")
		expect(formatRate(0.000632911)).toBe("0.00063291")
	})

	it("states a spread to two places, or a dash when there is none", () => {
		expect(formatPercent(0.584)).toBe("+0.58%")
		expect(formatPercent(-0.2149)).toBe("-0.21%")
		expect(formatPercent(0)).toBe("0.00%")
		expect(formatPercent(-0.001)).toBe("0.00%")
		expect(formatPercent(null)).toBe("—")
	})

	it("reads a gain as good and a loss as bad", () => {
		expect(toneOf(12)).toBe("ok")
		expect(toneOf(-12)).toBe("err")
		expect(toneOf(0)).toBe("")
		expect(toneOf(null)).toBe("")
	})

	it("says why a period has no spread", () => {
		expect(describeNoSpread({ buys: 0, sells: 0 })).toBe("No fills yet")
		expect(describeNoSpread({ buys: 3, sells: 0 })).toBe("Buys only, nothing sold yet")
		expect(describeNoSpread({ buys: 0, sells: 2 })).toBe("Sells only, nothing bought back yet")
		// Both sides traded, on different pairs.
		expect(describeNoSpread({ buys: 1, sells: 1 })).toBe("Nothing matched yet")
	})
})

describe("buckets", () => {
	// Noon, so the local date is the same in every zone the suite might run in.
	const start = new Date(2026, 9, 5, 12).getTime()

	it("names a bucket for a table row and, shorter, for an axis", () => {
		expect(bucketLabel(start, "day")).toBe("Oct 5")
		expect(bucketLabel(start, "week")).toBe("Week of Oct 5")
		expect(bucketLabel(start, "week", "axis")).toBe("Oct 5")
		expect(bucketLabel(start, "month")).toBe("Oct 2026")
		expect(bucketLabel(start, "month", "axis")).toBe("Oct")
		expect(bucketLabel(start, "year")).toBe("2026")
	})

	it("adds each bucket's profit to the total before it", () => {
		expect(
			runningTotal([{ realizedUsd: 100 }, { realizedUsd: -40 }, { realizedUsd: 0 }, { realizedUsd: 15 }]),
		).toEqual([100, 60, 60, 75])
		expect(runningTotal([])).toEqual([])
	})

	it("labels every nth bucket, counted back from the latest", () => {
		expect([...labelledBuckets(7, 10)].sort((a, b) => a - b)).toEqual([0, 1, 2, 3, 4, 5, 6])
		expect([...labelledBuckets(30, 7)].sort((a, b) => a - b)).toEqual([4, 9, 14, 19, 24, 29])
		// No room at all still names the latest.
		expect([...labelledBuckets(12, 0)]).toEqual([11])
	})
})

describe("an axis", () => {
	it("rules at round values that cover the data and include zero", () => {
		expect(niceTicks(0, 5409)).toEqual([0, 2000, 4000, 6000])
		expect(niceTicks(-71, 286)).toEqual([-100, 0, 100, 200, 300])
		expect(niceTicks(120, 480)).toEqual([0, 200, 400, 600])
		expect(niceTicks(-900, -100)).toEqual([-1000, -500, 0])
	})

	it("keeps fractions exact", () => {
		expect(niceTicks(0, 0.63)).toEqual([0, 0.2, 0.4, 0.6, 0.8])
	})

	it("gives a flat series something to sit on", () => {
		expect(niceTicks(0, 0)).toEqual([0, 1])
	})
})
