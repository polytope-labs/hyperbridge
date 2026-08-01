import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"
import { bookCrossedAt, FillerPricePolicy } from "@/config/interpolated-curve"

describe("bookCrossedAt", () => {
	it("returns null for an uncrossed book", () => {
		const bid = [
			{ amount: "100", price: "1580" },
			{ amount: "5000", price: "1570" },
		]
		const ask = [
			{ amount: "100", price: "1560" },
			{ amount: "5000", price: "1550" },
		]
		expect(bookCrossedAt(bid, ask)).toBeNull()
	})

	it("reports the first crossing amount with both prices", () => {
		const bid = [
			{ amount: "100", price: "1580" },
			{ amount: "5000", price: "1540" },
		]
		const ask = [{ amount: "0", price: "1560" }]
		const crossed = bookCrossedAt(bid, ask)
		expect(crossed).not.toBeNull()
		// bid falls below the flat 1560 ask before the 5000 breakpoint.
		expect(new Decimal(crossed!.bid).lte(crossed!.ask)).toBe(true)
	})

	it("treats an exact bid == ask touch as crossed (a round trip breaks even at best)", () => {
		const bid = [{ amount: "0", price: "1560" }]
		const ask = [{ amount: "0", price: "1560" }]
		expect(bookCrossedAt(bid, ask)).toEqual({ amount: "0", bid: "1560", ask: "1560" })
	})

	it("merges equivalent decimal spellings into one grid point", () => {
		const bid = [
			{ amount: "1000.0", price: "1580" },
			{ amount: "5000", price: "1570" },
		]
		const ask = [
			{ amount: "1000", price: "1560" },
			{ amount: "5000", price: "1550" },
		]
		expect(bookCrossedAt(bid, ask)).toBeNull()
	})

	it("evaluates duplicate-amount points without dividing by zero", () => {
		const bid = [
			{ amount: "1000", price: "1580" },
			{ amount: "1000", price: "1575" },
			{ amount: "5000", price: "1570" },
		]
		const ask = [{ amount: "0", price: "1500" }]
		expect(bookCrossedAt(bid, ask)).toBeNull()

		const policy = new FillerPricePolicy({ points: bid })
		expect(policy.getPrice(new Decimal("1000")).isFinite()).toBe(true)
		expect(policy.getPrice(new Decimal("3000")).isFinite()).toBe(true)
	})

	it("ignores partially-typed points instead of throwing", () => {
		const bid = [
			{ amount: "", price: "" },
			{ amount: "100", price: "abc" },
		]
		const ask = [{ amount: "0", price: "1560" }]
		expect(bookCrossedAt(bid, ask)).toBeNull()
	})
})
