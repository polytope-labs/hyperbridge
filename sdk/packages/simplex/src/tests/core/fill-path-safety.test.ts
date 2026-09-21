import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import type { LimitOrderStore } from "@/data/types"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { limitOrderStore } from "../helpers/limit-orders"

/**
 * The fill path holds money. By the time `executeOrder` returns, a bid may
 * already be on Hyperbridge reserving a deposit whose only route back is a
 * retraction — and the sweep can only retract bids it can find in the store.
 *
 * So nothing between the submission and that write may be allowed to fail the
 * write, and nothing in the fill path may be allowed to take the process down
 * (which would stop the sweep for every other outstanding bid too).
 */

const COMMITMENT = "0xc0mm1tment" as HexString
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const LIMIT_ORDER = "limit-0"
/** 1,000 of an 18-decimal token: what the bid below holds against the limit order. */
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
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			getIntentGatewayAddress: () => "0xGATE",
			getRpcUrls: () => ["https://rpc.example"],
		} as never,
		{} as never,
		// The fill path reads the match this order was priced against off the cache.
		{ cacheService: { getMatchedLimitOrder: () => MATCH } } as never,
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
		const { filler, data } = build({ success: true, commitment: COMMITMENT, txHash: "0xtx" })

		// A host listener that explodes must not cost us the bid record — the
		// deposit is already locked by this point.
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
		// stopping the retraction sweep for every other outstanding bid.
		expect(unhandled).not.toHaveBeenCalled()
		process.off("unhandledRejection", unhandled)
	})

	it("stores a pooled bid as reclaimable so the sweep can find it", async () => {
		const { filler, data } = build({
			success: false,
			pending: true,
			commitment: COMMITMENT,
			txHash: "0xpooled",
			error: "Transaction timed out while in the transaction pool",
		})

		await execute(filler, ORDER)

		const bid = await data.bids.byCommitment(COMMITMENT)
		expect(bid).not.toBeNull()
		expect(bid!.success).toBe(false)
		expect(bid!.pending).toBe(true)
		// The extrinsic may still land and reserve a deposit, so it must be sweepable
		// and must show up in the gauge operators are told to watch.
		expect((await data.bids.unretractedReclaimable()).map((b) => b.commitment)).toEqual([COMMITMENT])
		expect((await data.bids.stats()).pendingRetraction).toBe(1)
	})

	it("gives a limit order's hold back exactly once when the fill path throws late", async () => {
		// The hold belongs to the bid row from the moment it is written, so a throw
		// after that has to claim it rather than release it: releasing here and
		// again on the retraction would free capacity a second bid is holding.
		const limitOrders = await limitOrderStore([
			{ id: LIMIT_ORDER, base: "USDC", quote: "CNGN", side: "BID", fillChain: "EVM-1", price: "1500", size: "3000" },
		])
		// A second bid, still open, holding the same amount against the same order.
		expect(await limitOrders.reserve(LIMIT_ORDER, PAYOUT.toString())).toBe(true)

		const { filler } = build({ success: true, commitment: COMMITMENT, txHash: "0xtx" }, limitOrders)
		filler.monitor.on("orderFilled", () => {
			throw new Error("consumer exploded")
		})

		await execute(filler, ORDER)
		expect((await limitOrders.get(LIMIT_ORDER))!.reserved).toBe(PAYOUT.toString())

		// The retraction that follows finds the hold already claimed and takes nothing.
		// biome-ignore lint/suspicious/noExplicitAny: the settlement path is private
		await (filler as any).releaseReservation(COMMITMENT)
		expect((await limitOrders.get(LIMIT_ORDER))!.reserved).toBe(PAYOUT.toString())
	})

	it("leaves an outright failed bid out of the sweep", async () => {
		const { filler, data } = build({ success: false, commitment: COMMITMENT, error: "rejected" })

		await execute(filler, ORDER)

		expect(await data.bids.unretractedReclaimable()).toEqual([])
		expect((await data.bids.stats()).pendingRetraction).toBe(0)
	})
})

describe("every bid on one order", () => {
	const SECOND_ORDER = "limit-1"
	const FIRST_BID = `0x${"b1".repeat(32)}` as HexString
	const SECOND_BID = `0x${"b2".repeat(32)}` as HexString

	/** Two bid plans on one order, and a strategy answering each submission in turn. */
	async function twoBids(results: Record<string, unknown>[]) {
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
			executeOrder: async () => results[Math.min(calls++, results.length - 1)],
			getOrderUsdValue: async () => ({ inputUsd: { toNumber: () => 1 } }),
		}
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
			{
				cacheService: {
					getMatchedLimitOrder: () => [],
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
		return { filler, data, sent: () => calls }
	}

	it("goes out whatever became of the bid before it", async () => {
		// Each bid is the first sequence of its own nonce key, so nothing about one
		// bid's fate changes what the next can sign: a rejected bid holds nothing up.
		const { filler, data, sent } = await twoBids([
			{ success: false, error: "rejected", commitment: COMMITMENT, bid: FIRST_BID },
			{ success: true, commitment: COMMITMENT, txHash: "0xtx", bid: SECOND_BID },
		])

		await execute(filler, ORDER)

		expect(sent()).toBe(2)
		expect((await data.bids.byCommitment(COMMITMENT))?.bid).toBe(SECOND_BID)
	})

	it("files each bid's row under the identifier Hyperbridge knows it by", async () => {
		// Two bids on one order share a commitment; the identifier, keccak256 of each
		// one's calldata, is what keeps their rows and their holds apart.
		const { filler, data } = await twoBids([
			{ success: true, commitment: COMMITMENT, txHash: "0xtx", bid: FIRST_BID },
			{ success: true, commitment: COMMITMENT, txHash: "0xtx2", bid: SECOND_BID },
		])

		await execute(filler, ORDER)

		const rows = await data.bids.byCommitments([COMMITMENT])
		expect(rows.map((row) => row.bid).sort()).toEqual([FIRST_BID, SECOND_BID])
	})
})
