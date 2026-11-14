// @vitest-environment jsdom
import { act } from "react"
import { createRoot } from "react-dom/client"
import { expect, it, vi } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import type { SetupDefaults } from "../types"
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

it("asks for a signer and chains, then reviews", async () => {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	const element = document.createElement("div")
	const root = createRoot(element)
	await act(async () => root.render(<Wizard defaults={defaults} />))
	const steps = [...element.querySelectorAll(".stepper .step strong")].map((step) => step.textContent)
	expect(steps).toEqual(["Signer", "Chains", "Review & launch"])
	await act(async () => root.unmount())
})

it("writes no Hyperbridge account and no vaults into the config", () => {
	const config = assembleConfig(initialState(defaults), defaults)
	expect(Object.keys(config).sort()).toEqual(["chains", "orderbook", "simplex"])
	expect(Object.keys(config.simplex).sort()).toEqual(["maxConcurrentOrders", "signer"])
})
