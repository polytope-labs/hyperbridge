import { afterEach, describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { budgetFor, type LimitOrderBudget } from "@/orderbook/amounts"
import { CacheService, type BidPlan } from "@/services/CacheService"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { limitOrderStore } from "../helpers/limit-orders"

/**
 * The budget a bid settles against has to be the one of the limit order that priced
 * it, at the moment the bid is built: the filler sends several bids for one order in
 * turn, and each reads what it signs back out of the cache under the order's id.
 */

const COMMITMENT = "0xc0mm1tment" as HexString
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const CNGN = "0xcccc00000000000000000000000000000000cccc" as HexString
const FIRST = "3f2b8c1e-9d4a-4f6b-8a7c-5e1d2c3b4a59"
const SECOND = "b7a1d2c4-0e5f-4a3b-9c8d-1f2e3d4c5b6a"
const ORDER = { id: "0xorder", source: "EVM-1", destination: "EVM-1" }
const PAYOUT = 1000n * 10n ** 18n
const OUTPUTS = [{ token: `0x${"00".repeat(12)}${CNGN.slice(2)}` as HexString, amount: PAYOUT }]

const budgetOf = (id: string, size: bigint) => budgetFor({ id, size: size.toString() }, CNGN, 18)

function plan(limitOrderId: string, budget?: LimitOrderBudget): BidPlan {
	return {
		limitOrderId,
		leg: 0,
		payout: PAYOUT,
		fillerOutputs: OUTPUTS,
		fillerInputs: [],
		fundingCalls: [],
		partialFill: false,
		profit: 1,
		budget,
	}
}

/** A filler over the real cache, whose strategy notes the budget each bid would be built with. */
async function build() {
	const cache = new CacheService()
	const seen: (LimitOrderBudget | null)[] = []
	const strategy = {
		name: "test",
		canFill: async () => true,
		calculateProfitability: async () => 1,
		executeOrder: async (order: { id: string }) => {
			seen.push(cache.getBidBudget(order.id))
			return { success: true, commitment: COMMITMENT, txHash: "0xtx", bid: `0x${seen.length}` }
		},
		getOrderUsdValue: async () => ({ inputUsd: { toNumber: () => 1 } }),
	}
	const limitOrders = await limitOrderStore([
		{ id: FIRST, base: "USDC", quote: "CNGN", side: "BID", fillChain: "EVM-1", price: "1500", size: "3000" },
		{ id: SECOND, base: "USDC", quote: "CNGN", side: "BID", fillChain: "EVM-1", price: "1450", size: "5000" },
	])
	const filler = new IntentFiller(
		[{ chainId: 1 } as never],
		[strategy as never],
		{ maxConcurrentOrders: 1 } as never,
		{
			loggers: undefined,
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getIntentGatewayAddress: () => "0xGATE",
			getRpcUrls: () => ["https://rpc.example"],
		} as never,
		{} as never,
		{ cacheService: cache } as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([1]) },
		undefined,
		new MemoryDataStore().bids,
		limitOrders,
	)
	const executed: { success: boolean }[] = []
	filler.monitor.on("orderExecuted", (event: { success: boolean }) => executed.push(event))
	const execute = async () => {
		await (filler as any).executeOrder(ORDER, (filler as any).strategies[0], false, { toNumber: () => 1 }, 1)
		await (filler as any).chainQueues.get(1)?.onIdle()
	}
	const reserved = async (id: string) => (await limitOrders.get(id))!.reserved
	return { cache, seen, execute, executed, reserved }
}

describe("a bid's limit order budget", () => {
	afterEach(() => vi.restoreAllMocks())

	it("is there for the bid builder when a limit order priced the bid", async () => {
		const { cache, seen, execute, executed, reserved } = await build()
		const budget = budgetOf(FIRST, 3000n * 10n ** 18n)
		cache.setBidPlans(ORDER.id, [plan(FIRST, budget)])

		await execute()

		expect(seen).toEqual([budget])
		expect(seen[0]?.cap).toBe(3000n * 10n ** 18n)
		// The bid went out and stands: reported as executed, its hold still in place.
		expect(executed).toEqual([expect.objectContaining({ success: true })])
		expect(await reserved(FIRST)).toBe(PAYOUT.toString())
	})

	it("is each bid's own when several limit orders priced one order", async () => {
		const { cache, seen, execute, executed } = await build()
		const first = budgetOf(FIRST, 3000n * 10n ** 18n)
		const second = budgetOf(SECOND, 5000n * 10n ** 18n)
		cache.setBidPlans(ORDER.id, [plan(SECOND, second), plan(FIRST, first)])

		await execute()

		expect(seen).toEqual([second, first])
		expect(executed).toHaveLength(2)
	})

	it("is absent for a bid no limit order priced", async () => {
		const { cache, seen, execute, executed } = await build()
		cache.setFillerOutputs(ORDER.id, OUTPUTS)

		await execute()

		expect(seen).toEqual([null])
		expect(executed).toEqual([expect.objectContaining({ success: true })])
	})

	it("does not outlive its bid: the next bid's outputs replace it", async () => {
		const { cache, seen, execute, executed } = await build()
		const budget = budgetOf(FIRST, 3000n * 10n ** 18n)
		cache.setBidPlans(ORDER.id, [plan(FIRST, budget), plan(SECOND)])

		await execute()

		expect(seen).toEqual([budget, null])
		expect(executed).toHaveLength(2)
	})

	it("expires with the outputs it was cached beside", async () => {
		const { cache, execute, executed } = await build()
		const budget = budgetOf(FIRST, 3000n * 10n ** 18n)
		cache.setBidPlans(ORDER.id, [plan(FIRST, budget)])
		await execute()
		expect(executed).toEqual([expect.objectContaining({ success: true })])
		expect(cache.getBidBudget(ORDER.id)).toEqual(budget)

		const later = Date.now() + 61_000
		vi.spyOn(Date, "now").mockReturnValue(later)

		expect(cache.getFillerOutputs(ORDER.id)).toBeNull()
		expect(cache.getBidBudget(ORDER.id)).toBeNull()
	})
})
