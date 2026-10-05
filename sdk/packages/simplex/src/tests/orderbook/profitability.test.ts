import { describe, expect, it } from "vitest"
import type { LimitOrderFillRecord } from "@/data/types"
import {
	BALANCES_REACH_MS,
	type InventoryReading,
	type InventoryRecord,
	inventoryInstants,
	isProfitPeriod,
	summarizeProfit,
} from "@/orderbook/profitability"

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

const HOUR = 3_600_000
const DAY = 24 * HOUR
const NOW = Date.parse("2026-10-05T12:00:00Z")
/** The week's first instant on a UTC clock: seven days of buckets ending with today's. */
const WEEK_START = Date.parse("2026-09-29T00:00:00Z")
const week = (fills: LimitOrderFillRecord[], tzOffsetMinutes = 0) =>
	summarizeProfit(fills, { period: "7d", now: NOW, tzOffsetMinutes })

describe("profit as the change fills made to inventory", () => {
	it("is the spread when what was bought is sold again", () => {
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), sell(100, 1590, "2026-10-05 09:00:00")])

		// The base is back where it started and the quote is up by 10 a unit: 1,000 cNGN, worth
		// 1000 ÷ 1585 dollars at the rate midway between the latest buy and the latest sell.
		expect(summary.books[0]).toMatchObject({ bought: 100, sold: 100, baseChange: 0, quoteChange: 1000, rate: 1585 })
		expect(summary.books[0].profitUsd).toBeCloseTo(1000 / 1585, 9)
		expect(summary.totals.profitUsd).toBeCloseTo(1000 / 1585, 9)
		expect(summary.totals).toMatchObject({ buys: 1, sells: 1 })
		expect(summary.totals.boughtUsd).toBeCloseTo(100, 9)
		expect(summary.totals.soldUsd).toBeCloseTo(100, 9)
	})

	it("states how far the average sell sits above the average buy", () => {
		const summary = week([
			buy(100, 1580, "2026-10-03 09:00:00"),
			buy(100, 1600, "2026-10-04 09:00:00"),
			sell(100, 1595, "2026-10-05 09:00:00"),
		])

		expect(summary.books[0].averageBuy).toBe(1590)
		expect(summary.books[0].averageSell).toBe(1595)
		expect(summary.books[0].spreadPct).toBeCloseTo(100 * (1595 / 1590 - 1), 9)
		// One side alone has no spread.
		expect(week([buy(100, 1580, "2026-10-05 09:00:00")]).books[0].spreadPct).toBeNull()
	})

	it("values volume the period left open at the latest rate", () => {
		// 200 bought and only 100 sold. The 100 still held cost 1,570 and are worth the latest
		// rate, midway between the last buy at 1,580 and the last sell at 1,590.
		const summary = week([
			buy(100, 1570, "2026-10-03 09:00:00"),
			buy(100, 1580, "2026-10-04 09:00:00"),
			sell(100, 1590, "2026-10-05 09:00:00"),
		])

		expect(summary.books[0]).toMatchObject({ baseChange: 100, quoteChange: 159_000 - 157_000 - 158_000 })
		expect(summary.totals.profitUsd).toBeCloseTo(100 - 156_000 / 1585, 9)
	})

	it("neither gains nor loses on a first buy valued at its own rate", () => {
		// Nothing has sold, so the latest rate is the buy's, and what was paid is what it is worth.
		const summary = week([buy(100, 1580, "2026-10-05 09:00:00")])

		expect(summary.books[0]).toMatchObject({ baseChange: 100, quoteChange: -158_000, rate: 1580 })
		expect(summary.totals.profitUsd).toBeCloseTo(0, 9)
		expect(summary.totals).toMatchObject({ buys: 1, sells: 0 })
	})

	it("shows a loss when volume is sold under what it was bought for", () => {
		const summary = week([buy(100, 1590, "2026-10-04 09:00:00"), sell(100, 1585, "2026-10-05 09:00:00")])

		expect(summary.books[0].quoteChange).toBe(-500)
		expect(summary.totals.profitUsd).toBeCloseTo(-500 / 1587.5, 9)
	})

	it("prices a fill that kept no record of what it took in at its order's rate", () => {
		const legacy = sell(100, 1590, "2026-10-05 09:00:00", { amountIn: null })
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), legacy])

		expect(summary.books[0].quoteChange).toBe(1000)
		expect(summary.estimatedFills).toBe(1)
	})

	it("uses what a fill took in over the order's rate when it has both", () => {
		// The order asks 1,590, and the swapper's rate released 1,592 a unit.
		const better = sell(100, 1590, "2026-10-05 09:00:00", { amountIn: scaled(159_200) })
		const summary = week([buy(100, 1580, "2026-10-04 09:00:00"), better])

		expect(summary.books[0].quoteChange).toBe(1200)
		expect(summary.books[0].averageSell).toBe(1592)
		expect(summary.estimatedFills).toBe(0)
	})

	it("skips a fill it cannot price rather than failing the whole summary", () => {
		const broken = buy(100, 1580, "2026-10-04 09:00:00", { amount: "not a number" })
		const unpriced = sell(100, 1590, "2026-10-04 10:00:00", { price: "0", amountIn: null })
		const summary = week([broken, unpriced, buy(100, 1580, "2026-10-05 09:00:00")])

		expect(summary.totals).toMatchObject({ buys: 1, sells: 0 })
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
		// Each token has one price whichever book moved it, so the books add up to the total.
		const byBook = summary.books.reduce((sum, book) => sum + (book.profitUsd ?? 0), 0)
		expect(byBook).toBeCloseTo(summary.totals.profitUsd, 9)
	})
})

describe("the period a summary covers", () => {
	it("counts only the period's fills, at a rate the fills before it still inform", () => {
		const summary = week([buy(100, 1580, "2026-08-20 09:00:00"), sell(100, 1590, "2026-10-05 09:00:00")])

		// The buy is outside the week: only the sale changed this week's inventory. It is valued
		// midway between that sale and the last buy, old as it is.
		expect(summary.books[0]).toMatchObject({ bought: 0, sold: 100, baseChange: -100, quoteChange: 159_000 })
		expect(summary.books[0].rate).toBe(1585)
		expect(summary.totals.profitUsd).toBeCloseTo(-100 + 159_000 / 1585, 9)
		expect(summary.totals).toMatchObject({ buys: 0, sells: 1 })
	})

	it("counts each fill in its own bucket, and the buckets add up to the period", () => {
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
		const profits = summary.series.map((day) => day.profitUsd)
		// Bought five under the latest rate on the 1st, sold five over it on the 4th.
		expect(profits[2]).toBeCloseTo(100 - 158_000 / 1585, 9)
		expect(profits[5]).toBeCloseTo(-100 + 159_000 / 1585, 9)
		expect(profits.filter((profit) => profit === 0)).toHaveLength(5)
		expect(summary.series[2]).toMatchObject({ buys: 1, sells: 0 })
		expect(profits.reduce((sum, value) => sum + value, 0)).toBeCloseTo(summary.totals.profitUsd, 9)
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
		expect(summary.totals).toMatchObject({ profitUsd: 0, returnPct: null, buys: 0, sells: 0 })
		expect(summary.books).toEqual([])
		expect(summary.startInventory).toBeNull()
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

describe("the inventory a period began with", () => {
	/** A buy and a sell inside the week: 50 USDC more held, and 78,500 cNGN less. */
	const fills = () => [buy(100, 1580, "2026-10-02 09:00:00"), sell(50, 1590, "2026-10-04 09:00:00")]
	const live: InventoryRecord = { at: NOW, balances: { USDC: 1000, cNGN: 1_000_000 }, live: true }
	const withInventory = (
		inventory: InventoryRecord[],
		extra: { period?: "7d" | "30d"; outflows?: Array<{ at: number; symbol: string; amount: number }> } = {},
	) => summarizeProfit(fills(), { period: extra.period ?? "7d", now: NOW, inventory, outflows: extra.outflows })

	it("rebuilds it from today's balances by undoing the period's fills", () => {
		const summary = withInventory([live])

		// Before the buy there were 100 fewer USDC and 158,000 more cNGN; before the sale, 50 more
		// USDC and 79,500 fewer cNGN.
		expect(summary.startInventory).toMatchObject({ at: WEEK_START, source: "balances", recordedAt: NOW })
		expect(summary.startInventory?.tokens).toEqual([
			{ symbol: "USDC", amount: 950, usd: 950 },
			{ symbol: "cNGN", amount: 1_078_500, usd: 1_078_500 / 1585 },
		])
		expect(summary.startInventory?.usd).toBeCloseTo(950 + 1_078_500 / 1585, 9)
	})

	it("compares the period's profit with it", () => {
		const { totals } = withInventory([live])

		const profit = 50 - 78_500 / 1585
		const began = 950 + 1_078_500 / 1585
		expect(totals.profitUsd).toBeCloseTo(profit, 9)
		expect(totals.startInventoryUsd).toBeCloseTo(began, 9)
		expect(totals.returnPct).toBeCloseTo((100 * profit) / began, 9)
		// The profit is the whole of the difference between then and now, at one rate.
		expect(began + profit).toBeCloseTo(1000 + 1_000_000 / 1585, 9)
	})

	it("carries a stored snapshot forward by the fills between it and the period's start", () => {
		// Taken two days before the week began, with a sale of 20 USDC in between.
		const before = sell(20, 1590, "2026-09-28 09:00:00")
		const snapshot: InventoryRecord = { at: WEEK_START - 2 * DAY, balances: { USDC: 900, cNGN: 1_200_000 } }
		const summary = summarizeProfit([before, ...fills()], { period: "7d", now: NOW, inventory: [snapshot] })

		expect(summary.startInventory).toMatchObject({ source: "snapshot", recordedAt: snapshot.at })
		expect(summary.startInventory?.tokens.map(({ symbol, amount }) => [symbol, amount])).toEqual([
			["USDC", 880],
			["cNGN", 1_231_800],
		])
	})

	it("uses whichever record is nearest the period's start", () => {
		const near: InventoryRecord = { at: WEEK_START + HOUR, balances: { USDC: 700, cNGN: 0 } }
		const far: InventoryRecord = { at: WEEK_START - 3 * DAY, balances: { USDC: 5, cNGN: 0 } }
		const summary = withInventory([far, near, live])

		expect(summary.startInventory).toMatchObject({ source: "snapshot", recordedAt: near.at })
		expect(summary.startInventory?.tokens[0]).toMatchObject({ symbol: "USDC", amount: 700 })
	})

	it("carries today's balances back seven days and no further", () => {
		// Thirty days back, with only today's balances to go on.
		const summary = withInventory([live], { period: "30d" })

		expect(summary.startInventory).toBeNull()
		expect(summary.totals).toMatchObject({ startInventoryUsd: null, returnPct: null })
		// The profit itself needs no inventory, and is still stated.
		expect(summary.totals.profitUsd).toBeCloseTo(50 - 78_500 / 1585, 9)

		// Balances read exactly seven days after a bucket began still describe it. A millisecond
		// later they do not.
		const midnight = Date.parse("2026-10-05T00:00:00Z")
		const known = (now: number) =>
			summarizeProfit([], {
				period: "30d",
				now,
				inventory: [{ at: now, balances: { USDC: 500 }, live: true }],
			}).series.filter((day) => day.startInventoryUsd !== null)
		expect(midnight - known(midnight)[0].start).toBe(BALANCES_REACH_MS)
		expect(known(midnight)).toHaveLength(8)
		expect(known(midnight + 1)).toHaveLength(7)
	})

	it("uses a stored snapshot however far it is from the period's start", () => {
		// Taken an hour ago, for a period that began a month ago: the fills since are undone, just
		// as they would be from today's balances, with no limit on how far back that goes.
		const recent: InventoryRecord = { at: NOW - HOUR, balances: { USDC: 1000, cNGN: 1_000_000 } }
		const summary = withInventory([recent, live], { period: "30d" })

		expect(summary.startInventory).toMatchObject({ source: "snapshot", recordedAt: recent.at })
		expect(summary.startInventory?.usd).toBeCloseTo(950 + 1_078_500 / 1585, 9)
		expect(summary.totals.returnPct).not.toBeNull()

		// And one from long before the period is carried forward to it.
		const old: InventoryRecord = { at: Date.parse("2026-06-01T00:00:00Z"), balances: { USDC: 250 } }
		expect(withInventory([old], { period: "30d" }).startInventory).toMatchObject({ source: "snapshot", usd: 250 })
	})

	it("prefers today's balances to a snapshot only when they are nearer", () => {
		// The week began six and a half days ago. A snapshot from a month back is further off
		// than today's balances, which are still within their reach.
		const old: InventoryRecord = { at: NOW - 30 * DAY, balances: { USDC: 1, cNGN: 0 } }
		expect(withInventory([old, live]).startInventory?.source).toBe("balances")

		// For a period that began a month ago the balances are out of reach, so the snapshot is used.
		expect(withInventory([old, live], { period: "30d" }).startInventory?.source).toBe("snapshot")
	})

	it("compares each bucket with the inventory that bucket began with", () => {
		const { series } = withInventory([live], { period: "30d" })

		// Only the buckets that start within seven days of today's balances can be rebuilt. It is
		// noon, so midnight on the 28th is half a day too far back.
		const known = series.filter((day) => day.startInventoryUsd !== null)
		expect(known).toHaveLength(7)
		expect(known[0].start).toBe(Date.parse("2026-09-29T00:00:00Z"))

		// The 4th began with the buy of the 2nd already made, and ended with the sale.
		const fourth = series.find((day) => day.start === Date.parse("2026-10-04T00:00:00Z"))
		const began = 1050 + 920_500 / 1585
		expect(fourth?.startInventoryUsd).toBeCloseTo(began, 9)
		expect(fourth?.returnPct).toBeCloseTo((100 * (-50 + 79_500 / 1585)) / began, 9)
	})

	it("puts back what was sent out of the wallet since", () => {
		const sent = { at: Date.parse("2026-10-03T12:00:00Z"), symbol: "USDC", amount: 300 }
		const before = { at: WEEK_START - DAY, symbol: "USDC", amount: 9_999 }
		const summary = withInventory([live], { outflows: [sent, before] })

		// The 300 were still held when the week began. The earlier send was already gone.
		expect(summary.startInventory?.tokens[0]).toMatchObject({ symbol: "USDC", amount: 1250 })
	})

	it("never counts a token as held in the negative", () => {
		// 79,500 cNGN came in from the sale, but the wallet holds only 1,000 now: the rest was
		// moved out in a way no record shows.
		const summary = withInventory([{ at: NOW, balances: { USDC: 1000, cNGN: 1000 }, live: true }])

		expect(summary.startInventory?.tokens.find((token) => token.symbol === "cNGN")).toMatchObject({
			amount: 79_500,
		})
		const drained = summarizeProfit([sell(50, 1590, "2026-10-04 09:00:00")], {
			period: "7d",
			now: NOW,
			inventory: [{ at: NOW, balances: { USDC: 1000, cNGN: 1000 }, live: true }],
		})
		expect(drained.startInventory?.tokens.map((token) => token.symbol)).toEqual(["USDC"])
	})

	it("has no return without an inventory to compare with", () => {
		const summary = withInventory([{ at: NOW, balances: { USDC: 50, cNGN: 0 }, live: true }], {
			outflows: [],
		})

		// Rebuilt, the week began with nothing: 50 USDC fewer than now, and cNGN that was spent.
		expect(summary.totals.returnPct).not.toBeNull()
		const nothing = summarizeProfit([buy(50, 1580, "2026-10-04 09:00:00")], {
			period: "7d",
			now: NOW,
			inventory: [{ at: NOW, balances: { USDC: 50 }, live: true }],
		})
		expect(nothing.startInventory?.tokens).toEqual([{ symbol: "cNGN", amount: 79_000, usd: 50 }])
		const empty = summarizeProfit([], {
			period: "7d",
			now: NOW,
			inventory: [{ at: NOW, balances: {}, live: true }],
		})
		expect(empty.totals).toMatchObject({ startInventoryUsd: 0, returnPct: null })
	})
})

describe("what the indexer says moved", () => {
	/** A buy and a sell inside the week: 50 USDC more held, and 78,500 cNGN less. */
	const fills = () => [buy(100, 1580, "2026-10-02 09:00:00"), sell(50, 1590, "2026-10-04 09:00:00")]
	const live: InventoryRecord = { at: NOW, balances: { USDC: 1000, cNGN: 1_000_000 }, live: true }
	const reading = (at: number, balances: Record<string, number>, chains = ["EVM-8453"]): InventoryReading => ({
		at,
		balances,
		chains: Object.fromEntries(Object.keys(balances).map((symbol) => [symbol, chains])),
	})
	const summarize = (readings: InventoryReading[], inventory: InventoryRecord[] = [live]) =>
		summarizeProfit(fills(), { period: "7d", now: NOW, inventory, readings })

	it("accounts for a deposit that no fill or snapshot shows", () => {
		// The chain says 550 USDC more are held than when the week began. The fills explain 50
		// of that, so 500 came in some other way, and were not there at the start.
		const summary = summarize([
			reading(WEEK_START, { USDC: 450, cNGN: 1_078_500 }),
			reading(NOW, { USDC: 1000, cNGN: 1_000_000 }),
		])

		expect(summary.startInventory?.tokens.map(({ symbol, amount }) => [symbol, amount])).toEqual([
			["cNGN", 1_078_500],
			["USDC", 450],
		])
		expect(summary.totals.transfersUsd).toBeCloseTo(500, 9)
		// Undoing the fills alone would have counted the deposit as held all week.
		expect(summarize([]).startInventory?.tokens.find((token) => token.symbol === "USDC")?.amount).toBe(950)
		expect(summarize([]).totals.transfersUsd).toBeNull()
	})

	it("accounts for a withdrawal the same way, as a negative", () => {
		const summary = summarize([
			reading(WEEK_START, { USDC: 1250, cNGN: 1_078_500 }),
			reading(NOW, { USDC: 1000, cNGN: 1_000_000 }),
		])

		expect(summary.startInventory?.tokens.find((token) => token.symbol === "USDC")?.amount).toBe(1250)
		expect(summary.totals.transfersUsd).toBeCloseTo(-300, 9)
	})

	it("reads no transfer where the fills explain everything that moved", () => {
		const summary = summarize([
			reading(WEEK_START, { USDC: 950, cNGN: 1_078_500 }),
			reading(NOW, { USDC: 1000, cNGN: 1_000_000 }),
		])

		expect(summary.totals.transfersUsd).toBeCloseTo(0, 9)
		expect(summary.startInventory?.usd).toBeCloseTo(950 + 1_078_500 / 1585, 9)
	})

	it("carries what the record holds, not what the indexer counts", () => {
		// The indexer follows only one of the chains this solver holds USDC on, so its figures
		// are smaller than the record's. Only the difference between its readings is used.
		const summary = summarize([reading(WEEK_START, { USDC: 100 }), reading(NOW, { USDC: 400 })])

		// 300 more on chain, 50 of them from fills: the record's 1,000 were 700 at the start.
		expect(summary.startInventory?.tokens.find((token) => token.symbol === "USDC")?.amount).toBe(700)
		// cNGN has no reading, so it is carried by the fills as before.
		expect(summary.startInventory?.tokens.find((token) => token.symbol === "cNGN")?.amount).toBe(1_078_500)
		expect(summary.totals.transfersUsd).toBeCloseTo(250, 9)
	})

	it("falls back to the fills for a token two readings cover on different chains", () => {
		// The indexer began following a second chain during the week. Its later USDC figure
		// includes that chain's whole balance, which did not arrive from anywhere.
		const summary = summarize([
			reading(WEEK_START, { USDC: 450, cNGN: 1_078_500 }),
			{
				at: NOW,
				balances: { USDC: 9000, cNGN: 1_000_000 },
				chains: { USDC: ["EVM-8453", "EVM-56"], cNGN: ["EVM-8453"] },
			},
		])

		expect(summary.startInventory?.tokens.find((token) => token.symbol === "USDC")?.amount).toBe(950)
		expect(summary.startInventory?.tokens.find((token) => token.symbol === "cNGN")?.amount).toBe(1_078_500)
		// Only cNGN can be compared, and the fills explain all of it.
		expect(summary.totals.transfersUsd).toBeCloseTo(0, 9)
	})

	it("places a transfer in the bucket it happened in", () => {
		// 500 USDC arrive on the 3rd. Everything else that moves is a fill.
		const balanceAt = (at: number) => ({
			USDC:
				450 +
				(at > Date.parse("2026-10-02T09:00:00Z") ? 100 : 0) +
				(at > Date.parse("2026-10-03T12:00:00Z") ? 500 : 0) -
				(at > Date.parse("2026-10-04T09:00:00Z") ? 50 : 0),
			cNGN:
				1_078_500 -
				(at > Date.parse("2026-10-02T09:00:00Z") ? 158_000 : 0) +
				(at > Date.parse("2026-10-04T09:00:00Z") ? 79_500 : 0),
		})
		const instants = inventoryInstants(fills(), { period: "7d", now: NOW, inventory: [live] })
		const summary = summarize(instants.map((at) => reading(at, balanceAt(at))))

		const transfers = summary.series.map((day) => Math.round(day.transfersUsd ?? Number.NaN))
		expect(transfers).toEqual([0, 0, 0, 0, 500, 0, 0])
		// And each day began with what was really held then: the deposit only from the 4th on.
		const third = summary.series[4]
		const fourth = summary.series[5]
		expect(third.startInventoryUsd).toBeCloseTo(550 + 920_500 / 1585, 9)
		expect(fourth.startInventoryUsd).toBeCloseTo(1050 + 920_500 / 1585, 9)
	})

	it("uses the readings to carry a snapshot as well as today's balances", () => {
		// A snapshot from two days before the week, and a deposit of 200 USDC in between.
		const taken = WEEK_START - 2 * DAY
		const snapshot: InventoryRecord = { at: taken, balances: { USDC: 300, cNGN: 1_078_500 } }
		const summary = summarize(
			[reading(taken, { USDC: 250, cNGN: 1_078_500 }), reading(WEEK_START, { USDC: 450, cNGN: 1_078_500 })],
			[snapshot],
		)

		expect(summary.startInventory).toMatchObject({ source: "snapshot", recordedAt: taken })
		expect(summary.startInventory?.tokens.find((token) => token.symbol === "USDC")?.amount).toBe(500)
	})

	it("names the instants it needs readings for", () => {
		const taken = WEEK_START - 2 * DAY
		const snapshot: InventoryRecord = { at: taken, balances: { USDC: 300 } }
		const instants = inventoryInstants(fills(), { period: "7d", now: NOW, inventory: [snapshot, live] })

		// Where each of the seven days starts, now, and the snapshot the earliest days are
		// described from. The later days are described from today's balances, which is now.
		expect(instants).toEqual([taken, ...Array.from({ length: 7 }, (_, day) => WEEK_START + day * DAY), NOW])
		// With no record there is nothing to carry, and only the buckets' own edges are needed.
		expect(inventoryInstants(fills(), { period: "7d", now: NOW })).toHaveLength(8)
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

		expect(summary.totals.profitUsd).toBeCloseTo(10, 9)
		expect(summary.totals.boughtUsd).toBeCloseTo(1_000_000 * 0.000635, 9)
	})

	it("prices a token with no dollar stable of its own through one that has", () => {
		const euro = { book: "EURC-cNGN", base: "EURC", quote: "cNGN" }
		const summary = week([
			buy(100, 1850, "2026-10-04 09:00:00", euro),
			sell(100, 1860, "2026-10-05 09:00:00", euro),
			buy(100, 1580, "2026-10-04 09:00:00"),
			sell(100, 1590, "2026-10-05 09:00:00"),
		])

		// 1,000 cNGN from each pair, and cNGN has a dollar price from USDC.
		expect(summary.unpricedTokens).toEqual([])
		expect(summary.books.find((book) => book.book === "EURC-cNGN")?.profitUsd).toBeCloseTo(1000 / 1585, 9)
		expect(summary.totals.profitUsd).toBeCloseTo(2000 / 1585, 9)
	})

	it("leaves tokens no pair connects to a dollar out of the dollar figures, and names them", () => {
		const rand = { book: "EURC-ZARP", base: "EURC", quote: "ZARP" }
		const summary = week([
			buy(100, 20, "2026-10-04 09:00:00", rand),
			sell(100, 21, "2026-10-05 09:00:00", rand),
			buy(100, 1580, "2026-10-04 09:00:00"),
			sell(100, 1590, "2026-10-05 09:00:00"),
		])

		const unpriced = summary.books.find((book) => book.book === "EURC-ZARP")
		expect(unpriced).toMatchObject({ quoteChange: 100, profitUsd: null })
		// Its spread is still known, in its own tokens.
		expect(unpriced?.spreadPct).toBeCloseTo(5, 9)
		expect(summary.unpricedTokens).toEqual(["EURC", "ZARP"])
		expect(summary.totals.profitUsd).toBeCloseTo(1000 / 1585, 9)
		// Its fills are still counted as fills.
		expect(summary.totals).toMatchObject({ buys: 2, sells: 2 })
	})

	it("counts what a same-asset book takes in over what it pays out", () => {
		const same = { book: "USDC-USDC", base: "USDC", quote: "USDC" }
		// Paying 999 USDC out for every 1,000 taken in.
		const fill = buy(1000, 0.999, "2026-10-05 09:00:00", same)
		const summary = week([fill, { ...fill, id: nextId++ }])

		expect(summary.books[0].profitUsd).toBeCloseTo(2, 9)
		expect(summary.totals.profitUsd).toBeCloseTo(2, 9)
	})
})
