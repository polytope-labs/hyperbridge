import { afterEach, describe, expect, it, vi } from "vitest"
import { ORDERBOOK_DOCUMENTS, OrderbookClient, OrderbookRequestError, graphqlEndpoint } from "@/orderbook/client"

/**
 * A deployment serves GraphQL under its base URL. An operator who configures the base alone used
 * to get `HTTP 405 Method Not Allowed` on every request — the heartbeat interval, reconciliation
 * and each limit order — because the host redirects to a landing page that refuses POST.
 */
describe("the orderbook endpoint", () => {
	afterEach(() => vi.restoreAllMocks())

	it("adds /graphql to a base URL, and leaves one that has it alone", () => {
		expect(graphqlEndpoint("https://orderbook.hyperfx.finance/mainnet")).toBe(
			"https://orderbook.hyperfx.finance/mainnet/graphql",
		)
		expect(graphqlEndpoint("https://orderbook.hyperfx.finance/mainnet/")).toBe(
			"https://orderbook.hyperfx.finance/mainnet/graphql",
		)
		expect(graphqlEndpoint(" https://orderbook.hyperfx.finance/testnet/graphql ")).toBe(
			"https://orderbook.hyperfx.finance/testnet/graphql",
		)
		expect(graphqlEndpoint("http://127.0.0.1:8080/graphql")).toBe("http://127.0.0.1:8080/graphql")
	})

	it("leaves a URL carrying a query string as the operator wrote it", () => {
		expect(graphqlEndpoint("https://host/api?key=abc")).toBe("https://host/api?key=abc")
	})

	it("posts to the completed endpoint", async () => {
		const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
			new Response(JSON.stringify({ data: { serverInfo: { minOrderTtlSecs: 900 }, books: [] } }), {
				headers: { "content-type": "application/json" },
			}),
		)
		await new OrderbookClient("https://orderbook.hyperfx.finance/mainnet", 5_000).limits()

		expect(fetchMock.mock.calls[0][0]).toBe("https://orderbook.hyperfx.finance/mainnet/graphql")
	})

	it("asks whether heartbeat keys are supported on its own, so an older orderbook refuses that and nothing else", async () => {
		const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
			new Response(JSON.stringify({ data: { serverInfo: { heartbeatSignerTtlSecs: 7200 } } }), {
				headers: { "content-type": "application/json" },
			}),
		)
		const client = new OrderbookClient("https://orderbook.hyperfx.finance/mainnet", 5_000)

		expect(await client.heartbeatSignerTtlSecs()).toBe(7200)
		expect(JSON.parse(String(fetchMock.mock.calls[0][1]?.body)).query).not.toContain("minOrderTtlSecs")
		expect(ORDERBOOK_DOCUMENTS.limits).not.toContain("heartbeatSignerTtlSecs")
	})

	it("reads an older orderbook refusing the heartbeat key field as no TTL, and a lost request as a failure", async () => {
		const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
			new Response(JSON.stringify({ errors: [{ message: 'Unknown field "heartbeatSignerTtlSecs"' }] }), {
				headers: { "content-type": "application/json" },
			}),
		)
		const client = new OrderbookClient("https://orderbook.hyperfx.finance/mainnet", 5_000)
		expect(await client.heartbeatSignerTtlSecs()).toBeNull()

		fetchMock.mockRejectedValue(new Error("connection reset"))
		await expect(client.heartbeatSignerTtlSecs()).rejects.toThrow(OrderbookRequestError)
	})

	it("names the endpoint in an HTTP failure, so a misconfigured URL is visible", async () => {
		vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response("", { status: 405, statusText: "Method Not Allowed" }))

		await expect(new OrderbookClient("https://orderbook.hyperfx.finance/mainnet", 5_000).limits()).rejects.toThrow(
			"Orderbook at https://orderbook.hyperfx.finance/mainnet/graphql returned HTTP 405",
		)
	})
})
