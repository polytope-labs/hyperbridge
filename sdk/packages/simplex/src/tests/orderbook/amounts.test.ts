import { describe, expect, it } from "vitest"
import { inputFor, offerFor, ORDERBOOK_SCALE, rateFrom, signedAmounts, toRaw, toScaled } from "@/orderbook/amounts"

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

describe("rateFrom", () => {
	const book = { base: "USDC", quote: "CNGN" }

	it("derives the rate and the side from the two amounts the operator gave", () => {
		// 10,000 USDC in for 139,000,000 cNGN out: base in, quote out, so a bid.
		const { side, price } = rateFrom({
			...book,
			tokenIn: "USDC",
			amountIn: 10_000n * ONE,
			amountOut: 139_000_000n * ONE,
		})
		expect(side).toBe("BID")
		expect(price).toBe(13_900n * ONE)
	})

	it("reads the other direction on the same book as an ask", () => {
		// 139,000,000 cNGN in for 10,000 USDC out: quote in, base out.
		const { side, price } = rateFrom({
			...book,
			tokenIn: "CNGN",
			amountIn: 139_000_000n * ONE,
			amountOut: 10_000n * ONE,
		})
		expect(side).toBe("ASK")
		expect(price).toBe(13_900n * ONE)
	})

	it("rounds a bid's rate down, so it never pays away more quote than was offered", () => {
		// 3 quote for 7 base does not divide; the operator offered 3, not more.
		const { price } = rateFrom({ ...book, tokenIn: "USDC", amountIn: 7n, amountOut: 3n })
		expect(price).toBe((3n * ONE) / 7n)
		expect(price * 7n <= 3n * ONE).toBe(true)
	})

	it("rounds an ask's rate up, so it never takes in less quote than was asked for", () => {
		const { price } = rateFrom({ ...book, tokenIn: "CNGN", amountIn: 3n, amountOut: 7n })
		expect(price * 7n >= 3n * ONE).toBe(true)
	})

	it("refuses a symbol that is neither side of the book", () => {
		expect(() => rateFrom({ ...book, tokenIn: "EURC", amountIn: ONE, amountOut: ONE })).toThrow(
			/neither side of the USDC\/CNGN book/,
		)
	})

	it("refuses a zero amount rather than dividing by it", () => {
		expect(() => rateFrom({ ...book, tokenIn: "USDC", amountIn: 0n, amountOut: ONE })).toThrow(/greater than zero/)
		expect(() => rateFrom({ ...book, tokenIn: "USDC", amountIn: ONE, amountOut: 0n })).toThrow(/greater than zero/)
	})
})

describe("tokens finer than the orderbook's own unit", () => {
	// Uncommon, but a negative exponent is a RangeError rather than a wrong
	// number, so it would take the pricing path down rather than misprice it.
	it("converts a 24-decimal amount both ways", () => {
		expect(toRaw(ORDERBOOK_SCALE, 24)).toBe(10n ** 24n)
		expect(toScaled(10n ** 24n, 24)).toBe(ORDERBOOK_SCALE)
	})

	it("quantises an offer to nothing coarser than the scale itself", () => {
		const offer = offerFor({ side: "BID", inputAmount: 3n * ORDERBOOK_SCALE, price: ORDERBOOK_SCALE / 3n, outputDecimals: 24 })
		expect(offer).toBe(999999999999999999n)
	})
})

describe("inputFor", () => {
	it("inverts offerFor on both sides of the book", () => {
		const price = 1600n * ORDERBOOK_SCALE
		// A bid pays the quote: 1,600 per unit of base in, so 160,000 out costs 100 in.
		expect(inputFor({ side: "BID", outputAmount: 160_000n * ORDERBOOK_SCALE, price, inputDecimals: 18 })).toBe(
			100n * ORDERBOOK_SCALE,
		)
		// An ask pays the base: 187.5 out costs 300,000 of the quote in.
		expect(inputFor({ side: "ASK", outputAmount: (1875n * ORDERBOOK_SCALE) / 10n, price, inputDecimals: 6 })).toBe(
			300_000n * ORDERBOOK_SCALE,
		)
	})

	it("rounds up to the input token's unit, so a fill never pays above the operator's price", () => {
		// 100 out at 3 per unit is 33.33…, which a 6-decimal token takes as 33.333334.
		const input = inputFor({ side: "BID", outputAmount: 100n * ORDERBOOK_SCALE, price: 3n * ORDERBOOK_SCALE, inputDecimals: 6 })
		expect(input).toBe(33_333_334n * 10n ** 12n)
		expect(offerFor({ side: "BID", inputAmount: input, price: 3n * ORDERBOOK_SCALE, outputDecimals: 18 })).toBeGreaterThanOrEqual(
			100n * ORDERBOOK_SCALE,
		)
	})
})

