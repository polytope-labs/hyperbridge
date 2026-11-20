import { beforeEach, describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerRuntime } from "@/core/boot"
import type { OrderScanner } from "@/scanner/types"
import type { ChainFunding } from "@/services/chain-funding"
import { readChainFunding } from "@/services/read-chain-funding"
import { ChainController } from "@/simplex"

vi.mock("@/services/read-chain-funding", () => ({ readChainFunding: vi.fn() }))
vi.mock("@/services/bundler-preflight", () => ({ assertBundlersServeEntryPoint: vi.fn(async () => {}) }))

const FILLER = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const funded: ChainFunding = {
	feeToken: { symbol: "USDT", balance: 250 },
	nativeSetup: null,
	funded: true,
	delegated: false,
}

function controllerFor({
	signerless = false,
	globalWatchOnly = false,
	watchOnly = {} as Record<number, boolean>,
} = {}) {
	const intentFiller = {
		enableFills: vi.fn(async () => {}),
		guardDelegation: vi.fn(async () => {}),
		setWatchOnly: vi.fn(),
		getWatchOnly: () => watchOnly,
	}
	const runtime = {
		signerless,
		globalWatchOnly,
		config: { simplex: {} },
		configService: {},
		fillerAddress: FILLER,
		resolvedChains: [{ chainId: 56, rpcUrls: ["https://bsc.example"] }],
		chainClientManager: { getPublicClient: vi.fn(() => ({})) },
		intentFiller,
	} as unknown as FillerRuntime
	const persist = vi.fn()
	const controller = new ChainController(runtime, persist, {} as OrderScanner, true)
	return { controller, intentFiller, persist }
}

describe("ChainController.enableFills", () => {
	beforeEach(() => {
		vi.mocked(readChainFunding).mockReset()
		vi.mocked(readChainFunding).mockResolvedValue(funded)
	})

	it("delegates a funded running chain at once", async () => {
		const { controller, intentFiller, persist } = controllerFor()
		await controller.enableFills(56)
		expect(intentFiller.enableFills).toHaveBeenCalledWith(56)
		expect(readChainFunding).toHaveBeenCalledWith({}, {}, FILLER, "EVM-56")
		expect(persist).not.toHaveBeenCalled()
	})

	it("clears a watch-only flag left by an older config, and saves that", async () => {
		const { controller, intentFiller, persist } = controllerFor({ watchOnly: { 56: true } })
		await controller.enableFills(56)
		expect(intentFiller.setWatchOnly).toHaveBeenCalledWith(56, false)
		expect(persist).toHaveBeenCalled()
	})

	it("refuses an unfunded chain, naming what the wallet needs there", async () => {
		vi.mocked(readChainFunding).mockResolvedValue({
			...funded,
			feeToken: { symbol: "USDT", balance: 0 },
			nativeSetup: { required: 0.000142, feeToken: "USDT" },
			funded: false,
		})
		const { controller, intentFiller } = controllerFor()
		await expect(controller.enableFills(56)).rejects.toThrow(
			"The filler wallet needs at least 5 USDT and about 0.00014 BNB on BNB Chain before it can fill there",
		)
		expect(intentFiller.enableFills).not.toHaveBeenCalled()
	})

	it("refuses a chain that is not running", async () => {
		const { controller, intentFiller } = controllerFor()
		await expect(controller.enableFills(8453)).rejects.toThrow(/Base is not running yet/)
		expect(readChainFunding).not.toHaveBeenCalled()
		expect(intentFiller.enableFills).not.toHaveBeenCalled()
	})

	it("refuses a solver started without a signer", async () => {
		const { controller, intentFiller } = controllerFor({ signerless: true })
		await expect(controller.enableFills(56)).rejects.toThrow(/started without a signer/)
		expect(intentFiller.enableFills).not.toHaveBeenCalled()
	})

	it("refuses to enable fills while watch-only is in force", async () => {
		const { controller, intentFiller } = controllerFor({ globalWatchOnly: true })
		await expect(controller.enableFills(56)).rejects.toThrow(/runs watch-only/)
		expect(intentFiller.enableFills).not.toHaveBeenCalled()
	})
})

describe("ChainController.setWatchOnly", () => {
	it("refuses to clear watch-only while watch-only is in force", async () => {
		const { controller, intentFiller } = controllerFor({ globalWatchOnly: true, watchOnly: { 56: true } })
		await expect(controller.setWatchOnly(56, false)).rejects.toThrow(/runs watch-only/)
		expect(intentFiller.setWatchOnly).not.toHaveBeenCalled()
	})
})

describe("ChainController.setWatchOnly", () => {
	it("checks delegation before it turns watch-only off", async () => {
		const { controller, intentFiller, persist } = controllerFor({ watchOnly: { 56: true } })
		const order: string[] = []
		intentFiller.guardDelegation.mockImplementation(async () => void order.push("guard"))
		intentFiller.setWatchOnly.mockImplementation(() => void order.push("set"))

		await controller.setWatchOnly(56, false)
		expect(intentFiller.guardDelegation).toHaveBeenCalledWith(56)
		expect(order).toEqual(["guard", "set"])
		expect(persist).toHaveBeenCalled()
	})

	it("leaves the chain watch-only, and unsaved, when the delegation check rejects", async () => {
		const { controller, intentFiller, persist } = controllerFor({ watchOnly: { 56: true } })
		intentFiller.guardDelegation.mockRejectedValue(new Error("Could not read solver selection on EVM-56"))

		await expect(controller.setWatchOnly(56, false)).rejects.toThrow(/Could not read solver selection/)
		expect(intentFiller.setWatchOnly).not.toHaveBeenCalled()
		expect(persist).not.toHaveBeenCalled()
	})

	it("does not check delegation when it turns watch-only on", async () => {
		const { controller, intentFiller } = controllerFor()
		await controller.setWatchOnly(56, true)
		expect(intentFiller.guardDelegation).not.toHaveBeenCalled()
		expect(intentFiller.setWatchOnly).toHaveBeenCalledWith(56, true)
	})
})
