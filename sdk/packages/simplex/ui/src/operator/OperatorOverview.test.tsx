// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import type { BalanceSnapshot, StatusOperator } from "../types"
import { balancePollMs } from "./Operator"
import { OperatorOverview } from "./OperatorOverview"

vi.mock("../api", () => ({ api: { get: vi.fn(async () => ({ current: null })), post: vi.fn() } }))
vi.mock("./LimitOrders", () => ({ LimitOrders: () => null }))
vi.mock("./analytics/ReturnMetric", () => ({ ReturnMetric: () => null }))
vi.mock("./OperatorBalances", () => ({ OperatorBalances: () => null, availableStablecoinLiquidity: () => null }))

let root: Root
let element: HTMLDivElement

beforeEach(() => {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	// jsdom has no matchMedia, which the top-up dialogs read to pick a dialog or a drawer.
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
})

async function metricLabels(addresses: { evm: string; substrate?: string }): Promise<(string | null)[]> {
	const status = { chains: [8453], paused: false, addresses } as unknown as StatusOperator
	await act(async () =>
		root.render(
			<OperatorOverview
				status={status}
				balances={undefined}
				runtime={{ pending: false, onTogglePause: () => {}, onStop: () => {} }}
				onOpenAnalytics={() => {}}
				onBalancesChanged={() => {}}
			/>,
		),
	)
	return [...element.querySelectorAll(".operator-metrics > div span")].map((label) => label.textContent)
}

it("shows no BRIDGE balance for a solver without a Hyperbridge account", async () => {
	expect(await metricLabels({ evm: "0x01" })).toEqual(["Enabled networks", "Available liquidity"])
})

it("shows the BRIDGE balance for a solver with a Hyperbridge account", async () => {
	expect(await metricLabels({ evm: "0x01", substrate: "5Fxyz" })).toContain("BRIDGE available")
})

it("re-reads balances every second until the solver's first read has landed", () => {
	const snapshot = (status: BalanceSnapshot["status"]) => ({ status }) as BalanceSnapshot
	expect(balancePollMs(snapshot("loading"))).toBe(1_000)
	expect(balancePollMs(snapshot("fresh"))).toBe(30_000)
	// Before the dashboard's own first request returns, there is nothing to hurry.
	expect(balancePollMs(undefined)).toBe(30_000)
})
