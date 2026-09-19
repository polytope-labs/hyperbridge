;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

const mockEnv: Record<string, string | null | undefined> = {}
jest.mock("@/constants", () => ({ ENV_CONFIG: mockEnv }))

import { GRAPHQL_PATH, ORDERBOOK_URL_VAR, WATCHLIST_PATH, orderbookEndpoint } from "@/utils/orderbook"

const configure = (value: string | null | undefined) => {
	mockEnv[ORDERBOOK_URL_VAR] = value
}

const endpoints = () => [orderbookEndpoint(WATCHLIST_PATH), orderbookEndpoint(GRAPHQL_PATH)]

beforeEach(() => {
	jest.clearAllMocks()
	configure(undefined)
})

test("one base URL serves both endpoints", () => {
	configure("https://orderbook.example")

	expect(endpoints()).toEqual(["https://orderbook.example/solvers", "https://orderbook.example/graphql"])
})

test("a trailing slash and a port make no difference", () => {
	configure("http://127.0.0.1:8547/")

	expect(endpoints()).toEqual(["http://127.0.0.1:8547/solvers", "http://127.0.0.1:8547/graphql"])
})

test("an orderbook served under a prefix keeps it", () => {
	configure("https://example.test/orderbook")

	expect(endpoints()).toEqual(["https://example.test/orderbook/solvers", "https://example.test/orderbook/graphql"])
})

test("a base that already names an endpoint is taken as the host it is on", () => {
	// The value HYPERFX_WATCHLIST_URL held, which an operator is most likely to carry over.
	configure("https://orderbook.example/solvers")
	expect(endpoints()).toEqual(["https://orderbook.example/solvers", "https://orderbook.example/graphql"])

	configure("https://example.test/orderbook/graphql")
	expect(endpoints()).toEqual(["https://example.test/orderbook/solvers", "https://example.test/orderbook/graphql"])
})

test("nothing is resolved without a configured orderbook", () => {
	expect(endpoints()).toEqual([undefined, undefined])

	configure("")
	expect(endpoints()).toEqual([undefined, undefined])
})

test("a configured value that is not a URL resolves nothing, and says so", () => {
	configure("orderbook.example")

	expect(endpoints()).toEqual([undefined, undefined])
	expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("is not a URL"))
})
