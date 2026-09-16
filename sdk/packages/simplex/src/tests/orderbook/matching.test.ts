import { describe, expect, it } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import type { LimitOrder } from "@/data/types"
import { ORDERBOOK_SCALE } from "@/orderbook/amounts"
import { availableOn, matchLimitOrder, type IncomingOrder } from "@/orderbook/matching"

const ONE = ORDERBOOK_SCALE
const BASE_CHAIN = "EVM-8453"
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const CNGN = "0x2222222222222222222222222222222222222222" as HexString
const EURC = "0x3333333333333333333333333333333333333333" as HexString

const ADDRESSES: Record<string, HexString> = { USDC, CNGN, EURC }
const resolve = (symbol: string, chain: string) => (chain === BASE_CHAIN ? ADDRESSES[symbol] ?? null : null)

function limitOrder(overrides: Partial<LimitOrder> = {}): LimitOrder {
	return {
		id: "L1",
		book: "USDC/CNGN",
		base: "USDC",
		quote: "CNGN",
		side: "BID",
		fillChain: BASE_CHAIN,
		price: (1500n * ONE).toString(),
		size: (3_000_000n * ONE).toString(),
		remaining: (3_000_000n * ONE).toString(),
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
		createdAt: "2026-09-15 10:00:00",
		updatedAt: "2026-09-15 10:00:00",
		...overrides,
	}
}

/** 1,000 USDC in from Ethereum, wanting 1,400,000 cNGN on Base. */
function incoming(overrides: Partial<IncomingOrder> = {}): IncomingOrder {
	return {
		source: "EVM-1",
		destination: BASE_CHAIN,
		inputSymbol: "USDC",
		outputToken: CNGN,
		inputNet: 1000n * ONE,
		requestedOutput: 1_400_000n * ONE,
		outputDecimals: 18,
		...overrides,
	}
}

describe("matchLimitOrder", () => {
	it("prices a bid at its own rate: input times price", () => {
		const match = matchLimitOrder([limitOrder()], incoming(), resolve)
		expect(match?.offer).toBe(1_500_000n * ONE)
		expect(match?.payout).toBe(1_500_000n * ONE)
	})

	it("prices an ask the other way round: input divided by price", () => {
		// Selling 1,500,000 cNGN for USDC on a book whose base is USDC.
		const order = limitOrder({ side: "ASK" })
		const match = matchLimitOrder(
			[order],
			incoming({
				inputSymbol: "CNGN",
				outputToken: USDC,
				inputNet: 1_500_000n * ONE,
				requestedOutput: 900n * ONE,
				outputDecimals: 6,
			}),
			resolve,
		)
		expect(match?.offer).toBe(1000n * ONE)
	})

	it("matches an order whose price falls short of the ask, and says by how much", () => {
		// Whether a shortfall can be filled is the caller's rule: cross-chain
		// reverts on any under-fill, same-chain may take a partial. Refusing here
		// would make a same-chain partial impossible whenever the price, rather
		// than the size, is what falls short.
		const order = limitOrder({ price: (1300n * ONE).toString() })
		const match = matchLimitOrder([order], incoming(), resolve)
		expect(match?.payout).toBe(1_300_000n * ONE)
		expect(match?.payout).toBeLessThan(incoming().requestedOutput)
	})

	it("does not match an order with nothing left to pay", () => {
		const order = limitOrder({ remaining: (1_000n * ONE).toString(), reserved: (1_000n * ONE).toString() })
		expect(matchLimitOrder([order], incoming(), resolve)).toBeNull()
	})

	it("only matches on the fill chain the order named", () => {
		const order = limitOrder({ fillChain: "EVM-1" })
		expect(matchLimitOrder([order], incoming(), resolve)).toBeNull()
	})

	describe("direction", () => {
		it("does not let a USDC to cNGN limit order price a cNGN to USDC swap", () => {
			// The operator's order takes USDC in and pays cNGN out. The reverse swap
			// needs its own order; this one must not stand in for it at 1/price.
			const usdcToCngn = limitOrder({ side: "BID" })
			const reversed = incoming({
				inputSymbol: "CNGN",
				outputToken: USDC,
				inputNet: 1_500_000n * ONE,
				requestedOutput: 900n * ONE,
				outputDecimals: 6,
			})
			expect(matchLimitOrder([usdcToCngn], reversed, resolve)).toBeNull()
		})

		it("does not let a cNGN to USDC limit order price a USDC to cNGN swap", () => {
			expect(matchLimitOrder([limitOrder({ side: "ASK" })], incoming(), resolve)).toBeNull()
		})

		it("serves each direction from the order facing it, when both are open", () => {
			// Both directions of one book, held at once. Each swap draws on its own.
			const usdcToCngn = limitOrder({ id: "usdc-to-cngn", side: "BID" })
			const cngnToUsdc = limitOrder({ id: "cngn-to-usdc", side: "ASK" })
			const reversed = incoming({
				inputSymbol: "CNGN",
				outputToken: USDC,
				inputNet: 1_500_000n * ONE,
				requestedOutput: 900n * ONE,
				outputDecimals: 6,
			})

			expect(matchLimitOrder([usdcToCngn, cngnToUsdc], incoming(), resolve)?.order.id).toBe("usdc-to-cngn")
			expect(matchLimitOrder([usdcToCngn, cngnToUsdc], reversed, resolve)?.order.id).toBe("cngn-to-usdc")
		})
	})

	it("only matches when the output token is the one the order asked for", () => {
		expect(matchLimitOrder([limitOrder()], incoming({ outputToken: EURC }), resolve)).toBeNull()
	})

	it("skips an order that is not open, or whose operator expiry has passed", () => {
		const now = new Date("2026-09-15T12:00:00.000Z")
		expect(matchLimitOrder([limitOrder({ status: "cancelled" })], incoming(), resolve, now)).toBeNull()
		expect(matchLimitOrder([limitOrder({ status: "resizing" })], incoming(), resolve, now)).toBeNull()
		expect(
			matchLimitOrder([limitOrder({ expiresAt: "2026-09-15T11:00:00.000Z" })], incoming(), resolve, now),
		).toBeNull()
		expect(
			matchLimitOrder([limitOrder({ expiresAt: "2026-09-15T13:00:00.000Z" })], incoming(), resolve, now),
		).not.toBeNull()
	})

	describe("accepted sources", () => {
		it("refuses a cross-chain order from a source the limit order did not declare", () => {
			const order = limitOrder({ acceptedSources: ["EVM-42161"] })
			expect(matchLimitOrder([order], incoming({ source: "EVM-1" }), resolve)).toBeNull()
		})

		it("ignores the declaration for a same-chain swap, as the orderbook does", () => {
			const order = limitOrder({ acceptedSources: ["EVM-42161"] })
			const sameChain = incoming({ source: BASE_CHAIN, destination: BASE_CHAIN })
			expect(matchLimitOrder([order], sameChain, resolve)).not.toBeNull()
		})
	})

	describe("choosing between several matches", () => {
		it("takes the one that pays the most", () => {
			const cheap = limitOrder({ id: "cheap", price: (1450n * ONE).toString() })
			const generous = limitOrder({ id: "generous", price: (1600n * ONE).toString() })
			expect(matchLimitOrder([cheap, generous], incoming(), resolve)?.order.id).toBe("generous")
		})

		it("passes over a better rate that has nothing behind it", () => {
			// Ranking on the offer alone would take `thin`, pay out the little it has
			// left, and leave the caller to skip a cross-chain fill that `deep` could
			// have covered outright.
			const thin = limitOrder({ id: "thin", price: (1600n * ONE).toString(), remaining: ONE.toString() })
			const deep = limitOrder({ id: "deep", price: (1450n * ONE).toString() })
			expect(matchLimitOrder([thin, deep], incoming(), resolve)?.order.id).toBe("deep")
		})

		it("breaks a tie on the payout by taking the one with more left", () => {
			const small = limitOrder({ id: "small", remaining: (1_500_000n * ONE).toString() })
			const large = limitOrder({ id: "large", remaining: (3_000_000n * ONE).toString() })
			expect(matchLimitOrder([small, large], incoming(), resolve)?.order.id).toBe("large")
		})
	})

	describe("payout", () => {
		it("is capped by what the limit order has left, never by the wallet", () => {
			const order = limitOrder({ remaining: (1_450_000n * ONE).toString() })
			const match = matchLimitOrder([order], incoming(), resolve)
			expect(match?.offer).toBe(1_500_000n * ONE)
			expect(match?.payout).toBe(1_450_000n * ONE)
		})

		it("counts a reservation against what is left", () => {
			const order = limitOrder({
				remaining: (1_500_000n * ONE).toString(),
				reserved: (500_000n * ONE).toString(),
			})
			expect(matchLimitOrder([order], incoming(), resolve)?.payout).toBe(1_000_000n * ONE)
		})

		it("still matches on the offer, so a short order is a decision for the caller", () => {
			// The offer clears the ask; only `available` falls short. Cross-chain
			// skips on that, same-chain fills partially, and neither choice is the
			// matcher's to make.
			const order = limitOrder({ remaining: (1n * ONE).toString() })
			const match = matchLimitOrder([order], incoming(), resolve)
			expect(match).not.toBeNull()
			expect(match?.payout).toBe(1n * ONE)
		})
	})

	it("returns null when nothing matches, rather than falling back to a price", () => {
		expect(matchLimitOrder([], incoming(), resolve)).toBeNull()
	})
})

describe("availableOn", () => {
	it("never reports less than nothing, even if a reservation overran", () => {
		expect(availableOn(limitOrder({ remaining: "100", reserved: "250" }))).toBe(0n)
	})
})
