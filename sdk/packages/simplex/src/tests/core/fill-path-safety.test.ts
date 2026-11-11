import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import type { LimitOrderStore } from "@/data/types"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { limitOrderStore } from "../helpers/limit-orders"

/**
 * By the time `executeOrder` returns, a bid may already be on the orderbook, where a
 * swapper can execute it, and a fill it wins is settled against its limit order from
 * the bid store's row.
 *
 * So nothing between the submission and that write may be allowed to fail the
 * write, and nothing in the fill path may be allowed to take the process down.
 */

const COMMITMENT = "0xc0mm1tment" as HexString
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const LIMIT_ORDER = "limit-0"
/** 1,000 of an 18-decimal token: what the bids below offer from their limit orders. */
const PAYOUT = 1000n * 10n ** 18n
const MATCH = { limitOrderId: LIMIT_ORDER, payout: PAYOUT }

function build(result: Record<string, unknown>, limitOrders?: LimitOrderStore) {
	const data = new MemoryDataStore()
	const strategy = {
		name: "test",
		canFill: async () => true,
		calculateProfitability: async () => 1,
		executeOrder: async () => result,
		getOrderUsdValue: async () => ({ inputUsd: { toNumber: () => 1 } }),
	}

	const filler = new IntentFiller(
		[{ chainId: 1 } as never],
		[strategy as never],
		{ maxConcurrentOrders: 1 } as never,
		{
			loggers: undefined,
			getIntentGatewayAddress: () => "0xGATE",
			getRpcUrls: () => ["https://rpc.example"],
		} as never,
		{} as never,
		// The fill path reads the match this order was priced against off the cache.
		{ cacheService: { getMatchedLimitOrder: () => MATCH, pinOrder: () => () => {} } } as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([1]) },
		undefined,
		data.bids,
		limitOrders,
	)
	return { filler, data, strategy }
}

/** Drives one order through the chain queue and waits for it to settle. */
async function execute(filler: IntentFiller, order: Record<string, unknown>) {
	// biome-ignore lint/suspicious/noExplicitAny: exercising the private execution path
	await (filler as any).executeOrder(order, (filler as any).strategies[0], { toNumber: () => 1 }, 1, false)
	// biome-ignore lint/suspicious/noExplicitAny: draining the chain queue
	await (filler as any).chainQueues.get(1)?.onIdle()
}

const ORDER = { id: "0xorder", source: "EVM-1", destination: "EVM-1" }

describe("fill path safety", () => {
	it("persists the bid even when a listener on the emitted event throws", async () => {
		const { filler, data } = build({ success: true, commitment: COMMITMENT })

		// A host listener that explodes must not cost us the bid record — the
		// bid is already on the orderbook by this point.
		filler.monitor.on("orderExecuted", () => {
			throw new Error("consumer exploded")
		})

		await execute(filler, ORDER)

		expect(await data.bids.byCommitment(COMMITMENT)).not.toBeNull()
	})

	it("does not let an execution failure escape as an unhandled rejection", async () => {
		const { filler } = build({ success: false, error: "boom" })
		// biome-ignore lint/suspicious/noExplicitAny: forcing the strategy to throw
		;(filler as any).strategies[0].executeOrder = async () => {
			throw new Error("execution exploded")
		}

		const unhandled = vi.fn()
		process.once("unhandledRejection", unhandled)

		await execute(filler, ORDER)
		await new Promise((resolve) => setTimeout(resolve, 10))

		// An unhandled rejection here exits the process under Node's default policy,
		// stopping every other order's settlement with it.
		expect(unhandled).not.toHaveBeenCalled()
		process.off("unhandledRejection", unhandled)
	})

	it("records a bid no venue took as failed", async () => {
		const { filler, data } = build({ success: false, commitment: COMMITMENT, error: "NO_LIMIT_ORDER: no limit order" })

		await execute(filler, ORDER)

		expect(await data.bids.byCommitment(COMMITMENT)).toMatchObject({ success: false, onOrderbook: false })
		expect(await data.bids.stats()).toEqual({ total: 1, successful: 0, failed: 1 })
	})
})

describe("every bid on one order", () => {
	const SECOND_ORDER = "limit-1"
	const FIRST_BID = `0x${"b1".repeat(32)}` as HexString
	const SECOND_BID = `0x${"b2".repeat(32)}` as HexString

	/**
	 * Two bid plans on one order, and a strategy answering each submission in turn. A
	 * function in `results` runs while that bid is going out, then answers it.
	 */
	async function twoBids(
		results: Array<Record<string, unknown> | ((filler: IntentFiller) => Promise<Record<string, unknown>>)>,
	) {
		const data = new MemoryDataStore()
		const limitOrders = await limitOrderStore([
			{ id: LIMIT_ORDER, base: "USDC", quote: "CNGN", side: "BID", fillChain: "EVM-1", price: "1500", size: "3000" },
			{ id: SECOND_ORDER, base: "USDC", quote: "CNGN", side: "BID", fillChain: "EVM-1", price: "1450", size: "3000" },
		])
		const plan = (limitOrderId: string) => ({
			limitOrderId,
			payout: PAYOUT,
			fillerOutputs: [],
			fillerInputs: [],
			fundingCalls: [],
			partialFill: false,
			profit: 1,
		})
		let calls = 0
		const strategy = {
			name: "test",
			canFill: async () => true,
			calculateProfitability: async () => 1,
			executeOrder: async () => {
				const next = results[Math.min(calls++, results.length - 1)]
				return typeof next === "function" ? next(filler) : next
			},
			getOrderUsdValue: async () => ({ inputUsd: { toNumber: () => 1 } }),
		}
		const filler: IntentFiller = new IntentFiller(
			[{ chainId: 1 } as never],
			[strategy as never],
			{ maxConcurrentOrders: 1 } as never,
			{
				loggers: undefined,
				getIntentGatewayAddress: () => "0xGATE",
				getRpcUrls: () => ["https://rpc.example"],
			} as never,
			{} as never,
			{
				cacheService: {
					getMatchedLimitOrder: () => [],
					pinOrder: () => () => {},
					getBidPlans: () => [plan(LIMIT_ORDER), plan(SECOND_ORDER)],
					setFillerOutputs: () => {},
					setPartialFill: () => {},
					setFundingPrepends: () => {},
					clearFundingPrepends: () => {},
				},
			} as never,
			{ address: OUR_ADDRESS } as never,
			{ orders: stubOrderScanner([1]) },
			undefined,
			data.bids,
			limitOrders,
		)
		return { filler, data, limitOrders, sent: () => calls }
	}

	/** A rival completing the order: the scanner reports the fill and the settlement queue handles it. */
	async function rivalCompletes(filler: IntentFiller, commitment: HexString) {
		observeRivalCompletion(filler, commitment)
		await (filler as any).settlementQueue.onIdle()
	}

	/** The scanner reporting a rival's complete fill, without waiting for the settlement queue. */
	function observeRivalCompletion(filler: IntentFiller, commitment: HexString) {
		filler.monitor.emit("orderFillObserved", {
			commitment,
			filler: "0xBBBB00000000000000000000000000000000BBBB",
			chainId: 1,
			ours: false,
			complete: true,
		})
	}

	it("goes out whatever became of the bid before it", async () => {
		// Each bid is the first sequence of its own nonce key, so nothing about one
		// bid's fate changes what the next can sign: a rejected bid holds nothing up.
		const { filler, data, sent } = await twoBids([
			{ success: false, error: "rejected", commitment: COMMITMENT, bid: FIRST_BID },
			{ success: true, commitment: COMMITMENT, bid: SECOND_BID },
		])

		await execute(filler, ORDER)

		expect(sent()).toBe(2)
		expect((await data.bids.byCommitment(COMMITMENT))?.bid).toBe(SECOND_BID)
	})

	it("files each bid's row under its own identifier", async () => {
		// Two bids on one order share a commitment; the identifier, keccak256 of each
		// one's calldata, is what keeps their rows apart, each naming the limit order
		// that priced it.
		const { filler, data } = await twoBids([
			{ success: true, commitment: COMMITMENT, bid: FIRST_BID },
			{ success: true, commitment: COMMITMENT, bid: SECOND_BID },
		])

		await execute(filler, ORDER)

		const rows = await data.bids.byCommitments([COMMITMENT])
		expect(rows.map((row) => [row.bid, row.pricedBy]).sort()).toEqual([
			[FIRST_BID, { limitOrderId: LIMIT_ORDER, payout: PAYOUT.toString() }],
			[SECOND_BID, { limitOrderId: SECOND_ORDER, payout: PAYOUT.toString() }],
		])
	})

	it("places no more bids once the order completes after the first", async () => {
		// A rival completes the order while the first bid is going out. The second could
		// only revert with `Filled()`, so it is never sent.
		const order = { ...ORDER, id: COMMITMENT }
		const { filler, data, sent } = await twoBids([
			async (f) => {
				await rivalCompletes(f, COMMITMENT)
				return { success: true, commitment: COMMITMENT, bid: FIRST_BID }
			},
			{ success: true, commitment: COMMITMENT, bid: SECOND_BID },
		])

		await execute(filler, order)

		expect(sent()).toBe(1)
		const rows = await data.bids.byCommitments([COMMITMENT])
		expect(rows.map((row) => row.bid)).toEqual([FIRST_BID])
	})

	describe("a limit order that closes around a bid", () => {
		const order = { ...ORDER, id: COMMITMENT }

		it.each(["cancelled", "expired", "filled", "rejected"] as const)(
			"places no bid its %s limit order priced",
			async (status) => {
				// The plans were cached when the order was evaluated; the limit order closed
				// before the loop reached its bid.
				const { filler, data, limitOrders, sent } = await twoBids([
					{ success: true, commitment: COMMITMENT, bid: FIRST_BID },
					{ success: true, commitment: COMMITMENT, bid: SECOND_BID },
				])
				await limitOrders.setStatus(LIMIT_ORDER, status)

				await execute(filler, order)

				expect(sent()).toBe(1)
				const rows = await data.bids.byCommitments([COMMITMENT])
				expect(rows.map((row) => row.pricedBy?.limitOrderId)).toEqual([SECOND_ORDER])
			},
		)

		it("places nothing when every limit order that priced the order has closed", async () => {
			const { filler, data, limitOrders, sent } = await twoBids([
				{ success: true, commitment: COMMITMENT, bid: FIRST_BID },
			])
			await limitOrders.setStatus(LIMIT_ORDER, "cancelled")
			await limitOrders.setStatus(SECOND_ORDER, "cancelled")

			await execute(filler, order)

			expect(sent()).toBe(0)
			expect(await data.bids.byCommitments([COMMITMENT])).toEqual([])
		})

		it("places no bid when the limit order cannot be read", async () => {
			// Unconfirmed is not standing: the orderbook would serve a bid at a price that may be withdrawn.
			const { filler, limitOrders, sent } = await twoBids([
				{ success: true, commitment: COMMITMENT, bid: FIRST_BID },
			])
			limitOrders.get = async () => {
				throw new Error("database is locked")
			}

			await execute(filler, order)

			expect(sent()).toBe(0)
		})

		it("keeps a limit order's bid that is still resizing", async () => {
			const { filler, limitOrders, sent } = await twoBids([
				{ success: true, commitment: COMMITMENT, bid: FIRST_BID },
				{ success: true, commitment: COMMITMENT, bid: SECOND_BID },
			])
			await limitOrders.setStatus(LIMIT_ORDER, "resizing")

			await execute(filler, order)

			expect(sent()).toBe(2)
		})
	})

	it("stops at once when the completion is recorded while the settlement queue is busy", async () => {
		// Settlement waits behind reconcile passes and orderbook requests. The completion is
		// recorded when the fill is seen, not when its settlement runs, so the second bid,
		// which could only revert `Filled()`, is not sent while the queue is stuck.
		const order = { ...ORDER, id: COMMITMENT }
		let release!: () => void
		const stuck = new Promise<void>((resolve) => (release = resolve))
		const { filler, sent } = await twoBids([
			async (f) => {
				observeRivalCompletion(f, COMMITMENT)
				return { success: true, commitment: COMMITMENT, bid: FIRST_BID }
			},
			{ success: true, commitment: COMMITMENT, bid: SECOND_BID },
		])
		;(filler as any).settlementQueue.add(() => stuck)

		await execute(filler, order)

		expect(sent()).toBe(1)
		expect((filler as any).hasCompleted(COMMITMENT)).toBe(true)
		release()
		await (filler as any).settlementQueue.onIdle()
	})

	it("records only a complete fill as completing the order", async () => {
		const { filler } = await twoBids([])
		const monitor = filler.monitor
		monitor.emit("orderFillObserved", { commitment: COMMITMENT, filler: "0xBBBB", chainId: 1, ours: false, complete: false })
		monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 1,
			outputs: [],
			inputs: [],
			complete: false,
		})
		expect((filler as any).hasCompleted(COMMITMENT)).toBe(false)

		monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 1,
			outputs: [],
			inputs: [],
			complete: true,
		})
		expect((filler as any).hasCompleted(COMMITMENT)).toBe(true)
		await (filler as any).settlementQueue.onIdle()
	})

	it("remembers only the most recent completed orders", async () => {
		const { filler } = await twoBids([])
		// Every complete fill on a configured chain is marked, a rival's included, so the cap
		// is well above what a bid loop's lifetime can add.
		for (let i = 0; i < 20_001; i++) (filler as any).markCompleted(`0x${i.toString(16).padStart(64, "0")}`)

		expect((filler as any).completedOrders.size).toBe(20_000)
		expect((filler as any).hasCompleted(`0x${"0".repeat(64)}`)).toBe(false)
		expect((filler as any).hasCompleted(`0x${(20_000).toString(16).padStart(64, "0")}`)).toBe(true)
	})
})
