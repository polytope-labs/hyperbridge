import { describe, expect, it } from "vitest"
import type { LimitOrder } from "../../types"
import {
	groupThousands,
	describeProgress,
	describeRate,
	fromScaled,
	legs,
	type LimitOrderDraft,
	progressOf,
	rateParts,
	requestFrom,
	rowBadge,
	statusOf,
	tabOf,
} from "./limitOrderModel"

const ONE = 10n ** 18n

function order(overrides: Partial<LimitOrder> = {}): LimitOrder {
	return {
		id: "limit-0",
		book: "USDC/CNGN",
		base: "USDC",
		quote: "CNGN",
		side: "BID",
		fillChain: "EVM-8453",
		price: (1500n * ONE).toString(),
		size: (1_500_000n * ONE).toString(),
		remaining: (1_500_000n * ONE).toString(),
		reserved: "0",
		acceptedSources: ["EVM-1"],
		ttlSecs: 900,
		expiresAt: null,
		status: "open",
		commitment: "0xabc",
		orderNonce: "0",
		bookExpiresAt: null,
		bookPrice: null,
		lastError: null,
		createdAt: "2026-09-19 10:00:00",
		updatedAt: "2026-09-19 10:00:00",
		...overrides,
	} as LimitOrder
}

describe("reading a limit order back", () => {
	it("shows whole tokens, not the orderbook's 1e18", () => {
		// What the operator typed is what they should read back.
		expect(fromScaled((1_500_000n * ONE).toString())).toBe("1,500,000")
		expect(fromScaled((ONE / 2n).toString())).toBe("0.5")
		expect(fromScaled((1000n * ONE + ONE / 4n).toString())).toBe("1,000.25")
	})

	it("says which side is taken in and which is paid out", () => {
		expect(legs(order())).toEqual({ input: "USDC", output: "CNGN" })
		expect(legs(order({ side: "ASK" }))).toEqual({ input: "CNGN", output: "USDC" })
	})

	it("states the rate the two amounts imply", () => {
		expect(describeRate(order())).toBe("1,500 CNGN per USDC")
	})

	it("rounds the rate, where an amount is cut", () => {
		// Buying with 20,000 CNGN at 1374 takes in 20,000 ÷ 1374 USDC, rounded up, which
		// divides back to a price 44e-18 short of 1374.
		const typed = "1373999999999999999956"
		expect(describeRate(order({ price: typed }))).toBe("1,374 CNGN per USDC")
		expect(describeRate(order({ price: (1373n * ONE + ONE / 2n).toString() }))).toBe("1,373.5 CNGN per USDC")
		// An amount is never shown as more than is there.
		expect(fromScaled(typed)).toBe("1,373.999999")
	})
})

describe("what the operator sees at a glance", () => {
	it("separates an order on the book from one still posting", () => {
		expect(statusOf(order())).toMatchObject({ label: "On the book", tone: "ok" })
		expect(statusOf(order({ commitment: null }))).toMatchObject({ label: "Posting", tone: "warn" })
	})

	it("keeps a refusal visible on an order that is otherwise open", () => {
		// The row stays open and the next cycle tries again, so the status alone
		// would not tell the operator the orderbook turned it down.
		const refused = statusOf(order({ lastError: "TTL_TOO_SHORT: minimum is 900" }))
		expect(refused).toMatchObject({ label: "On the book", tone: "warn" })
		expect(refused.detail).toContain("TTL_TOO_SHORT")
	})

	it("names the terminal states plainly", () => {
		expect(statusOf(order({ status: "expired" })).label).toBe("Expired")
		expect(statusOf(order({ status: "cancelled" })).label).toBe("Cancelled")
		expect(statusOf(order({ status: "filled", remaining: "0" }))).toEqual({
			label: "Filled",
			tone: "ok",
			detail: undefined,
		})
	})

	it("says a filled order with something left was closed under the dust floor", () => {
		const closed = statusOf(order({ status: "filled", quote: "cNGN", remaining: "3624688001000000000000" }))
		expect(closed).toMatchObject({ label: "Filled", tone: "ok" })
		expect(closed.detail).toBe("closed with 3,624.688001 cNGN left, below the orderbook's dust floor")
	})
})

describe("the list an order is kept in", () => {
	it("keeps an order live while it is open or between postings", () => {
		expect(tabOf(order())).toBe("live")
		// A fill is being settled and the order goes straight back on the book.
		expect(tabOf(order({ status: "resizing" }))).toBe("live")
	})

	it("keeps filled orders apart from the ones that closed without filling", () => {
		expect(tabOf(order({ status: "filled" }))).toBe("filled")
		expect(tabOf(order({ status: "cancelled" }))).toBe("cancelled")
		expect(tabOf(order({ status: "expired" }))).toBe("cancelled")
		expect(tabOf(order({ status: "rejected" }))).toBe("cancelled")
	})
})

describe("a row as a bar of its cap", () => {
	const size = 1_500_000n * ONE
	const left = (remaining: bigint, reserved = 0n) =>
		order({ remaining: remaining.toString(), reserved: reserved.toString() })

	it("measures what has gone out and what bids hold against the cap", () => {
		expect(progressOf(order())).toEqual({ consumed: 0, held: 0 })
		expect(progressOf(left(570_000n * ONE, 60_000n * ONE))).toEqual({ consumed: 62, held: 4 })
		expect(progressOf(left(0n))).toEqual({ consumed: 100, held: 0 })
	})

	it("cuts rather than rounds, so a bar never shows more consumed than was", () => {
		// 2/3 consumed is 66.666…%.
		expect(progressOf(left(size / 3n)).consumed).toBe(66.66)
		// One unit short of the whole cap is not yet 100.
		expect(progressOf(left(1n)).consumed).toBe(99.99)
	})

	it("keeps the held part inside what is left", () => {
		// Bids hold against what is left without counting one another, so their sum can pass it.
		expect(progressOf(left(150_000n * ONE, 400_000n * ONE))).toEqual({ consumed: 90, held: 10 })
	})

	it("draws nothing for an order it cannot measure", () => {
		expect(progressOf(order({ size: "0", remaining: "0" }))).toEqual({ consumed: 0, held: 0 })
		expect(progressOf(order({ remaining: "not a number" }))).toEqual({ consumed: 0, held: 0 })
	})

	it("says how far an order has filled, in words at either end", () => {
		expect(describeProgress(order())).toBe("Nothing filled yet")
		expect(describeProgress(order({ status: "expired" }))).toBe("Nothing filled")
		expect(describeProgress(left(570_000n * ONE))).toBe("62% filled")
		expect(describeProgress(order({ status: "filled", remaining: "0" }))).toBe("Filled in full")
	})

	it("never reads a part fill as none or as the whole", () => {
		expect(describeProgress(left(size - size / 400n))).toBe("0.25% filled")
		expect(describeProgress(left(size - 1n))).toBe("<0.01% filled")
		// Closed under the dust floor with a little left.
		expect(describeProgress(order({ status: "filled", remaining: (size / 1000n).toString() }))).toBe("99.9% filled")
	})

	it("splits the rate into the figure and what it is a rate of", () => {
		expect(rateParts(order())).toEqual({ figure: "1,500", unit: "CNGN per USDC" })
	})

	it("badges only a status the operator has to read", () => {
		// Being in the live list already says an order is on the book.
		expect(rowBadge(order())).toBeNull()
		expect(rowBadge(order({ lastError: "TTL_TOO_SHORT: minimum is 900" }))).toBeNull()
		expect(rowBadge(order({ commitment: null }))).toEqual({ label: "Posting", tone: "warn" })
		expect(rowBadge(order({ status: "resizing" }))).toEqual({ label: "Resizing", tone: "warn" })
		expect(rowBadge(order({ status: "filled", remaining: "0" }))).toEqual({ label: "Filled", tone: "ok" })
		expect(rowBadge(order({ status: "rejected", lastError: "UNSUPPORTED_PAIR: no such book" }))).toEqual({
			label: "Refused",
			tone: "err",
		})
	})
})

describe("the order a draft stands for", () => {
	const book = { id: "USDC-cNGN", base: "USDC", quote: "cNGN" }
	const draft = (overrides: Partial<LimitOrderDraft> = {}): LimitOrderDraft => ({
		book,
		side: "BID",
		amount: "10",
		rate: "1590",
		fillChain: "EVM-97",
		acceptedSources: ["EVM-97", "EVM-80002"],
		...overrides,
	})

	it("buys the base: pays the amount out in the quote, takes amount ÷ rate of the base in", () => {
		expect(requestFrom(draft({ amount: "15900" }))).toEqual({
			fillChain: "EVM-97",
			tokenIn: "USDC",
			amountIn: "10",
			tokenOut: "cNGN",
			amountOut: "15900",
			acceptedSources: ["EVM-97", "EVM-80002"],
		})
	})

	it("sells the base: pays the amount out, takes amount × rate in", () => {
		expect(requestFrom(draft({ side: "ASK" }))).toMatchObject({
			tokenIn: "cNGN",
			amountIn: "15900",
			tokenOut: "USDC",
			amountOut: "10",
		})
	})

	it("keeps fractional amounts and rates exact, without floating point", () => {
		// 0.1 × 0.2 is 0.020000000000000004 in binary floating point.
		expect(requestFrom(draft({ side: "ASK", amount: "0.1", rate: "0.2" }))).toMatchObject({ amountIn: "0.02" })
		expect(requestFrom(draft({ side: "ASK", amount: "1234.5678", rate: "1590.25" }))).toMatchObject({
			amountIn: "1963271.44395",
		})
		// And back: the division is exact when the rate divides the amount.
		expect(requestFrom(draft({ amount: "1963271.44395", rate: "1590.25" }))).toMatchObject({ amountIn: "1234.5678" })
	})

	it("rounds so the posted rate is never better for the taker than the one stated", () => {
		// Both sides take in, and both round what they take in up. A bid paying out one unit of
		// quote at 3 per base takes in a whole unit of base, not a third of one...
		expect(requestFrom(draft({ amount: "0.000000000000000001", rate: "3" }))).toMatchObject({
			amountIn: "0.000000000000000001",
		})
		// ...and an ask paying out half a base at 3e-18 quote each takes in two units, not one and a half.
		const half = { amount: "0.5", rate: "0.000000000000000003", side: "ASK" as const }
		expect(requestFrom(draft(half))).toMatchObject({ amountIn: "0.000000000000000002" })
	})

	it("never asks for nothing in return, however small the order", () => {
		// Rounding what is taken in up keeps it at least the smallest unit the orderbook carries.
		expect(requestFrom(draft({ side: "ASK", amount: "0.4", rate: "0.000000000000000001" }))).toMatchObject({
			amountIn: "0.000000000000000001",
		})
	})

	it("is nothing until the draft is a whole order", () => {
		expect(requestFrom(draft({ amount: "" }))).toBeNull()
		expect(requestFrom(draft({ rate: "0" }))).toBeNull()
		expect(requestFrom(draft({ amount: "1.2.3" }))).toBeNull()
		expect(requestFrom(draft({ acceptedSources: [] }))).toBeNull()
		expect(requestFrom(draft({ fillChain: "" }))).toBeNull()
		// More decimals than the orderbook carries would be silently dropped.
		expect(requestFrom(draft({ amount: `0.${"0".repeat(18)}1` }))).toBeNull()
	})
})

describe("groupThousands", () => {
	it("groups the whole part and leaves the fraction as typed", () => {
		expect(groupThousands("1590")).toBe("1,590")
		expect(groupThousands("1590000.12345")).toBe("1,590,000.12345")
		expect(groupThousands("100")).toBe("100")
	})

	it("keeps a figure mid-keystroke intact", () => {
		expect(groupThousands("")).toBe("")
		expect(groupThousands("1000.")).toBe("1,000.")
		expect(groupThousands(".5")).toBe(".5")
	})
})
