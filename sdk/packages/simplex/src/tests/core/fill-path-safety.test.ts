import { describe, expect, it, vi } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

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

function build(result: Record<string, unknown>) {
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
		{} as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner([1]) },
		undefined,
		data.bids,
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

	it("leaves an outright failed bid out of the sweep", async () => {
		const { filler, data } = build({ success: false, commitment: COMMITMENT, error: "rejected" })

		await execute(filler, ORDER)

		expect(await data.bids.unretractedReclaimable()).toEqual([])
		expect((await data.bids.stats()).pendingRetraction).toBe(0)
	})
})
