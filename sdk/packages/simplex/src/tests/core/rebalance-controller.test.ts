import { describe, expect, it, vi } from "vitest"
import type { RebalancingConfig } from "@/config/filler-toml"
import type { FillerRuntime } from "@/core/boot"
import { RebalanceController } from "@/simplex"

const USDC_HUB = { asset: "USDC", chain: "EVM-8453", threshold: "6000", minBalance: "5000" }
const USDT_HUB = { asset: "USDT", chain: "EVM-42161", threshold: "6000", minBalance: "5000" }
const COVERAGE = [{ asset: "USDC", hub: "EVM-8453", chains: [{ chain: "EVM-137", method: "cctp" as const }] }]

/** The controller over a runtime that only records what reaches it. */
function setUp(rebalancing?: RebalancingConfig) {
	const runtime = {
		config: { rebalancing },
		configService: { setRebalancing: vi.fn() },
		rebalancingService: { preflight: vi.fn(() => COVERAGE) },
		intentFiller: { rebalanceNow: vi.fn(async () => ({ started: [], skipped: [] })) },
	}
	const persist = vi.fn()
	const controller = new RebalanceController(runtime as unknown as FillerRuntime, persist)
	return { runtime, persist, controller }
}

describe("RebalanceController", () => {
	it("lists the saved hubs", () => {
		expect(setUp().controller.list()).toEqual([])
		expect(setUp({ hubs: [USDC_HUB] }).controller.list()).toEqual([USDC_HUB])
	})

	it("preflights a hub set against the running solver and returns what it covers", () => {
		const { controller, runtime, persist } = setUp()
		expect(controller.preflight([USDC_HUB])).toBe(COVERAGE)
		expect(runtime.rebalancingService.preflight).toHaveBeenCalledWith({ hubs: [USDC_HUB] })
		expect(runtime.configService.setRebalancing).not.toHaveBeenCalled()
		expect(persist).not.toHaveBeenCalled()
	})

	it("applies a hub set to the running solver and persists it, keeping the configured interval", async () => {
		const { controller, runtime, persist } = setUp({ hubs: [USDC_HUB], intervalMs: 600_000 })

		await controller.set([USDC_HUB, USDT_HUB])
		const saved = { hubs: [USDC_HUB, USDT_HUB], intervalMs: 600_000 }
		expect(runtime.rebalancingService.preflight).toHaveBeenCalledWith(saved)
		expect(runtime.configService.setRebalancing).toHaveBeenCalledWith(saved)
		expect(runtime.config.rebalancing).toEqual(saved)
		expect(persist).toHaveBeenCalledOnce()

		await controller.set([USDC_HUB], { intervalMs: 60_000 })
		expect(runtime.config.rebalancing).toEqual({ hubs: [USDC_HUB], intervalMs: 60_000 })
	})

	it("switches rebalancing off with an empty set", async () => {
		const off = setUp({ hubs: [USDC_HUB] })
		await off.controller.set([])
		expect(off.runtime.configService.setRebalancing).toHaveBeenCalledWith(undefined)
		expect(off.runtime.config.rebalancing).toBeUndefined()

		// A configured interval is the operator's too, and stays for when hubs come back.
		const kept = setUp({ hubs: [USDC_HUB], intervalMs: 600_000 })
		await kept.controller.set([])
		expect(kept.runtime.config.rebalancing).toEqual({ hubs: [], intervalMs: 600_000 })
	})

	it("changes nothing when the running solver refuses the set", async () => {
		const { controller, runtime, persist } = setUp({ hubs: [USDC_HUB] })
		runtime.rebalancingService.preflight.mockImplementation(() => {
			throw new Error("rebalancing.hubs: USDT's hub EVM-1 is not a chain this solver runs")
		})

		await expect(controller.set([{ ...USDT_HUB, chain: "EVM-1" }])).rejects.toThrow(/not a chain this solver runs/)
		expect(runtime.configService.setRebalancing).not.toHaveBeenCalled()
		expect(runtime.config.rebalancing).toEqual({ hubs: [USDC_HUB] })
		expect(persist).not.toHaveBeenCalled()
	})

	it("runs a pass through the filler, which is what refuses a second one", async () => {
		const { controller, runtime } = setUp()
		await expect(controller.rebalanceNow()).resolves.toEqual({ started: [], skipped: [] })
		expect(runtime.intentFiller.rebalanceNow).toHaveBeenCalledOnce()
	})
})
