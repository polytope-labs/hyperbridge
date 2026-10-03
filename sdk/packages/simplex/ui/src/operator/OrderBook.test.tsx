// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import type { OrderbookSnapshot } from "../types"
import { OrderBook } from "./OrderBook"
import { formatExpiry } from "./orderbook/OrderbookLevelDialog"

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
	// The level dialog asks whether it is on a phone; jsdom has no media queries to answer.
	window.matchMedia ??= ((query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addEventListener: () => {},
		removeEventListener: () => {},
		addListener: () => {},
		removeListener: () => {},
		dispatchEvent: () => false,
	})) as typeof window.matchMedia
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

	it("opens a level's solvers and orders when the level is clicked", async () => {
		const order = {
			solver: "0xce319986ca4d5d0893751a628d0db3dc8fc91d62",
			commitment: "0xa",
			fillChain: "EVM-8453",
			price: (1360n * e18).toString(),
			advertisedSize: (100_000_000n * e18).toString(),
			quotedAmount: (100_000_000n * e18).toString(),
			resized: false,
			expiresAt: new Date(Date.now() + 3 * 3_600_000 + 5 * 60_000 + 30_000).toISOString(),
			acceptedSources: ["EVM-8453"],
		}
		const get = vi.spyOn(api, "get").mockImplementation(async (path: string) => {
			if (path.startsWith("/api/orderbook/books")) return books
			if (path.startsWith("/api/orderbook/level-orders")) return [order]
			return snapshot
		})
		await act(async () => root.render(<OrderBook />))
		await flush()
		const bid = element.querySelector<HTMLElement>('[data-side="BID"].orderbook-level')!
		expect(bid.getAttribute("aria-haspopup")).toBe("dialog")
		await act(async () => bid.click())
		await flush()
		const levelCall = get.mock.calls.find(([path]) => path.startsWith("/api/orderbook/level-orders"))
		expect(levelCall?.[0]).toBe(
			`/api/orderbook/level-orders?book=USDC-cNGN&side=BID&fillChain=EVM-8453&priceBucket=1360`,
		)
		const dialog = document.querySelector('[role="dialog"]')!
		expect(dialog.querySelector(".orderbook-level-dialog-side")?.textContent).toBe("Buy level")
		expect(dialog.querySelector("h2")?.textContent).toBe("1,360.00 CNGN")
		expect(dialog.textContent).toContain("Fills on Base, swaps from any chain")
		expect(dialog.textContent).toContain("0xce31…1d62")
		expect(dialog.textContent).toContain("100,000,000.00")
		expect(dialog.textContent).toContain("Sizes in CNGN")
		expect(dialog.textContent).toContain("Expires in 3h 5m")
	})
})

describe("order expiry", () => {
	it("reads in days and hours, then hours and minutes, and says when an order has expired", () => {
		const now = Date.parse("2026-10-01T00:00:00Z")
		expect(formatExpiry("2027-10-01T00:00:00Z", now)).toBe("Expires in 365d 0h")
		expect(formatExpiry("2026-10-01T03:05:00Z", now)).toBe("Expires in 3h 5m")
		expect(formatExpiry("2026-10-01T00:42:00Z", now)).toBe("Expires in 42m")
		expect(formatExpiry("2026-09-30T23:59:00Z", now)).toBe("Expired")
		expect(formatExpiry("not a date", now)).toBe("—")
	})
})
