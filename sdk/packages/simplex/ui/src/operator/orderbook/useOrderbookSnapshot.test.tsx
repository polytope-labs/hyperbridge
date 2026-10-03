// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../../api"
import type { OrderbookSnapshot } from "../../types"
import { useOrderbookSnapshot } from "./useOrderbookSnapshot"

const snapshot: OrderbookSnapshot = {
	id: "a",
	base: "USDC",
	quote: "cNGN",
	bids: [],
	asks: [],
	bidLiquidity: "0",
	askLiquidity: "0",
	granularity: null,
}
let root: Root
let element: HTMLDivElement
function Harness({ book = "a", source = "", fill = "" }: { book?: string; source?: string; fill?: string }) {
	const state = useOrderbookSnapshot(book, source, fill)
	return (
		<p>
			{state.snapshot?.id ?? "loading"}|{state.error ?? ""}
		</p>
	)
}

beforeEach(() => {
	vi.useFakeTimers()
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	element = document.createElement("div")
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	vi.useRealTimers()
	vi.restoreAllMocks()
})

describe("order book polling", () => {
	it("polls after completion, keeps same-route prices on failure, and recovers", async () => {
		const get = vi
			.spyOn(api, "get")
			.mockResolvedValueOnce(snapshot)
			.mockRejectedValueOnce(new Error("offline"))
			.mockResolvedValueOnce({ ...snapshot, id: "updated" })
		await act(async () => root.render(<Harness />))
		expect(element.textContent).toBe("a|")
		await act(async () => vi.advanceTimersByTimeAsync(10_000))
		expect(element.textContent).toBe("a|offline")
		await act(async () => vi.advanceTimersByTimeAsync(10_000))
		expect(element.textContent).toBe("updated|")
		await act(async () => root.render(null))
		await act(async () => vi.advanceTimersByTimeAsync(10_000))
		expect(get).toHaveBeenCalledTimes(3)
	})
	it("hides old route data immediately and ignores a late answer after switching routes", async () => {
		let resolveOld: (value: OrderbookSnapshot) => void = () => {}
		const get = vi
			.spyOn(api, "get")
			.mockImplementationOnce(
				() =>
					new Promise((resolve) => {
						resolveOld = resolve as typeof resolveOld
					}),
			)
			.mockResolvedValueOnce({ ...snapshot, id: "b" })
		await act(async () => root.render(<Harness />))
		const signal = get.mock.calls[0][1]!.signal!
		await act(async () => root.render(<Harness book="b" source="EVM-1" fill="EVM-8453" />))
		expect(signal.aborted).toBe(true)
		expect(element.textContent).toBe("b|")
		await act(async () => resolveOld(snapshot))
		expect(element.textContent).toBe("b|")
		expect(get.mock.calls[1][0]).toBe("/api/orderbook/snapshot?book=b&sourceChain=EVM-1&fillChain=EVM-8453")
	})
	it("does not overlap a slow request with another poll and aborts on unmount", async () => {
		const get = vi.spyOn(api, "get").mockImplementation(() => new Promise(() => {}))
		await act(async () => root.render(<Harness />))
		await act(async () => vi.advanceTimersByTimeAsync(30_000))
		expect(get).toHaveBeenCalledTimes(1)
		const signal = get.mock.calls[0][1]!.signal!
		await act(async () => root.render(null))
		expect(signal.aborted).toBe(true)
	})
	it("hides a completed snapshot while a new book is loading", async () => {
		vi.spyOn(api, "get")
			.mockResolvedValueOnce(snapshot)
			.mockImplementation(() => new Promise(() => {}))
		await act(async () => root.render(<Harness />))
		expect(element.textContent).toBe("a|")
		await act(async () => root.render(<Harness book="b" />))
		expect(element.textContent).toBe("loading|")
	})
})
