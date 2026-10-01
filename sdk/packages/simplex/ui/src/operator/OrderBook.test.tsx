// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import type { OrderbookSnapshot } from "../types"
import { OrderBook } from "./OrderBook"

const e18 = 10n ** 18n
const level = (price: bigint) => ({
	fillChain: "EVM-8453",
	priceBucket: price.toString(),
	price: (price * e18).toString(),
	worstPrice: (price * e18).toString(),
	baseSize: e18.toString(),
	quoteSize: (price * e18).toString(),
	orderCount: 1,
	solverCount: 1,
})
const snapshot: OrderbookSnapshot = {
	id: "USDC-cNGN",
	base: "USDC",
	quote: "cNGN",
	bids: [level(1360n)],
	asks: [level(1363n)],
	bidLiquidity: e18.toString(),
	askLiquidity: e18.toString(),
	granularity: (e18 / 100n).toString(),
}
const books = {
	books: [{ id: "USDC-cNGN", base: "USDC", quote: "cNGN" }],
	chains: [{ id: "EVM-8453", name: "Base" }],
}

let root: Root
let element: HTMLDivElement
const flush = () => act(async () => {})

beforeEach(() => {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	element = document.createElement("div")
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	vi.restoreAllMocks()
})

describe("order book page", () => {
	it("shows the summary and both sides of the ladder", async () => {
		vi.spyOn(api, "get").mockImplementation(async (path: string) =>
			path.startsWith("/api/orderbook/books") ? books : snapshot,
		)
		await act(async () => root.render(<OrderBook />))
		await flush()
		const text = element.textContent ?? ""
		expect(text).toContain("1,361.50")
		expect(text).toContain("22.03 bps")
		expect(element.querySelectorAll('[data-side="ASK"].orderbook-level')).toHaveLength(1)
		expect(element.querySelectorAll('[data-side="BID"].orderbook-level')).toHaveLength(1)
	})

	it("keeps the route filters when the book cannot be read, so the operator can switch away", async () => {
		vi.spyOn(api, "get").mockImplementation(async (path: string) => {
			if (path.startsWith("/api/orderbook/books")) return books
			throw new Error("offline")
		})
		await act(async () => root.render(<OrderBook />))
		await flush()
		expect(element.querySelector('[role="alert"]')?.textContent).toContain("temporarily unavailable")
		const labels = [...element.querySelectorAll(".orderbook-filter > span")].map((node) => node.textContent)
		expect(labels).toEqual(["Pair", "From", "To"])
		expect(element.querySelector(".orderbook-ladder")).toBeNull()
	})
})
