// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import type { ProfitabilityDto, ProfitFigures } from "../types"
import { Analytics } from "./Analytics"
import { SpreadMetric } from "./analytics/SpreadMetric"

const none: ProfitFigures = {
	realizedUsd: 0,
	matchedUsd: 0,
	boughtUsd: 0,
	soldUsd: 0,
	spreadPct: null,
	buys: 0,
	sells: 0,
}
const DAY = 86_400_000
const start = new Date(2026, 9, 4).getTime()

/** Two days: nothing on the first, a matched buy and sell on the second. */
function summary(overrides: Partial<ProfitabilityDto> = {}): ProfitabilityDto {
	const traded: ProfitFigures = {
		realizedUsd: 62.89,
		matchedUsd: 10_000,
		boughtUsd: 10_000,
		soldUsd: 10_000,
		spreadPct: 0.6289,
		buys: 1,
		sells: 1,
	}
	return {
		period: "30d",
		bucket: "day",
		from: start,
		to: start + 2 * DAY,
		totals: traded,
		series: [
			{ start, ...none },
			{ start: start + DAY, ...traded },
		],
		books: [
			{
				book: "USDC-cNGN",
				base: "USDC",
				quote: "cNGN",
				bought: 10_000,
				sold: 10_000,
				averageBuy: 1580,
				averageSell: 1590,
				matched: 10_000,
				realized: 100_000,
				realizedUsd: 62.89,
				spreadPct: 0.6289,
				position: 2_500,
				positionUsd: 2_500,
			},
		],
		openPositionUsd: 2_500,
		estimatedFills: 0,
		unpricedBooks: [],
		...overrides,
	}
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

/** Answers every period with `answer`, stamped with the period that was asked for. */
function answerWith(answer: (period: string) => ProfitabilityDto) {
	return vi.spyOn(api, "get").mockImplementation(async (path: string) => {
		const period = new URL(path, "http://localhost").searchParams.get("period") ?? "7d"
		return { ...answer(period), period }
	})
}

describe("analytics page", () => {
	it("asks for thirty days on the viewer's clock and shows what they came to", async () => {
		const get = answerWith(() => summary())
		await act(async () => root.render(<Analytics />))
		await flush()

		expect(get).toHaveBeenCalledWith(`/api/analytics/profitability?period=30d&tz=${new Date().getTimezoneOffset()}`)
		const text = element.textContent ?? ""
		expect(text).toContain("+$62.89")
		expect(text).toContain("+0.63%")
		expect(text).toContain("$10,000")
		expect(text).toContain("2 fills in 30 days")
		// The pair's own figures, and what it still holds.
		expect(text).toContain("1,580.00")
		expect(text).toContain("1,590.00")
		expect(text).toContain("+2,500 USDC")
		// A gain reads green.
		expect(element.querySelector('.operator-metrics strong[data-tone="ok"]')?.textContent).toBe("+$62.89")
	})

	it("lists every bucket, latest first, and quiets the ones nothing happened in", async () => {
		answerWith(() => summary())
		await act(async () => root.render(<Analytics />))
		await flush()

		const rows = [...element.querySelectorAll(".analytics-table")].at(-1)?.querySelectorAll("tbody tr") ?? []
		expect(rows).toHaveLength(2)
		expect(rows[0].textContent).toContain("Oct 5")
		expect(rows[0].textContent).toContain("+$62.89")
		expect(rows[1].textContent).toContain("Oct 4")
		expect(rows[1].hasAttribute("data-quiet")).toBe(true)
	})

	it("reads a loss red, with its sign", async () => {
		const lost = { ...summary().totals, realizedUsd: -41.2, spreadPct: -0.412 }
		answerWith(() => summary({ totals: lost }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const figures = [...element.querySelectorAll('.operator-metrics strong[data-tone="err"]')].map(
			(figure) => figure.textContent,
		)
		expect(figures).toEqual(["-$41.20", "-0.41%"])
	})

	it("says so when nothing filled, in place of charts and tables of nothing", async () => {
		answerWith(() => summary({ totals: none, series: [{ start, ...none }], books: [], openPositionUsd: 0 }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const text = element.textContent ?? ""
		expect(text).toContain("Nothing filled in this period.")
		expect(text).toContain("No fills yet")
		expect(element.querySelector(".analytics-charts")).toBeNull()
		expect(element.querySelector(".analytics-table")).toBeNull()
	})

	it("asks again when the period changes, and never shows one period's figures under another", async () => {
		let release: (() => void) | undefined
		const get = vi.spyOn(api, "get").mockImplementation(async (path: string) => {
			const period = new URL(path, "http://localhost").searchParams.get("period") ?? "7d"
			// The second period's answer is held back until the test lets it go.
			if (period === "12w") await new Promise<void>((resolve) => (release = resolve))
			return { ...summary(), period, bucket: period === "12w" ? "week" : "day" }
		})
		await act(async () => root.render(<Analytics />))
		await flush()
		expect(element.textContent).toContain("+$62.89")

		const weeks = [...element.querySelectorAll("button")].find((button) => button.textContent === "12 weeks")
		await act(async () => weeks?.click())
		expect(get).toHaveBeenLastCalledWith(expect.stringContaining("period=12w"))
		expect(element.textContent).not.toContain("+$62.89")
		expect(element.textContent).toContain("Reading your fills")

		await act(async () => release?.())
		await flush()
		expect(element.textContent).toContain("Every week in the period")
	})

	it("says which figures are estimates and which pairs the dollars leave out", async () => {
		answerWith(() => summary({ estimatedFills: 3, unpricedBooks: ["EURC-cNGN"] }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const method = element.querySelector(".analytics-method")?.textContent ?? ""
		expect(method).toContain("3 fills in this period were recorded before fills kept what they took in")
		expect(method).toContain("EURC-cNGN has no dollar-stable token")
	})
})

describe("the overview's spread", () => {
	it("shows the week's spread and what it realized, and opens analytics", async () => {
		const get = answerWith(() => summary())
		const open = vi.fn()
		await act(async () => root.render(<SpreadMetric onOpen={open} />))
		await flush()

		expect(get).toHaveBeenCalledWith(expect.stringContaining("period=7d"))
		const link = element.querySelector("a")
		expect(link?.getAttribute("href")).toBe("/analytics")
		expect(link?.textContent).toContain("+0.63%")
		expect(link?.textContent).toContain("+$62.89 realized")
		expect(link?.querySelector("strong")?.getAttribute("data-tone")).toBe("ok")

		await act(async () => link?.click())
		expect(open).toHaveBeenCalledTimes(1)
	})

	it("is a dash with the reason while only one side has traded", async () => {
		answerWith(() => summary({ totals: { ...none, buys: 4, boughtUsd: 900 } }))
		await act(async () => root.render(<SpreadMetric onOpen={() => {}} />))
		await flush()

		expect(element.querySelector("strong")?.textContent).toBe("—")
		expect(element.querySelector("strong")?.getAttribute("data-tone")).toBe("")
		expect(element.textContent).toContain("Buys only, nothing sold yet")
	})
})
