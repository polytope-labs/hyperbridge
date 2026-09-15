import { describe, expect, it } from "vitest"
import { ORDERBOOK_SCALE, signedAmounts, toRaw } from "@/orderbook/amounts"

/** 1,500 quote per 1 base, the shape a USDC/cNGN book reads at. */
const PRICE = 1500n * ORDERBOOK_SCALE
const ONE = ORDERBOOK_SCALE

describe("toRaw", () => {
	it("scales an 18-decimal amount down to the token's own units", () => {
		expect(toRaw(1000n * ONE, 6)).toBe(1_000_000_000n)
		expect(toRaw(1000n * ONE, 18)).toBe(1000n * ONE)
	})

	it("truncates rather than rounding, so a raw amount is never more than was offered", () => {
		expect(toRaw(ONE - 1n, 6)).toBe(999_999n)
	})
})

describe("signedAmounts", () => {
	it("prices a bid at the operator's rate: pay the quote, receive the base", () => {
		// Pay 1,500,000 cNGN (18dp) at 1500, so the base leg is 1,000 USDC (6dp).
		const { inputAmount, outputAmount } = signedAmounts({
			side: "BID",
			size: 1_500_000n * ONE,
			price: PRICE,
			baseDecimals: 6,
			quoteDecimals: 18,
		})
		expect(outputAmount).toBe(1_500_000n * ONE)
		expect(inputAmount).toBe(1_000_000_000n)
	})

	it("prices an ask the other way round: pay the base, receive the quote", () => {
		const { inputAmount, outputAmount } = signedAmounts({
			side: "ASK",
			size: 1000n * ONE,
			price: PRICE,
			baseDecimals: 6,
			quoteDecimals: 18,
		})
		expect(outputAmount).toBe(1_000_000_000n)
		expect(inputAmount).toBe(1_500_000n * ONE)
	})

	it("rounds the input up, so the signed rate is never better for the taker than the price", () => {
		// 1 unit of an 18dp quote at 1500 needs a base input of 1/1500 of a unit,
		// which is not a whole 6dp unit — the ceiling costs the taker one more.
		const bid = signedAmounts({ side: "BID", size: 1n, price: PRICE, baseDecimals: 6, quoteDecimals: 18 })
		expect(bid.outputAmount).toBe(1n)
		expect(bid.inputAmount).toBe(1n)

		const ask = signedAmounts({ side: "ASK", size: 1n, price: PRICE, baseDecimals: 18, quoteDecimals: 6 })
		expect(ask.outputAmount).toBe(1n)
		expect(ask.inputAmount).toBe(1n)
	})

	it("holds the rate across a pair whose tokens disagree about decimals", () => {
		// 6dp base against 6dp quote: both legs land on whole raw units.
		const { inputAmount, outputAmount } = signedAmounts({
			side: "BID",
			size: 3000n * ONE,
			price: 2n * ONE,
			baseDecimals: 6,
			quoteDecimals: 6,
		})
		expect(outputAmount).toBe(3_000_000_000n)
		expect(inputAmount).toBe(1_500_000_000n)
	})

	it("refuses a price of zero rather than dividing by it", () => {
		expect(() => signedAmounts({ side: "BID", size: ONE, price: 0n, baseDecimals: 6, quoteDecimals: 18 })).toThrow(
			/greater than zero/,
		)
	})
})
