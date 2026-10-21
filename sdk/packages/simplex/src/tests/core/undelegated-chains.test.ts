import { afterEach, describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * A chain whose EIP-7702 delegation failed at boot must not be bid on. The
 * bundler's estimate injects the SolverAccount code by state override, so an
 * undelegated EOA's bid passes estimation and pays the Hyperbridge bid fees and
 * deposit for a fill that can never execute.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString

function build(
	results: Record<string, boolean>,
	options: {
		chainConfigs?: number[]
		solverSelectionActive?: boolean
		solverSelectionFails?: boolean
		/** The shared cache starts empty, as for a chain that was watch-only at boot. */
		cacheEmpty?: boolean
		maxConcurrentOrders?: number
	} = {},
) {
	const chainConfigs = options.chainConfigs ?? [1, 8453]
	const delegationResults = { ...results }
	// Once the operator enables fills on a chain, its delegation succeeds.
	const delegate = (chain: string) => {
		delegationResults[chain] = true
	}
	const setupDelegationOnChains = vi.fn(async (chains: string[]) => ({
		success: chains.every((chain) => delegationResults[chain]),
		results: Object.fromEntries(chains.map((chain) => [chain, delegationResults[chain]])),
	}))
	// What the chain itself says, as the dashboard's `getCode` read does. It can
	// disagree with the boot result: a delegation that mined after its timeout.
	const onChain: Record<string, boolean> = {}
	const isDelegated = vi.fn(async (chain: string) => onChain[chain] ?? false)
	const isUserAllowed = vi.fn(() => false) // ends the happy path right after the guards
	// The shared cache the intake check reads. Boot fills it for the chains that fill;
	// a chain that was watch-only at boot has no entry until a read of solver selection
	// writes one, which is what `ContractInteractionService.isSolverSelectionActive` does.
	const cache = new Map<string, boolean>(options.cacheEmpty ? [] : [["EVM-1", false], ["EVM-8453", false]])
	const getSolverSelection = vi.fn((chain: string) => cache.get(chain) ?? null)

	const filler = new IntentFiller(
		chainConfigs.map((chainId) => ({ chainId }) as never),
		[],
		{ maxConcurrentOrders: options.maxConcurrentOrders ?? 1 } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getConfiguredChainIds: () => [1, 8453],
			isUserAllowed,
			getTargetGasUnits: () => 1_000_000,
			getSimplexPaymasterAddress: () => undefined,
		} as never,
		{} as never,
		{
			getCache: () => ({ getSolverSelection }),
			isSolverSelectionActive: async (chain: string) => {
				if (options.solverSelectionFails) throw new Error("rpc down")
				const active = options.solverSelectionActive ?? true
				cache.set(chain, active)
				return active
			},
			topUpEntryPointDeposit: async () => {},
		} as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner(chainConfigs) },
		undefined,
		new MemoryDataStore().bids,
	)
	// `setupSolverSelection` bails without a Hyperbridge connection, and delegates
	// through this service. Both are stubbed to the failure under test.
	;(filler as any).hyperbridge = Promise.resolve({})
	;(filler as any).delegationService = { setupDelegationOnChains, isDelegated }
	return { filler, isUserAllowed, delegate, isDelegated, onChain, cache }
}

async function detect(filler: IntentFiller, destination: string) {
	;(filler as any).handleNewOrder({ id: "0xorder", user: "0xUSER", source: "EVM-1", destination }, "0xtx")
	await (filler as any).globalQueue.onIdle()
}

describe("a chain whose delegation failed", () => {
	afterEach(() => {
		vi.useRealTimers()
	})

	it("skips orders to it, and bids once enableFills delegates", async () => {
		const { filler, isUserAllowed, delegate } = build({ "EVM-1": true, "EVM-8453": false })
		const skipped: Array<{ orderId?: string; reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))

		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })

		await detect(filler, "EVM-8453")
		expect(skipped).toEqual([{ orderId: "0xorder", reason: "undelegated" }])
		expect(isUserAllowed).not.toHaveBeenCalled()

		// The dashboard's "Enable fills" delegates and clears the skip.
		delegate("EVM-8453")
		await filler.enableFills(8453)

		await detect(filler, "EVM-8453")
		expect(isUserAllowed).toHaveBeenCalled()
		expect(skipped).toHaveLength(1)
	})

	it("leaves the chains that delegated alone", async () => {
		const { filler, isUserAllowed } = build({ "EVM-1": true, "EVM-8453": false })
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })

		await detect(filler, "EVM-1")
		expect(isUserAllowed).toHaveBeenCalled()
	})

	it("leaves no stale skip when addChain rolls a failed delegation back", async () => {
		// The chain is not running yet, so addChain owns the delegation attempt. It
		// fails under `requireAll`, and the rollback must not leave the chain skipped.
		const { filler, isUserAllowed } = build({ "EVM-8453": false }, { chainConfigs: [1] })

		await expect(filler.addChain({ chainId: 8453 } as never)).rejects.toThrow(/EIP-7702 delegation failed/)
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)

		await detect(filler, "EVM-8453")
		expect(isUserAllowed).toHaveBeenCalled()
	})

	it("clears the skip when the chain is removed", async () => {
		const { filler } = build({ "EVM-1": true, "EVM-8453": false })
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })
		expect((filler as any).undelegatedChains.has(8453)).toBe(true)

		await filler.removeChain(8453)
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)
	})

	it("clears the skip when enableFills finds solver selection inactive", async () => {
		// Nothing to delegate on such a chain, so a mark from an earlier failure must
		// not keep skipping its orders forever.
		const { filler, isUserAllowed } = build({ "EVM-1": true, "EVM-8453": false }, { solverSelectionActive: false })
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })
		expect((filler as any).undelegatedChains.has(8453)).toBe(true)

		await filler.enableFills(8453)
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)

		await detect(filler, "EVM-8453")
		expect(isUserAllowed).toHaveBeenCalled()
	})

	it("clears a mark the chain disproves, and evaluates the order that found out", async () => {
		// Boot saw the delegation fail (a receipt timeout), but the transaction
		// mined: the wallet is delegated and the dashboard's switch cannot be clicked.
		const { filler, isUserAllowed, isDelegated, onChain } = build({ "EVM-1": true, "EVM-8453": false })
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })
		expect((filler as any).undelegatedChains.has(8453)).toBe(true)
		onChain["EVM-8453"] = true

		await detect(filler, "EVM-8453")
		expect(isDelegated).toHaveBeenCalledWith("EVM-8453")
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)
		expect(isUserAllowed).toHaveBeenCalled()

		// Cleared for good: later orders no longer read the chain.
		isDelegated.mockClear()
		await detect(filler, "EVM-8453")
		expect(isDelegated).not.toHaveBeenCalled()
	})

	it("re-reads a marked chain at most once a minute", async () => {
		vi.useFakeTimers()
		const { filler, isUserAllowed, isDelegated, onChain } = build({ "EVM-1": true, "EVM-8453": false })
		const skipped: Array<{ reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })

		await detect(filler, "EVM-8453")
		await detect(filler, "EVM-8453")
		await detect(filler, "EVM-8453")
		expect(isDelegated).toHaveBeenCalledTimes(1)
		expect(skipped).toHaveLength(3)

		// The chain becomes delegated, but the next read is not due yet.
		onChain["EVM-8453"] = true
		vi.advanceTimersByTime(59_000)
		await detect(filler, "EVM-8453")
		expect(isDelegated).toHaveBeenCalledTimes(1)
		expect(isUserAllowed).not.toHaveBeenCalled()

		vi.advanceTimersByTime(1_000)
		await detect(filler, "EVM-8453")
		expect(isDelegated).toHaveBeenCalledTimes(2)
		expect(isUserAllowed).toHaveBeenCalledTimes(1)
	})

	it("makes orders that arrive during the re-check wait for its answer", async () => {
		const { filler, isUserAllowed, isDelegated } = build({ "EVM-1": true, "EVM-8453": false }, { maxConcurrentOrders: 5 })
		const skipped: Array<{ orderId?: string; reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })

		let answer!: (delegated: boolean) => void
		isDelegated.mockImplementationOnce(() => new Promise<boolean>((resolve) => (answer = resolve)))
		for (const id of ["0xa", "0xb", "0xc"]) {
			;(filler as any).handleNewOrder({ id, user: "0xUSER", source: "EVM-1", destination: "EVM-8453" }, "0xtx")
		}
		await vi.waitFor(() => expect(isDelegated).toHaveBeenCalledTimes(1))
		expect(skipped).toEqual([])

		// The one read clears the mark, and every order that was waiting is evaluated.
		answer(true)
		await (filler as any).globalQueue.onIdle()
		expect(isDelegated).toHaveBeenCalledTimes(1)
		expect(isUserAllowed).toHaveBeenCalledTimes(3)
		expect(skipped).toEqual([])
	})

	it("skips every order that waited when the re-check finds the chain undelegated", async () => {
		const { filler, isUserAllowed, isDelegated } = build({ "EVM-1": true, "EVM-8453": false }, { maxConcurrentOrders: 5 })
		const skipped: Array<{ orderId?: string; reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })

		let answer!: (delegated: boolean) => void
		isDelegated.mockImplementationOnce(() => new Promise<boolean>((resolve) => (answer = resolve)))
		for (const id of ["0xa", "0xb"]) {
			;(filler as any).handleNewOrder({ id, user: "0xUSER", source: "EVM-1", destination: "EVM-8453" }, "0xtx")
		}
		await vi.waitFor(() => expect(isDelegated).toHaveBeenCalledTimes(1))

		answer(false)
		await (filler as any).globalQueue.onIdle()
		expect(isDelegated).toHaveBeenCalledTimes(1)
		expect(skipped.map((event) => event.reason)).toEqual(["undelegated", "undelegated"])
		expect(isUserAllowed).not.toHaveBeenCalled()
	})

	it("keeps a chain marked when the read fails", async () => {
		const { filler, isUserAllowed, isDelegated } = build({ "EVM-1": true, "EVM-8453": false })
		await (filler as any).setupSolverSelection(["EVM-1", "EVM-8453"], { requireAll: false })
		isDelegated.mockRejectedValueOnce(new Error("rpc down"))

		await detect(filler, "EVM-8453")
		expect((filler as any).undelegatedChains.has(8453)).toBe(true)
		expect(isUserAllowed).not.toHaveBeenCalled()
	})
})

describe("guardDelegation, for a chain turned out of watch-only", () => {
	it("marks a chain that was never delegated, so its orders are skipped", async () => {
		const { filler, isUserAllowed, isDelegated } = build({}, { chainConfigs: [8453] })
		const skipped: Array<{ reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))

		await filler.guardDelegation(8453)
		expect(isDelegated).toHaveBeenCalledWith("EVM-8453")
		expect((filler as any).undelegatedChains.has(8453)).toBe(true)

		// Just read, so intake does not read the chain again at once.
		await detect(filler, "EVM-8453")
		expect(skipped).toEqual([{ orderId: "0xorder", reason: "undelegated" }])
		expect(isDelegated).toHaveBeenCalledTimes(1)
		expect(isUserAllowed).not.toHaveBeenCalled()
	})

	it("leaves a chain that is delegated alone", async () => {
		const { filler, isUserAllowed, onChain } = build({}, { chainConfigs: [8453] })
		onChain["EVM-8453"] = true

		await filler.guardDelegation(8453)
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)

		await detect(filler, "EVM-8453")
		expect(isUserAllowed).toHaveBeenCalled()
	})

	it("fails, leaving the chain watch-only, when the solver selection read fails", async () => {
		// Uncached for a chain that was watch-only at boot. Assuming "active" would leave
		// the shared cache empty and every later order dropped, so the error escapes and
		// the operator can retry.
		const { filler, isDelegated, cache } = build({}, { chainConfigs: [8453], solverSelectionFails: true, cacheEmpty: true })
		filler.setWatchOnly(8453, true)

		await expect(filler.guardDelegation(8453)).rejects.toThrow(/Could not read solver selection on EVM-8453.*rpc down/)
		expect(isDelegated).not.toHaveBeenCalled()
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)
		expect(filler.getWatchOnly()[8453]).toBe(true)
		expect(cache.has("EVM-8453")).toBe(false)
	})

	it("fills the shared cache, so a chain turned out of watch-only evaluates its orders", async () => {
		const { filler, isUserAllowed, onChain, cache } = build(
			{},
			{ chainConfigs: [8453], solverSelectionActive: true, cacheEmpty: true },
		)
		onChain["EVM-8453"] = true
		const skipped: Array<{ reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))

		await filler.guardDelegation(8453)
		expect(cache.get("EVM-8453")).toBe(true)

		await detect(filler, "EVM-8453")
		expect(isUserAllowed).toHaveBeenCalled()
		expect(skipped).toEqual([])
	})

	it("records a skip, rather than dropping silently, when the shared cache has no answer", async () => {
		const { filler, isUserAllowed } = build({}, { chainConfigs: [8453], cacheEmpty: true })
		const skipped: Array<{ orderId?: string; reason?: string }> = []
		filler.monitor.on("orderSkipped", (event) => skipped.push(event))

		await detect(filler, "EVM-8453")
		expect(skipped).toEqual([{ orderId: "0xorder", reason: "Solver selection not read for the destination chain" }])
		expect(isUserAllowed).not.toHaveBeenCalled()
	})

	it("leaves a chain without solver selection alone", async () => {
		const { filler, isDelegated } = build({}, { chainConfigs: [8453], solverSelectionActive: false })

		await filler.guardDelegation(8453)
		expect(isDelegated).not.toHaveBeenCalled()
		expect((filler as any).undelegatedChains.has(8453)).toBe(false)
	})
})
