import { afterEach, describe, expect, it, vi } from "vitest"
import { OrderbookClient } from "@/orderbook/client"

describe("public order book snapshot", () => {
	afterEach(() => vi.restoreAllMocks())
	it("passes the book and route filters and preserves fixed-point values", async () => {
		const book = {
			id: "USDC/cNGN",
			base: "USDC",
			quote: "cNGN",
			bids: [],
			asks: [],
			bidLiquidity: "9007199254740993000000000000000",
			askLiquidity: "1000000000000000001",
		}
		const serverInfo = {
			priceGranularities: [
				{ book: "EURC/cNGN", granularity: "1000000000000000000" },
				{ book: book.id, granularity: "10000000000000000" },
			],
		}
		const mock = vi
			.spyOn(globalThis, "fetch")
			.mockResolvedValue(new Response(JSON.stringify({ data: { book, serverInfo } })))
		expect(
			await new OrderbookClient("https://book/mainnet", 5000).snapshot(book.id, {
				sourceChain: "EVM-1",
				fillChain: "EVM-8453",
			}),
		).toEqual({ ...book, granularity: "10000000000000000" })
		const body = JSON.parse(mock.mock.calls[0][1]!.body as string)
		expect(body.variables).toEqual({ id: book.id, sourceChain: "EVM-1", fillChain: "EVM-8453" })
		expect(body.query).toContain("availableLiquidity(side: BID, sourceChain: $sourceChain, fillChain: $fillChain)")
	})
	it("omits chain filters for the whole book and preserves a missing book", async () => {
		const mock = vi
			.spyOn(globalThis, "fetch")
			.mockResolvedValue(new Response(JSON.stringify({ data: { book: null, serverInfo: { priceGranularities: [] } } })))
		expect(await new OrderbookClient("https://book/graphql", 5000).snapshot("unknown")).toBeNull()
		expect(JSON.parse(mock.mock.calls[0][1]!.body as string).variables).toEqual({
			id: "unknown",
			sourceChain: null,
			fillChain: null,
		})
	})
	it("surfaces GraphQL failures instead of silently showing empty liquidity", async () => {
		vi.spyOn(globalThis, "fetch").mockResolvedValue(
			new Response(JSON.stringify({ errors: [{ message: "Book unavailable" }] })),
		)
		await expect(new OrderbookClient("https://book/graphql", 5000).snapshot("USDC/cNGN")).rejects.toThrow(
			"Book unavailable",
		)
	})
})
