// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import { formatDate, sqliteUtcToMs } from "../lib/format"
import type { BalanceSnapshot, LimitOrder } from "../types"
import { LimitOrders } from "./LimitOrders"

const ONE = 10n ** 18n

function order(overrides: Partial<LimitOrder> = {}): LimitOrder {
	return {
		id: "limit-0",
		book: "USDC-cNGN",
		base: "USDC",
		quote: "cNGN",
		side: "BID",
		fillChain: "EVM-8453",
		price: (1545n * ONE).toString(),
		size: (1_545_000n * ONE).toString(),
		remaining: (1_545_000n * ONE).toString(),
		reserved: "0",
		acceptedSources: ["EVM-8453"],
		ttlSecs: 900,
		expiresAt: null,
		status: "open",
		commitment: "0xabc",
		orderNonce: "0",
		bookExpiresAt: null,
		bookPrice: null,
		lastError: null,
		createdAt: "2026-10-01 10:00:00",
		updatedAt: "2026-10-01 10:00:00",
		...overrides,
	} as LimitOrder
}

const held = (symbol: string, amount: number) => ({
	address: "0x0000000000000000000000000000000000000000",
	symbol,
	wallet: amount,
	walletReserve: 0,
	vaultPosition: 0,
	vaultAvailable: 0,
	total: amount,
	available: amount,
	vaults: [],
	status: "fresh" as const,
})
const balances: BalanceSnapshot = {
	updatedAt: 0,
	status: "fresh",
	chains: [{ chainId: 8453, assets: [held("USDC", 10_000), held("cNGN", 20_000_000)] }],
	issues: [],
}

let root: Root
let element: HTMLDivElement
/** What the list endpoint answers with, newest first, as the server orders it. */
let orders: LimitOrder[]
/** What the orderbook makes of the next order posted. */
let answer: (request: { tokenOut: string }) => LimitOrder
const flush = () => act(async () => {})

const rows = () => [...element.querySelectorAll<HTMLElement>(".limit-order-item")]
const line = (row: HTMLElement) => row.querySelector(".limit-order-item-progress small")
const pressed = (group: string) =>
	element.querySelector(`[aria-label="${group}"] [aria-pressed="true"]`)?.firstChild?.textContent
const button = (scope: ParentNode, label: string) => {
	const found = [...scope.querySelectorAll("button")].find((entry) => entry.textContent?.trim().startsWith(label))
	if (!found) throw new Error(`No "${label}" button`)
	return found
}
const click = (target: HTMLElement) => act(async () => target.click())

async function enter(input: HTMLInputElement, value: string) {
	await act(async () => {
		Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value)
		input.dispatchEvent(new Event("input", { bubbles: true }))
	})
}

/** Posts an order from the form, on the side given, the way an operator fills it in. */
async function post(side: "Buy" | "Sell") {
	await click(element.querySelector<HTMLElement>(".limit-order-new-button")!)
	const form = document.querySelector<HTMLElement>('[role="dialog"]')!
	await click(button(form, `${side} USDC`))
	await enter(form.querySelector<HTMLInputElement>('input[aria-label^="Amount in"]')!, "5000")
	await enter(form.querySelector<HTMLInputElement>('input[aria-label^="Rate in"]')!, "1545")
	await click(button(form, "Post limit order"))
	await flush()
}

beforeEach(async () => {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	// The sheet asks whether it is on a phone; jsdom has no media queries to answer.
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
	orders = []
	answer = (request) => order({ id: "limit-new", side: request.tokenOut === "USDC" ? "ASK" : "BID" })
	vi.spyOn(api, "get").mockImplementation(async (path: string) =>
		path.startsWith("/api/orderbook/books")
			? { books: [{ id: "USDC-cNGN", base: "USDC", quote: "cNGN" }], chains: [] }
			: { orders },
	)
	vi.spyOn(api, "post").mockImplementation(async (_path: string, body?: unknown) => {
		const created = answer(body as { tokenOut: string })
		orders = [created, ...orders]
		return { order: created, result: { kind: "accepted" } }
	})
	element = document.createElement("div")
	document.body.append(element)
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	element.remove()
	vi.restoreAllMocks()
})

const render = async () => {
	await act(async () =>
		root.render(<LimitOrders chains={[8453]} chainLabels={{ "8453": "Base" }} balances={balances} />),
	)
	await flush()
}

describe("a live order that is not on the book", () => {
	it("keeps its Posting badge when the last posting failed, and says why", async () => {
		orders = [order({ commitment: null, lastError: "REQUEST_FAILED: fetch failed" })]
		await render()
		const [row] = rows()
		expect(row.querySelector(".badge")?.textContent).toBe("Posting")
		expect(line(row)?.textContent).toContain("REQUEST_FAILED: fetch failed")
		expect(line(row)?.getAttribute("data-tone")).toBe("warn")
	})

	it("leaves an order on the book unbadged, with any problem on its second line", async () => {
		orders = [order({ id: "limit-1", lastError: "UNDER_FUNDED: the balance covers 200" }), order()]
		await render()
		const [troubled, healthy] = rows()
		expect(troubled.querySelector(".badge")).toBeNull()
		expect(line(troubled)?.textContent).toContain("UNDER_FUNDED")
		expect(line(troubled)?.getAttribute("data-tone")).toBe("warn")
		expect(healthy.querySelector(".badge")).toBeNull()
		expect(line(healthy)?.getAttribute("data-tone")).toBeNull()
	})
})

describe("posting an order from another list", () => {
	it("shows the new order's side and its live list", async () => {
		orders = [order({ status: "filled", remaining: "0" })]
		await render()
		await click(button(element, "Filled"))
		expect(rows()).toHaveLength(1)

		await post("Sell")
		expect(document.querySelector('[role="dialog"]')).toBeNull()
		expect(pressed("Order side")).toBe("Sells")
		expect(pressed("Order status")).toBe("Live")
		expect(rows()).toHaveLength(1)
		expect(rows()[0].getAttribute("data-side")).toBe("ASK")
	})

	it("returns to the first page, where the newest order is", async () => {
		orders = Array.from({ length: 11 }, (_, index) => order({ id: `limit-${index}` }))
		await render()
		await click(element.querySelector<HTMLElement>('[aria-label="Next page"]')!)
		expect(rows()).toHaveLength(1)

		await post("Buy")
		expect(element.querySelector('[aria-current="page"]')?.textContent).toBe("1")
		expect(rows()).toHaveLength(10)
	})

	it("shows an order the orderbook refused where it is kept, under Cancelled", async () => {
		answer = () =>
			order({ id: "limit-new", status: "rejected", commitment: null, lastError: "TTL_TOO_SHORT: minimum is 900" })
		await render()

		await post("Buy")
		expect(pressed("Order side")).toBe("Buys")
		expect(pressed("Order status")).toBe("Cancelled")
		const [row] = rows()
		expect(row.querySelector(".badge")?.textContent).toBe("Refused")
		expect(line(row)?.textContent).toContain("TTL_TOO_SHORT: minimum is 900")
	})
})

describe("a closed order's second line", () => {
	const when = formatDate(sqliteUtcToMs("2026-10-03 08:30:00"))

	it("dates an order closed under the dust floor", async () => {
		orders = [order({ status: "filled", remaining: (3n * ONE).toString(), updatedAt: "2026-10-03 08:30:00" })]
		await render()
		await click(button(element, "Filled"))
		expect(line(rows()[0])?.textContent).toBe(`${when} · closed with 3 cNGN left, below the orderbook's dust floor`)
	})

	it("dates a refusal, ahead of the orderbook's reason", async () => {
		orders = [
			order({
				status: "rejected",
				commitment: null,
				lastError: "TTL_TOO_SHORT: minimum is 900",
				updatedAt: "2026-10-03 08:30:00",
			}),
		]
		await render()
		await click(button(element, "Cancelled"))
		expect(line(rows()[0])?.textContent).toBe(`refused ${when} · TTL_TOO_SHORT: minimum is 900`)
	})
})
