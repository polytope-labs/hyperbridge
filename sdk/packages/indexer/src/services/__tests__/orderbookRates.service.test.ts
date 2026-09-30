;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

jest.mock("@/constants", () => ({ ENV_CONFIG: {} }))
jest.mock("@/utils/safeFetch", () => ({ safeFetch: jest.fn() }))

import {
	FAILURE_TTL_MS,
	FETCH_TIMEOUT_MS,
	MAX_SYMBOL_LENGTH,
	PRICE_TTL_MS,
	fetchOrderbookUsdPrice,
	resetOrderbookRates,
} from "@/services/orderbookRates.service"
import { safeFetch } from "@/utils/safeFetch"

const fetchMock = jest.mocked(safeFetch)
const URL = "https://orderbook.example/graphql"
const NOW = Date.parse("2026-09-19T12:00:00Z")

/** A rate at 1e18, as the orderbook's BigInt scalar writes it. */
const e18 = (whole: number) => (BigInt(Math.round(whole * 1e6)) * 10n ** 12n).toString()

function respond(body: unknown, status = 200) {
	fetchMock.mockResolvedValueOnce({
		ok: status >= 200 && status < 300,
		status,
		statusText: "",
		headers: {},
		json: async () => body,
		text: async () => JSON.stringify(body),
	})
}

/**
 * The orderbook's answer for the first stable (USDC), with the second unquoted. A `bestRate` alias
 * that errors is left out of `data` entirely rather than nulled, and the aliases beside it still
 * answer. `base` and `quote` name the pair and never flip with the side.
 */
const quoted = (base: string, quote: string, rate: string) => ({
	data: { q0: { base, quote, rate } },
	errors: [{ message: "no book trades X for USDT" }],
})

/** A book quoting the token in dollars, e.g. the WETH/USDC pair. */
const pricedInUsd = (rate: string) => quoted("WETH", "USDC", rate)

const price = (symbol: string, now = NOW) => fetchOrderbookUsdPrice(symbol, { url: URL, now })

const sentBody = (call = 0) => JSON.parse(fetchMock.mock.calls[call][1]!.body!)

beforeEach(() => {
	jest.clearAllMocks()
	fetchMock.mockReset()
	resetOrderbookRates()
})

test("a rate quoted in the stable is the price as it stands: the token is the base", async () => {
	respond(quoted("WETH", "USDC", e18(2_500)))

	expect((await price("WETH"))?.toFixed(2)).toBe("2500.00")
})

test("a rate quoted in the token is inverted: the stable is the base", async () => {
	// The USDC/cNGN book quotes 1,500.5 cNGN per USDC, so one cNGN is worth 1/1500.5 dollars. The
	// pair is named base-first whichever way it is read, so this is the shape for cNGN either way.
	respond(quoted("USDC", "cNGN", e18(1_500.5)))

	const usd = await price("cNGN")
	expect(usd?.toFixed(9)).toBe("0.000666445")
	expect(usd?.times(1_500.5).toFixed(6)).toBe("1.000000")
})

test("a rate naming neither side the stable asked about is no price", async () => {
	respond({ data: { q0: { base: "WETH", quote: "DAI", rate: e18(2_500) } } })

	expect(await price("WETH")).toBeNull()
})

test("both stables are asked for in one request, with the symbol as a variable", async () => {
	respond(pricedInUsd(e18(3)))
	await price("cNGN")

	expect(fetchMock).toHaveBeenCalledTimes(1)
	const [url, options] = fetchMock.mock.calls[0]
	expect(url).toBe(URL)
	expect(options).toMatchObject({ method: "POST", timeoutMs: FETCH_TIMEOUT_MS })
	const body = sentBody()
	// The symbol is never interpolated into the query: it comes off an ERC-20 `symbol()` call.
	expect(body.variables).toEqual({ symbol: "cNGN" })
	expect(body.query).toContain('tokenOut: "USDC"')
	expect(body.query).toContain('tokenOut: "USDT"')
	expect(body.query).not.toContain("cNGN")
	// The units are read off the entry, not derived from the side.
	expect(body.query).toContain("{ base quote rate }")
})

test("the second stable answers when the first book does not quote the token", async () => {
	respond({
		data: { q1: { base: "PEPE", quote: "USDT", rate: e18(4) } },
		errors: [{ message: "no book trades PEPE for USDC" }],
	})

	expect((await price("PEPE"))?.toFixed(0)).toBe("4")
})

test("a token no book quotes is unpriced, and is not asked about again until the TTL lapses", async () => {
	// Every alias errored, so the orderbook sends no data at all.
	respond({
		data: null,
		errors: [{ message: "no book trades SHIB for USDC" }, { message: "no book trades SHIB for USDT" }],
	})

	expect(await price("SHIB")).toBeNull()
	expect(await price("SHIB", NOW + PRICE_TTL_MS - 1)).toBeNull()
	expect(fetchMock).toHaveBeenCalledTimes(1)

	respond(pricedInUsd(e18(7)))
	expect((await price("SHIB", NOW + PRICE_TTL_MS))?.toFixed(0)).toBe("7")
	expect(fetchMock).toHaveBeenCalledTimes(2)
})

test("a price is reused for the TTL, per symbol", async () => {
	respond(pricedInUsd(e18(2_500)))
	respond(pricedInUsd(e18(1)))

	expect((await price("WETH"))?.toFixed(0)).toBe("2500")
	expect((await price("WETH", NOW + PRICE_TTL_MS - 1))?.toFixed(0)).toBe("2500")
	expect((await price("DAI"))?.toFixed(0)).toBe("1")
	expect(fetchMock).toHaveBeenCalledTimes(2)

	respond(pricedInUsd(e18(2_600)))
	expect((await price("WETH", NOW + PRICE_TTL_MS))?.toFixed(0)).toBe("2600")
})

test("orders in one block share a single request rather than racing", async () => {
	respond(pricedInUsd(e18(2_500)))

	const prices = await Promise.all([price("WETH"), price("WETH"), price("WETH")])

	expect(prices.map((value) => value?.toFixed(0))).toEqual(["2500", "2500", "2500"])
	expect(fetchMock).toHaveBeenCalledTimes(1)
})

test("an unreachable orderbook leaves the token unpriced, never throws, and is retried sooner", async () => {
	fetchMock.mockRejectedValueOnce(new Error("connect ECONNREFUSED"))

	await expect(price("WETH")).resolves.toBeNull()
	expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("ECONNREFUSED"))

	// Still cached briefly, so one bad block does not become one request per order.
	await expect(price("WETH", NOW + FAILURE_TTL_MS - 1)).resolves.toBeNull()
	expect(fetchMock).toHaveBeenCalledTimes(1)

	respond(pricedInUsd(e18(2_500)))
	expect((await price("WETH", NOW + FAILURE_TTL_MS))?.toFixed(0)).toBe("2500")
})

test("an HTTP error and an unrecognised body are both failures, not answers", async () => {
	respond({}, 503)
	await expect(price("WETH")).resolves.toBeNull()

	respond("not graphql")
	await expect(price("cNGN")).resolves.toBeNull()

	expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("503"))
	expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("unrecognised body"))
})

test("a rate that is not a positive number is no price", async () => {
	for (const rate of [e18(0), "-1", "not a number", null]) {
		resetOrderbookRates()
		respond({ data: { q0: { base: "WETH", quote: "USDC", rate } } })
		expect(await price("WETH")).toBeNull()
	}
})

test("nothing is asked about a token whose symbol() returned junk", async () => {
	expect(await price("")).toBeNull()
	expect(await price("x".repeat(MAX_SYMBOL_LENGTH + 1))).toBeNull()
	expect(fetchMock).not.toHaveBeenCalled()
})

test("nothing is fetched, and the reason is reported at most once a minute, with no URL configured", async () => {
	expect(await fetchOrderbookUsdPrice("WETH", { now: NOW })).toBeNull()
	expect(await fetchOrderbookUsdPrice("WETH", { now: NOW + 1_000 })).toBeNull()

	expect(fetchMock).not.toHaveBeenCalled()
	expect(logger.info).toHaveBeenCalledTimes(1)
	expect(logger.info).toHaveBeenCalledWith(expect.stringContaining("HYPERFX_ORDERBOOK_URL is not set"))
})
