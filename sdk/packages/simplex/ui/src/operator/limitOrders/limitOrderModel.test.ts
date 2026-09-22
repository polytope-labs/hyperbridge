import { describe, expect, it } from "vitest"
import type { LimitOrder } from "../../types"
import { available, describeRate, fromScaled, legs, statusOf } from "./limitOrderModel"

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

	it("counts what live bids are holding out of what is left", () => {
		const held = order({ remaining: (1_000_000n * ONE).toString(), reserved: (400_000n * ONE).toString() })
		expect(available(held)).toBe(600_000n * ONE)
		// Never negative: a reservation can briefly outrun a draw-down.
		expect(available(order({ remaining: "0", reserved: ONE.toString() }))).toBe(0n)
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
		expect(statusOf(order({ status: "filled" }))).toMatchObject({ label: "Filled", tone: "ok" })
	})
})
