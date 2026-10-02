import { describe, expect, it } from "vitest"
import type { OrderbookLevel, OrderbookSnapshot } from "../../types"
import { depthLevels, formatFixed, formatSize, priceDecimals, topOfBook } from "./orderbookModel"

function level(price: string, baseSize = "1000000000000000000", fillChain = "EVM-8453"): OrderbookLevel {
	return {
		price,
		priceBucket: price,
		worstPrice: price,
		baseSize,
		quoteSize: baseSize,
		fillChain,
		orderCount: 2,
		solverCount: 1,
	}
}
function book(bids: OrderbookLevel[], asks: OrderbookLevel[]): OrderbookSnapshot {
	return {
		id: "USDC/cNGN",
		base: "USDC",
		quote: "cNGN",
		bids,
		asks,
		bidLiquidity: "1",
		askLiquidity: "1",
		granularity: null,
	}
}

describe("order book depth", () => {
	it("orders prices differing below Number precision, with the best price first on each side", () => {
		const levels = [level("1500000000000000000000"), level("1500000000000000000001")]
		expect(depthLevels(levels, "BID").map((row) => row.price)).toEqual([levels[1].price, levels[0].price])
		expect(depthLevels(levels, "ASK").map((row) => row.price)).toEqual([levels[0].price, levels[1].price])
		expect(levels[0].price).toBe("1500000000000000000000")
	})
	it("accumulates all quoted base depth exactly, including chain-specific levels at the same price", () => {
		const levels = depthLevels(
			[level("2", "1000000000000000001"), level("2", "2000000000000000002", "EVM-1")],
			"BID",
		)
		expect(levels.map((row) => row.cumulativeBase)).toEqual(["1000000000000000001", "3000000000000000003"])
		expect(levels[0].depthRatio).toBe(33.33)
		expect(levels[1].depthRatio).toBe(100)
		expect(new Set(levels.map((row) => row.id)).size).toBe(2)
	})
	it("handles zero depth without invalid bar widths", () => {
		expect(depthLevels([level("1", "0")], "ASK")[0].depthRatio).toBe(0)
	})
	it("derives mid and spread from the returned levels", () => {
		expect(topOfBook(book([level("1480"), level("1490")], [level("1520"), level("1510")]))).toMatchObject({
			mid: "1500",
			spread: "20",
			crossed: false,
		})
	})
	it("keeps a crossed mid, with no negative spread", () => {
		expect(topOfBook(book([level("1510")], [level("1490")]))).toMatchObject({
			mid: "1500",
			spread: null,
			crossed: true,
		})
	})
	it("leaves mid and spread absent when either side is empty", () => {
		for (const snapshot of [book([], []), book([level("1")], []), book([], [level("2")])]) {
			expect(topOfBook(snapshot)).toMatchObject({ mid: null, spread: null, crossed: false })
		}
	})
	it("derives spread in basis points of mid", () => {
		const e18 = 10n ** 18n
		const top = topOfBook(book([level((1360n * e18).toString())], [level((1363n * e18).toString())]))
		expect(top.spreadBps).toBeCloseTo(22.03, 2)
		expect(top.bidOrderCount + top.askOrderCount).toBe(4)
	})
})

describe("order book formatting", () => {
	it("shows enough price digits to tell buckets apart, never fewer than two", () => {
		expect(priceDecimals(null)).toBe(2)
		expect(priceDecimals("1000000000000000000")).toBe(2)
		expect(priceDecimals("10000000000000000")).toBe(2)
		expect(priceDecimals("1000000000000000")).toBe(3)
		expect(priceDecimals("1")).toBe(6)
	})
	it("rounds fixed-point prices to exact, grouped decimals", () => {
		expect(formatFixed("1361500000000000000000", 2)).toBe("1,361.50")
		expect(formatFixed("1999999000000000000", 2)).toBe("2.00")
		expect(formatFixed(null, 2)).toBe("—")
	})
	it("keeps two size decimals below a million and goes compact above", () => {
		expect(formatSize("199750050000000000000000")).toBe("199,750.05")
		expect(formatSize("297990000000000000000000000")).toBe("297.9M")
	})
})
