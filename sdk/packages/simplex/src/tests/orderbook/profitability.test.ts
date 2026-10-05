import { describe, expect, it } from "vitest"
import type { LimitOrderFillRecord } from "@/data/types"
import { isProfitPeriod, summarizeProfit } from "@/orderbook/profitability"

const ONE = 10n ** 18n
/** A whole-token figure at 1e18, exact to six decimals. */
const scaled = (value: number) => ((BigInt(Math.round(value * 1e6)) * ONE) / 1_000_000n).toString()

let nextId = 1
type Overrides = Partial<LimitOrderFillRecord>

/** Buying `base` at `rate`: pays base × rate of quote out and takes the base in. */
function buy(base: number, rate: number, filledAt: string, overrides: Overrides = {}): LimitOrderFillRecord {
	return {
		id: nextId++,
		limitOrderId: "limit-buy",
		book: "USDC-cNGN",
		base: "USDC",
		quote: "cNGN",
		side: "BID",
		price: scaled(rate),
		amount: scaled(base * rate),
		amountIn: scaled(base),
		filledAt,
		...overrides,
	}
}

/** Selling `base` at `rate`: pays the base out and takes base × rate of quote in. */
function sell(base: number, rate: number, filledAt: string, overrides: Overrides = {}): LimitOrderFillRecord {
	return {
		id: nextId++,
		limitOrderId: "limit-sell",
		book: "USDC-cNGN",
		base: "USDC",
		quote: "cNGN",
		side: "ASK",
		price: scaled(rate),
		amount: scaled(base),
		amountIn: scaled(base * rate),
		filledAt,
		...overrides,
	}
}

const NOW = Date.parse("2026-10-05T12:00:00Z")
const week = (fills: LimitOrderFillRecord[], tzOffsetMinutes = 0) =>
	summarizeProfit(fills, { period: "7d", now: NOW, tzOffsetMinutes })

describe("profit from buys against sells", () => {
	it("realizes the spread when bought volume is sold again", () => {
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), sell(100, 1590, "2026-10-05 09:00:00")])

		// 100 USDC bought at 1,580 and sold at 1,590 leaves 1,000 cNGN, which is 1000 ÷ 1590 dollars
		// at the rate of the sale that realized it.
		expect(summary.books[0]).toMatchObject({ bought: 100, sold: 100, matched: 100, realized: 1000, position: 0 })
		expect(summary.totals.realizedUsd).toBeCloseTo(1000 / 1590, 9)
		expect(summary.totals.matchedUsd).toBeCloseTo(100, 9)
		expect(summary.totals.spreadPct).toBeCloseTo(100 * (10 / 1590), 9)
		expect(summary.totals).toMatchObject({ buys: 1, sells: 1 })
	})

	it("realizes nothing while only one side has traded", () => {
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), buy(50, 1582, "2026-10-05 09:00:00")])

		expect(summary.totals).toMatchObject({ realizedUsd: 0, matchedUsd: 0, spreadPct: null, buys: 2, sells: 0 })
		expect(summary.totals.boughtUsd).toBeCloseTo(150, 9)
		// Bought and not yet sold, and dollars at face since the base is one.
		expect(summary.books[0]).toMatchObject({ position: 150, positionUsd: 150 })
		expect(summary.openPositionUsd).toBe(150)
	})

	it("closes against the average of what was bought, not the first or the last", () => {
		const summary = week([
			buy(100, 1580, "2026-10-03 09:00:00"),
			buy(100, 1600, "2026-10-04 09:00:00"),
			sell(100, 1595, "2026-10-05 09:00:00"),
		])

		// The 200 held cost 1,590 on average, so selling 100 at 1,595 earns 5 each.
		expect(summary.books[0]).toMatchObject({ matched: 100, realized: 500, position: 100 })
		expect(summary.books[0].averageBuy).toBe(1590)
		expect(summary.books[0].averageSell).toBe(1595)
	})

	it("counts selling first and buying back the same way round", () => {
		const summary = week([sell(100, 1590, "2026-10-04 09:00:00"), buy(100, 1580, "2026-10-05 09:00:00")])

		expect(summary.books[0]).toMatchObject({ matched: 100, realized: 1000, position: 0 })
		expect(summary.totals.realizedUsd).toBeCloseTo(1000 / 1580, 9)
	})

	it("shows a loss when volume is sold under what it cost", () => {
		const summary = week([buy(100, 1590, "2026-10-04 09:00:00"), sell(100, 1585, "2026-10-05 09:00:00")])

		expect(summary.books[0].realized).toBe(-500)
		expect(summary.totals.realizedUsd).toBeLessThan(0)
		expect(summary.totals.spreadPct).toBeLessThan(0)
	})

	it("opens the other way at the fill's own rate once it has closed what was held", () => {
		const summary = week([
			buy(50, 1580, "2026-10-03 09:00:00"),
			sell(80, 1590, "2026-10-04 09:00:00"),
			buy(30, 1585, "2026-10-05 09:00:00"),
		])

		// The sale closes the 50 held for 10 each and leaves 30 sold at 1,590; buying those back
		// at 1,585 earns 5 each.
		expect(summary.books[0]).toMatchObject({ matched: 80, realized: 500 + 150, position: 0 })
	})

	it("prices a fill that kept no record of what it took in at its order's rate", () => {
		const legacy = sell(100, 1590, "2026-10-05 09:00:00", { amountIn: null })
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), legacy])

		expect(summary.books[0].realized).toBe(1000)
		expect(summary.estimatedFills).toBe(1)
	})

	it("uses what a fill took in over the order's rate when it has both", () => {
		// The order asks 1,590, and the swapper's rate released 1,592 a unit.
		const better = sell(100, 1590, "2026-10-05 09:00:00", { amountIn: scaled(159_200) })
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), better])

		expect(summary.books[0].realized).toBe(1200)
		expect(summary.books[0].averageSell).toBe(1592)
		expect(summary.estimatedFills).toBe(0)
	})

	it("skips a fill it cannot price rather than failing the whole summary", () => {
		const broken = buy(100, 1580, "2026-10-04 09:00:00", { amount: "not a number" })
		const unpriced = sell(100, 1590, "2026-10-04 10:00:00", { price: "0", amountIn: null })
		const summary = week([broken, unpriced, buy(100, 1580, "2026-10-05 09:00:00")])

		expect(summary.totals).toMatchObject({ buys: 1, sells: 0 })
	})
})

describe("the period a summary covers", () => {
	it("closes against volume bought before the period began", () => {
		const summary = week([buy(100, 1580, "2026-08-20 09:00:00"), sell(100, 1590, "2026-10-05 09:00:00")])

		// The buy is outside the week, so it is not counted as bought, but it set the cost.
		expect(summary.books[0]).toMatchObject({ bought: 0, sold: 100, matched: 100, realized: 1000 })
		expect(summary.totals).toMatchObject({ buys: 0, sells: 1 })
	})

	it("counts profit in the bucket of the fill that realized it", () => {
		const summary = week([buy(100, 1580, "2026-10-01 09:00:00"), sell(100, 1590, "2026-10-04 09:00:00")])

		expect(summary.bucket).toBe("day")
		expect(summary.series.map((day) => new Date(day.start).toISOString().slice(0, 10))).toEqual([
			"2026-09-29",
			"2026-09-30",
			"2026-10-01",
			"2026-10-02",
			"2026-10-03",
			"2026-10-04",
			"2026-10-05",
		])
		const profits = summary.series.map((day) => day.realizedUsd)
		expect(profits.slice(0, 5)).toEqual([0, 0, 0, 0, 0])
		expect(profits[5]).toBeCloseTo(1000 / 1590, 9)
		expect(summary.series[2]).toMatchObject({ buys: 1, sells: 0 })
		// The buckets add up to the period.
		expect(profits.reduce((sum, value) => sum + value, 0)).toBeCloseTo(summary.totals.realizedUsd, 9)
	})

	it("starts a day at the viewer's midnight", () => {
		// 23:30 UTC on the 4th is already the 5th in Lagos, an hour ahead.
		const fills = [buy(100, 1580, "2026-10-04 22:00:00"), sell(100, 1590, "2026-10-04 23:30:00")]

		const utc = week(fills)
		expect(utc.series[5].sells).toBe(1)
		expect(utc.series[6].sells).toBe(0)

		const lagos = week(fills, -60)
		expect(lagos.series[5]).toMatchObject({ buys: 1, sells: 0 })
		expect(lagos.series[6].sells).toBe(1)
		// A bucket's start is a real instant: Lagos midnight is 23:00 UTC the day before.
		expect(new Date(lagos.series[6].start).toISOString()).toBe("2026-10-04T23:00:00.000Z")
	})

	it("buckets each period at its own size", () => {
		const fills = [buy(100, 1580, "2025-11-20 09:00:00")]
		const at = (period: "30d" | "12w" | "12m" | "all") => summarizeProfit(fills, { period, now: NOW })
		const starts = (period: "30d" | "12w" | "12m" | "all") =>
			at(period).series.map((entry) => new Date(entry.start).toISOString().slice(0, 10))

		expect(at("30d").bucket).toBe("day")
		expect(starts("30d")).toHaveLength(30)
		expect(starts("30d")[0]).toBe("2026-09-06")

		// Weeks start on Monday; the 5th of October 2026 is one.
		expect(at("12w").bucket).toBe("week")
		expect(starts("12w")).toHaveLength(12)
		expect(starts("12w").at(-1)).toBe("2026-10-05")
		expect(starts("12w")[0]).toBe("2026-07-20")

		expect(at("12m").bucket).toBe("month")
		expect(starts("12m")).toEqual([
			"2025-11-01",
			"2025-12-01",
			"2026-01-01",
			"2026-02-01",
			"2026-03-01",
			"2026-04-01",
			"2026-05-01",
			"2026-06-01",
			"2026-07-01",
			"2026-08-01",
			"2026-09-01",
			"2026-10-01",
		])

		// "all" runs by year from the first fill.
		expect(at("all").bucket).toBe("year")
		expect(starts("all")).toEqual(["2025-01-01", "2026-01-01"])
	})

	it("is one empty bucket when nothing has ever filled", () => {
		const summary = summarizeProfit([], { period: "all", now: NOW })

		expect(summary.series).toHaveLength(1)
		expect(summary.totals).toMatchObject({ realizedUsd: 0, spreadPct: null, buys: 0, sells: 0 })
		expect(summary.books).toEqual([])
	})

	it("ignores a fill dated after the summary was taken", () => {
		const summary = week([buy(100, 1580, "2026-10-05 09:00:00"), sell(100, 1590, "2026-10-06 09:00:00")])

		expect(summary.totals).toMatchObject({ buys: 1, sells: 0 })
	})

	it("knows the periods it can be asked for", () => {
		expect(isProfitPeriod("12w")).toBe(true)
		expect(isProfitPeriod("1y")).toBe(false)
	})
})

describe("dollars", () => {
	const inverse = { book: "cNGN-USDC", base: "cNGN", quote: "USDC" }

	it("reads a quote that is a dollar stable at face", () => {
		// Buying 1,000,000 cNGN at 0.00063 USDC each and selling it at 0.00064.
		const summary = week([
			buy(1_000_000, 0.00063, "2026-10-04 09:00:00", inverse),
			sell(1_000_000, 0.00064, "2026-10-05 09:00:00", inverse),
		])

		expect(summary.totals.realizedUsd).toBeCloseTo(10, 9)
		expect(summary.totals.matchedUsd).toBeCloseTo(640, 9)
		expect(summary.totals.boughtUsd).toBeCloseTo(630, 9)
	})

	it("values held volume at what it cost when the base is not dollars", () => {
		const summary = week([buy(1_000_000, 0.00063, "2026-10-05 09:00:00", inverse)])

		expect(summary.books[0].positionUsd).toBeCloseTo(630, 9)
		expect(summary.openPositionUsd).toBeCloseTo(630, 9)
	})

	it("leaves a book with no dollar stable out of the dollar figures", () => {
		const euro = { book: "EURC-cNGN", base: "EURC", quote: "cNGN" }
		const summary = week([
			buy(100, 1850, "2026-10-04 09:00:00", euro),
			sell(100, 1860, "2026-10-05 09:00:00", euro),
			buy(100, 1580, "2026-10-04 09:00:00"),
			sell(100, 1590, "2026-10-05 09:00:00"),
		])

		const unpriced = summary.books.find((book) => book.book === "EURC-cNGN")
		expect(unpriced).toMatchObject({ realized: 1000, realizedUsd: null, positionUsd: null })
		// Its spread is still known, in its own tokens.
		expect(unpriced?.spreadPct).toBeCloseTo(100 * (10 / 1860), 9)
		expect(summary.unpricedBooks).toEqual(["EURC-cNGN"])
		expect(summary.totals.realizedUsd).toBeCloseTo(1000 / 1590, 9)
		// Its fills are still counted as fills.
		expect(summary.totals).toMatchObject({ buys: 2, sells: 2 })
	})

	it("realizes a same-asset book's spread fill by fill, with no position", () => {
		const same = { book: "USDC-USDC", base: "USDC", quote: "USDC" }
		// Paying 999 USDC out for every 1,000 taken in.
		const fill = buy(1000, 0.999, "2026-10-05 09:00:00", same)
		const summary = week([fill, { ...fill, id: nextId++ }])

		expect(summary.books[0]).toMatchObject({ realized: 2, position: 0 })
		expect(summary.totals.realizedUsd).toBeCloseTo(2, 9)
	})

	it("lists the book that earned the most first", () => {
		const usdt = { book: "USDT-cNGN", base: "USDT" }
		const summary = week([
			buy(100, 1580, "2026-10-04 09:00:00"),
			sell(100, 1582, "2026-10-05 09:00:00"),
			buy(100, 1580, "2026-10-04 09:00:00", usdt),
			sell(100, 1595, "2026-10-05 09:00:00", usdt),
		])

		expect(summary.books.map((book) => book.book)).toEqual(["USDT-cNGN", "USDC-cNGN"])
	})
})
