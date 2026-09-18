import { describe, expect, it } from "vitest"
import { Decimal } from "decimal.js"
import type { LimitOrder } from "@/data/types"
import { ORDERBOOK_SCALE } from "@/orderbook/amounts"
import { limitOrderUsdEdges, usdFactorsFrom, usdValueOf } from "@/orderbook/usd"

const ONE = ORDERBOOK_SCALE

function limitOrder(base: string, quote: string, price: string): LimitOrder {
	return {
		id: `${base}/${quote}`,
		book: `${base}/${quote}`,
		base,
		quote,
		side: "BID",
		fillChain: "EVM-8453",
		price,
		size: "0",
		remaining: "0",
		reserved: "0",
		acceptedSources: ["EVM-1"],
		ttlSecs: 900,
		expiresAt: null,
		status: "open",
		commitment: null,
		orderNonce: "0",
		bookExpiresAt: null,
		bookPrice: null,
		lastError: null,
		createdAt: "2026-09-15 10:00:00",
		updatedAt: "2026-09-15 10:00:00",
	}
}

const at = (whole: number) => (BigInt(whole) * ONE).toString()

describe("usdFactorsFrom", () => {
	it("pins dollar stables at a dollar", () => {
		const factors = usdFactorsFrom([])
		expect(factors.get("USDC")?.toString()).toBe("1")
		expect(factors.get("USDT")?.toString()).toBe("1")
		expect(factors.get("DAI")?.toString()).toBe("1")
	})

	it("prices a symbol against a stable from the order's own rate", () => {
		// 1 USDC buys 1,500 cNGN, so a cNGN is worth a fifteen-hundredth of a dollar.
		const factors = usdFactorsFrom(limitOrderUsdEdges([limitOrder("USDC", "CNGN", at(1500))]))
		expect(factors.get("CNGN")?.toFixed(8)).toBe(new Decimal(1).div(1500).toFixed(8))
	})

	it("prices a symbol whose book puts the stable on the base side", () => {
		// 1 EURC buys 1.08 USDC, so a EURC is worth $1.08.
		const factors = usdFactorsFrom(limitOrderUsdEdges([limitOrder("EURC", "USDC", "1080000000000000000")]))
		expect(factors.get("EURC")?.toFixed(4)).toBe("1.0800")
	})

	it("reaches a symbol two hops from a dollar", () => {
		const factors = usdFactorsFrom(
			limitOrderUsdEdges([limitOrder("USDC", "CNGN", at(1500)), limitOrder("CNGN", "XAF", at(2))]),
		)
		// 1 cNGN = 2 XAF, and a cNGN is 1/1500 of a dollar.
		expect(factors.get("XAF")?.toFixed(8)).toBe(new Decimal(1).div(3000).toFixed(8))
	})

	it("leaves a symbol no order connects to a dollar unpriced", () => {
		const factors = usdFactorsFrom(limitOrderUsdEdges([limitOrder("XAF", "KES", at(5))]))
		expect(factors.get("XAF")).toBeUndefined()
		expect(factors.get("KES")).toBeUndefined()
	})

	it("never re-prices a stable, so a mis-set stable market cannot move the anchors", () => {
		const factors = usdFactorsFrom(limitOrderUsdEdges([limitOrder("USDC", "USDT", at(3))]))
		expect(factors.get("USDT")?.toString()).toBe("1")
		expect(factors.get("USDC")?.toString()).toBe("1")
	})

	it("gives the same answer whichever order the routes were created in", () => {
		const cheap = limitOrder("USDC", "CNGN", at(1400))
		const rich = limitOrder("USDT", "CNGN", at(1600))
		const one = usdFactorsFrom(limitOrderUsdEdges([cheap, rich]))
		const other = usdFactorsFrom(limitOrderUsdEdges([rich, cheap]))
		expect(one.get("CNGN")?.toString()).toBe(other.get("CNGN")?.toString())
	})
})

describe("limitOrderUsdEdges", () => {
	it("skips a same-asset order, which carries no exchange rate", () => {
		expect(limitOrderUsdEdges([limitOrder("USDC", "USDC", at(1))])).toEqual([])
	})

	it("skips an order priced at zero rather than dividing by it", () => {
		expect(limitOrderUsdEdges([limitOrder("USDC", "CNGN", "0")])).toEqual([])
	})
})

describe("usdValueOf", () => {
	it("values an amount through the factor, case-insensitively on the symbol", () => {
		const factors = usdFactorsFrom(limitOrderUsdEdges([limitOrder("USDC", "CNGN", at(1500))]))
		expect(usdValueOf(factors, "cNGN", new Decimal(1_500_000))?.toFixed(2)).toBe("1000.00")
		expect(usdValueOf(factors, "USDC", new Decimal(1000))?.toFixed(2)).toBe("1000.00")
	})

	it("answers null for a symbol with no route to a dollar, rather than guessing", () => {
		expect(usdValueOf(usdFactorsFrom([]), "XAF", new Decimal(1000))).toBeNull()
	})
})
