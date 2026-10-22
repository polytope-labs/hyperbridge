import { describe, expect, it, vi } from "vitest"
import { RebalancingService, type PlannedTransfer } from "@/services/RebalancingService"

function deferred<T>() {
	let resolve!: (value: T) => void
	let reject!: (error: unknown) => void
	const promise = new Promise<T>((res, rej) => {
		resolve = res
		reject = rej
	})
	return { promise, resolve, reject }
}

const flush = () => new Promise((resolve) => setImmediate(resolve))

/**
 * The service without its constructor, which builds live rebalancers from a signer and
 * config. Only `rebalance` (stubbed) and the logger are reached by the code under test.
 */
function makeService(rebalance: ReturnType<typeof vi.fn>): RebalancingService {
	const service = Object.create(RebalancingService.prototype) as RebalancingService
	const logger = { info: vi.fn(), error: vi.fn(), warn: vi.fn(), debug: vi.fn() }
	Object.assign(service, { logger, rebalance })
	return service
}

const leg = (sourceChain: string, destChain: string, amount: string, coin: "USDC" | "USDT" = "USDC"): PlannedTransfer => ({
	sourceChain,
	destChain,
	coin,
	amount,
})

/** A `rebalance` stub whose calls stay pending until the case resolves them, by amount. */
function pendingRebalance() {
	const pending = new Map<string, ReturnType<typeof deferred<unknown>>>()
	const rebalance = vi.fn(({ amount }: { amount: string }) => {
		const d = deferred<unknown>()
		pending.set(amount, d)
		return d.promise
	})
	const started = () => rebalance.mock.calls.map(([o]) => o.amount)
	return { pending, rebalance, started }
}

describe("executePortfolioRebalancing", () => {
	it("runs legs from one source chain one after another and other sources in parallel", async () => {
		const { pending, rebalance, started } = pendingRebalance()
		const service = makeService(rebalance)
		// Two CCTP legs out of Base, and a Binance leg out of BSC that sends only from BSC.
		const transfers = [
			leg("EVM-8453", "EVM-1", "1"),
			leg("EVM-8453", "EVM-42161", "2"),
			leg("EVM-56", "EVM-137", "3"),
		]

		const run = service.executePortfolioRebalancing(transfers)
		await flush()
		// The second Base leg waits for the first; the BSC leg does not.
		expect(started()).toEqual(["1", "3"])

		pending.get("3")!.resolve({ id: 3 })
		await flush()
		expect(rebalance).toHaveBeenCalledTimes(2)

		pending.get("1")!.resolve({ id: 1 })
		await flush()
		expect(started()).toEqual(["1", "3", "2"])

		pending.get("2")!.resolve({ id: 2 })
		const result = await run
		expect(result.success).toBe(true)
		expect(result.executedTransfers.map((t) => t.transfer)).toEqual(transfers)
		expect(result.executedTransfers.map((t) => t.result)).toEqual([{ id: 1 }, { id: 2 }, { id: 3 }])
	})

	it("runs CCTP legs into one destination one after another, since each mints there from our EOA", async () => {
		const { pending, rebalance, started } = pendingRebalance()
		const service = makeService(rebalance)
		const transfers = [
			leg("EVM-8453", "EVM-1", "1"),
			leg("EVM-42161", "EVM-1", "2"),
			// USDT0 is delivered by LayerZero: it sends nothing from us on Ethereum.
			leg("EVM-137", "EVM-1", "3", "USDT"),
		]

		const run = service.executePortfolioRebalancing(transfers)
		await flush()
		expect(started()).toEqual(["1", "3"])

		pending.get("1")!.resolve({ id: 1 })
		await flush()
		expect(started()).toEqual(["1", "3", "2"])

		pending.get("2")!.resolve({ id: 2 })
		pending.get("3")!.resolve({ id: 3 })
		expect((await run).success).toBe(true)
	})

	it("runs opposite CCTP legs between two chains one after another", async () => {
		const { pending, rebalance, started } = pendingRebalance()
		const service = makeService(rebalance)

		const run = service.executePortfolioRebalancing([leg("EVM-8453", "EVM-1", "1"), leg("EVM-1", "EVM-8453", "2")])
		await flush()
		expect(started()).toEqual(["1"])

		pending.get("1")!.resolve({ id: 1 })
		await flush()
		expect(started()).toEqual(["1", "2"])
		pending.get("2")!.resolve({ id: 2 })
		expect((await run).success).toBe(true)
	})

	it("still runs a later same-source leg after an earlier one fails, and reports the failure", async () => {
		const rebalance = vi.fn(async ({ amount }: { amount: string }) => {
			if (amount === "1") throw new Error("bridge down")
			return { ok: amount }
		})
		const service = makeService(rebalance)
		const transfers = [leg("EVM-8453", "EVM-1", "1"), leg("EVM-8453", "EVM-42161", "2")]

		const result = await service.executePortfolioRebalancing(transfers)
		expect(rebalance).toHaveBeenCalledTimes(2)
		expect(result.success).toBe(false)
		expect(result.errors).toHaveLength(1)
		expect(result.errors![0]).toMatch(/EVM-8453 -> EVM-1 .*bridge down/)
		expect(result.executedTransfers).toEqual([{ transfer: transfers[1], result: { ok: "2" } }])
	})
})
