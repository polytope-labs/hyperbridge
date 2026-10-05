// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import type { ProfitabilityDto, ProfitFigures } from "../types"
import { Analytics } from "./Analytics"
import { ReturnMetric } from "./analytics/ReturnMetric"

const none: ProfitFigures = {
	profitUsd: 0,
	startInventoryUsd: 10_000,
	returnPct: 0,
	boughtUsd: 0,
	soldUsd: 0,
	buys: 0,
	sells: 0,
}
const DAY = 86_400_000
const start = new Date(2026, 9, 4).getTime()

/** Two days: nothing on the first, a buy and a sell on the second, against $10,000 of inventory. */
function summary(overrides: Partial<ProfitabilityDto> = {}): ProfitabilityDto {
	const traded: ProfitFigures = {
		profitUsd: 62.89,
		startInventoryUsd: 10_000,
		returnPct: 0.6289,
		boughtUsd: 5_000,
		soldUsd: 5_000,
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
				bought: 5_000,
				sold: 4_900,
				averageBuy: 1580,
				averageSell: 1590,
				spreadPct: 0.6329,
				baseChange: 100,
				quoteChange: -109_000,
				rate: 1585,
				profitUsd: 62.89,
			},
		],
		startInventory: {
			at: start,
			source: "snapshot",
			recordedAt: start - DAY,
			usd: 10_000,
			tokens: [
				{ symbol: "USDC", amount: 6_000, usd: 6_000 },
				{ symbol: "cNGN", amount: 6_340_000, usd: 4_000 },
			],
		},
		estimatedFills: 0,
		unpricedTokens: [],
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
		const figures = [...element.querySelectorAll(".operator-metrics > div")].map((figure) => figure.textContent)
		expect(figures).toEqual([
			"Profit+$62.892 fills in 30 days",
			"Return on inventory+0.63%Profit over starting inventory",
			"Starting inventory$10,000From the snapshot of Oct 3, 2026",
			"Volume$10,000Bought and sold",
		])
		// A gain reads green.
		expect(element.querySelector('.operator-metrics strong[data-tone="ok"]')?.textContent).toBe("+$62.89")
		expect(element.querySelector(".analytics-holdings")?.textContent).toBe(
			"The period began with 6,000 USDC and 6,340,000 cNGN.",
		)
	})

	it("sets each pair's buys against its sells, with what they did to its inventory", async () => {
		answerWith(() => summary())
		await act(async () => root.render(<Analytics />))
		await flush()

		const cells = [...element.querySelectorAll(".analytics-table")[0].querySelectorAll("tbody td")].map(
			(cell) => cell.textContent,
		)
		expect(cells).toEqual([
			"USDC / cNGN",
			"5,000 USDC",
			"1,580.00",
			"4,900 USDC",
			"1,590.00",
			"+0.63%",
			"+100 USDC-109,000 cNGN",
			"+$62.89",
		])
	})

	it("lists every bucket, latest first, and quiets the ones nothing happened in", async () => {
		answerWith(() => summary())
		await act(async () => root.render(<Analytics />))
		await flush()

		const rows = [...element.querySelectorAll(".analytics-table")].at(-1)?.querySelectorAll("tbody tr") ?? []
		expect(rows).toHaveLength(2)
		expect([...rows[0].querySelectorAll("td")].map((cell) => cell.textContent)).toEqual([
			"Oct 5",
			"$5,000",
			"$5,000",
			"$10,000",
			"+0.63%",
			"+$62.89",
		])
		expect(rows[1].textContent).toContain("Oct 4")
		expect(rows[1].hasAttribute("data-quiet")).toBe(true)
	})

	it("reads a loss red, with its sign", async () => {
		const lost = { ...summary().totals, profitUsd: -41.2, returnPct: -0.412 }
		answerWith(() => summary({ totals: lost }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const figures = [...element.querySelectorAll('.operator-metrics strong[data-tone="err"]')].map(
			(figure) => figure.textContent,
		)
		expect(figures).toEqual(["-$41.20", "-0.41%"])
	})

	it("states the profit without a return when the starting inventory is not on record", async () => {
		const totals = { ...summary().totals, startInventoryUsd: null, returnPct: null }
		answerWith(() => summary({ totals, startInventory: null }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const figures = [...element.querySelectorAll(".operator-metrics > div")].map((figure) => figure.textContent)
		expect(figures.slice(0, 3)).toEqual([
			"Profit+$62.892 fills in 30 days",
			"Return on inventory—Starting inventory not on record",
			"Starting inventory—Not on record for this period",
		])
		expect(element.querySelector(".analytics-holdings")).toBeNull()
	})

	it("says where a rebuilt starting inventory came from", async () => {
		const rebuilt = { ...summary().startInventory!, source: "balances" as const }
		answerWith(() => summary({ startInventory: rebuilt }))
		await act(async () => root.render(<Analytics />))
		await flush()

		expect(element.textContent).toContain("Rebuilt from today's balances")
	})

	it("says so when nothing filled, in place of charts and tables of nothing", async () => {
		answerWith(() => summary({ totals: none, series: [{ start, ...none }], books: [] }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const text = element.textContent ?? ""
		expect(text).toContain("Nothing filled in this period.")
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

	it("says which figures are estimates and which tokens the dollars leave out", async () => {
		answerWith(() => summary({ estimatedFills: 3, unpricedTokens: ["EURC", "ZARP"] }))
		await act(async () => root.render(<Analytics />))
		await flush()

		const method = element.querySelector(".analytics-method")?.textContent ?? ""
		expect(method).toContain("3 fills in this period were recorded before fills kept what they took in")
		expect(method).toContain("EURC, ZARP have no dollar price")
	})
})

describe("the overview's return", () => {
	it("shows the week's return and what it is a return on, and opens analytics", async () => {
		const get = answerWith(() => summary())
		const open = vi.fn()
		await act(async () => root.render(<ReturnMetric onOpen={open} />))
		await flush()

		expect(get).toHaveBeenCalledWith(expect.stringContaining("period=7d"))
		const link = element.querySelector("a")
		expect(link?.getAttribute("href")).toBe("/analytics")
		expect(link?.textContent).toBe("7-day return+0.63%+$62.89 on $10,000")
		expect(link?.querySelector("strong")?.getAttribute("data-tone")).toBe("ok")

		await act(async () => link?.click())
		expect(open).toHaveBeenCalledTimes(1)
	})

	it("reads a week that shrank inventory red", async () => {
		answerWith(() => summary({ totals: { ...summary().totals, profitUsd: -30, returnPct: -0.3 } }))
		await act(async () => root.render(<ReturnMetric onOpen={() => {}} />))
		await flush()

		expect(element.querySelector("strong")?.textContent).toBe("-0.30%")
		expect(element.querySelector("strong")?.getAttribute("data-tone")).toBe("err")
	})

	it("is a dash with the reason when there is nothing to compare", async () => {
		answerWith(() => summary({ totals: { ...none, startInventoryUsd: null, returnPct: null } }))
		await act(async () => root.render(<ReturnMetric onOpen={() => {}} />))
		await flush()

		expect(element.querySelector("strong")?.textContent).toBe("—")
		expect(element.querySelector("strong")?.getAttribute("data-tone")).toBe("")
		expect(element.textContent).toContain("No fills yet")
	})
})
