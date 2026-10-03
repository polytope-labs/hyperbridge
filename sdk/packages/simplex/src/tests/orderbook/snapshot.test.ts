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
	it("walks a side's orders and keeps only the clicked level's bucket", async () => {
		const node = (bucket: string, commitment: string) => ({
			solver: { address: "0xce319986ca4d5d0893751a628d0db3dc8fc91d62" },
			commitment,
			fillChain: "EVM-8453",
			price: "1357999999990173608026",
			priceBucket: bucket,
			advertisedSize: "1",
			quotedAmount: "2",
			resized: true,
			expiresAt: "2027-10-01T20:31:55Z",
			acceptedSources: ["EVM-1", "EVM-8453"],
		})
		const page = (nodes: unknown[], endCursor: string | null) => ({
			data: {
				book: {
					orders: { edges: nodes.map((node) => ({ node })), pageInfo: { hasNextPage: !!endCursor, endCursor } },
				},
			},
		})
		const mock = vi
			.spyOn(globalThis, "fetch")
			.mockResolvedValueOnce(new Response(JSON.stringify(page([node("1357", "0xa"), node("1356", "0xb")], "c1"))))
			.mockResolvedValueOnce(new Response(JSON.stringify(page([node("1357", "0xc")], null))))
		const orders = await new OrderbookClient("https://book/graphql", 5000).levelOrders("USDC-cNGN", {
			side: "BID",
			fillChain: "EVM-8453",
			priceBucket: "1357",
			sourceChain: "EVM-1",
		})
		expect(orders?.map((order) => order.commitment)).toEqual(["0xa", "0xc"])
		expect(orders?.[0]).toEqual({
			solver: "0xce319986ca4d5d0893751a628d0db3dc8fc91d62",
			commitment: "0xa",
			fillChain: "EVM-8453",
			price: "1357999999990173608026",
			advertisedSize: "1",
			quotedAmount: "2",
			resized: true,
			expiresAt: "2027-10-01T20:31:55Z",
			acceptedSources: ["EVM-1", "EVM-8453"],
		})
		const second = JSON.parse(mock.mock.calls[1][1]!.body as string).variables
		expect(second).toEqual({ id: "USDC-cNGN", side: "BID", fillChain: "EVM-8453", sourceChain: "EVM-1", after: "c1" })
	})
	it("reports a missing book for level orders as null", async () => {
		vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(JSON.stringify({ data: { book: null } })))
		expect(
			await new OrderbookClient("https://book/graphql", 5000).levelOrders("nope", {
				side: "ASK",
				fillChain: "EVM-1",
				priceBucket: "1",
			}),
		).toBeNull()
	})
})
