import { afterEach, describe, expect, it, vi } from "vitest"
import { OrderbookClient, graphqlEndpoint } from "@/orderbook/client"

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

	it("names the endpoint in an HTTP failure, so a misconfigured URL is visible", async () => {
		vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response("", { status: 405, statusText: "Method Not Allowed" }))

		await expect(new OrderbookClient("https://orderbook.hyperfx.finance/mainnet", 5_000).limits()).rejects.toThrow(
			"Orderbook at https://orderbook.hyperfx.finance/mainnet/graphql returned HTTP 405",
		)
	})
})
