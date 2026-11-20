// @vitest-environment jsdom
import { act } from "react"
import { createRoot } from "react-dom/client"
import { afterEach, expect, it, vi } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import { api } from "../api"
import type { SetupDefaults } from "../types"
import { useLaunch } from "./launch"
import { assembleConfig, initialState } from "./state"
import { Wizard } from "./Wizard"

vi.mock("./orderbook", () => ({ loadOrderbook: vi.fn() }))
vi.mock("../components/InstallAppButton", () => ({ InstallAppButton: () => null }))

const defaults = {
	chains: chainsForNetwork("mainnet"),
	hyperbridgeWs: { mainnet: "wss://nexus.rpc.polytope.technology" },
	usdStables: ["USDC", "USDT"],
	maxConcurrentOrders: 5,
	configPath: "/tmp/filler-config.toml",
	knownTokens: {},
	knownVaults: {},
} as unknown as SetupDefaults

afterEach(() => {
	vi.restoreAllMocks()
})

it("asks for a wallet and chains, and nothing after them", async () => {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	const element = document.createElement("div")
	const root = createRoot(element)
	await act(async () => root.render(<Wizard defaults={defaults} />))
	const steps = [...element.querySelectorAll(".stepper .step strong")].map((step) => step.textContent)
	expect(steps).toEqual(["Wallet", "Chains"])
	await act(async () => root.unmount())
})

it("writes no Hyperbridge account and no vaults into the config", () => {
	const config = assembleConfig(initialState(defaults), defaults)
	expect(Object.keys(config).sort()).toEqual(["chains", "orderbook", "simplex"])
	expect(Object.keys(config.simplex).sort()).toEqual(["maxConcurrentOrders", "signer"])
})

/** Shows what `useLaunch` reports, with one button to start. */
function Launcher() {
	const launch = useLaunch(initialState(defaults), defaults)
	return (
		<button type="button" onClick={launch.start}>
			{launch.phase}
			{launch.error ? `: ${launch.error}` : ""}
		</button>
	)
}

async function launchWith(post: () => Promise<unknown>) {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	const saved = vi.spyOn(api, "post").mockImplementation(post)
	const element = document.createElement("div")
	const root = createRoot(element)
	await act(async () => root.render(<Launcher />))
	await act(async () => element.querySelector("button")?.click())
	const shown = element.textContent
	await act(async () => root.unmount())
	return { saved, shown }
}

it("saves the config and starts the solver from the last step", async () => {
	const { saved, shown } = await launchWith(async () => undefined)
	expect(saved).toHaveBeenCalledWith(
		"/api/setup/save-and-start",
		expect.objectContaining({ config: expect.anything() }),
	)
	expect(shown).toBe("starting")
})

it("reports a start the server refused, so the operator can retry", async () => {
	const { shown } = await launchWith(async () => {
		throw new Error("an [orderbook] section is required")
	})
	expect(shown).toBe("failed: an [orderbook] section is required")
})
