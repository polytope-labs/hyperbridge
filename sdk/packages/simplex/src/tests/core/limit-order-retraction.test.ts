import { EventEmitter } from "node:events"
import { describe, expect, it, vi } from "vitest"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
import type { SubmitOrderResult } from "@/orderbook/types"
import type { FillerRuntime } from "@/core/boot"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { LoggerContext } from "@/services/Logger"
import { Simplex } from "@/simplex"
import { CREATE_REQUEST as REQUEST, fakeClient, limitOrderService as makeService } from "../helpers/limit-orders"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * A limit order the operator cancels, or that outlives its `expiresAt`, comes off the
 * orderbook, but bids it already priced still pay out at its price until they are
 * retracted. Each of them is retracted by its own row, so a bid another limit order
 * priced on the same swap order keeps standing.
 */

const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const FIRST = "0x1111111111111111111111111111111111111111111111111111111111111111"
const SECOND = "0x2222222222222222222222222222222222222222222222222222222222222222"
const OURS_ON_FIRST = `0x${"a1".repeat(32)}`
const OURS_ON_SECOND = `0x${"a2".repeat(32)}`
const ANOTHER_ORDERS = `0x${"a3".repeat(32)}`

async function build(results: SubmitOrderResult[] = []) {
	const data = new MemoryDataStore()
	const client = fakeClient(results)
	const { service, store } = makeService(client, data.limitOrders)
	const created = await service.create({ ...REQUEST, ttlSecs: 3600 })
	const id = created.order.id

	const priced = (limitOrderId: string) => ({ limitOrderId, payout: "1" })
	await data.bids.store({ commitment: FIRST, bid: OURS_ON_FIRST, success: true, pricedBy: priced(id) })
	await data.bids.store({ commitment: SECOND, bid: OURS_ON_SECOND, success: true, pricedBy: priced(id) })
	await data.bids.store({ commitment: SECOND, bid: ANOTHER_ORDERS, success: true, pricedBy: priced("another") })

	const filler = new IntentFiller(
		[],
		[],
		{},
		{
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
		} as never,
		{} as never,
		{} as never,
		{ address: OUR_ADDRESS } as never,
		{ orders: stubOrderScanner() },
		undefined,
		data.bids,
		store,
	)
	const retractBid = vi.fn(
		async (_commitment?: string, _bid?: string): Promise<BidSubmissionResult> => ({
			success: true,
			extrinsicHash: "0x01" as HexString,
		}),
	)
	;(filler as any).hyperbridge = Promise.resolve({ retractBid })

	// Only what the constructor and the limit-order controller touch.
	const runtime = {
		loggers: new LoggerContext({}),
		intentFiller: filler,
		activity: new EventEmitter(),
		limitOrders: service,
		orderScanner: stubOrderScanner(),
	} as unknown as FillerRuntime
	const simplex = new (Simplex as any)(runtime, {}) as Simplex

	const drained = async () => {
		// The expiry listener does not await its retraction; let it queue first.
		await new Promise((resolve) => setTimeout(resolve, 0))
		await (filler as any).retractionQueue.onIdle()
	}
	const retracted = () => retractBid.mock.calls.map((call) => (call as unknown[]).slice(0, 2)).sort()
	return { simplex, service, store, data, id, results, retractBid, drained, retracted }
}

describe("a limit order taken off the book", () => {
	it("retracts exactly its own bids when the operator cancels it", async () => {
		const ctx = await build()

		await ctx.simplex.limitOrders.cancel(ctx.id)
		await ctx.drained()

		expect(ctx.retracted()).toEqual([
			[FIRST, OURS_ON_FIRST],
			[SECOND, OURS_ON_SECOND],
		])
		const standing = (await ctx.data.bids.byCommitments([SECOND])).filter((row) => !row.retracted)
		expect(standing.map((row) => row.bid)).toEqual([ANOTHER_ORDERS])
		// Nothing is flagged for the sweep: every selected bid closed.
		expect((await ctx.data.bids.byCommitments([FIRST, SECOND])).some((row) => row.dead)).toBe(false)
	})

	it("cancels even when retracting its bids fails", async () => {
		const ctx = await build()
		ctx.data.bids.byLimitOrder = async () => {
			throw new Error("store away")
		}

		await expect(ctx.simplex.limitOrders.cancel(ctx.id)).resolves.toMatchObject({ order: { status: "cancelled" } })
		expect(ctx.retractBid).not.toHaveBeenCalled()
	})

	it("flags only the selected bid a retraction could not close, so the sweep retries just that one", async () => {
		const ctx = await build()
		ctx.retractBid.mockImplementation(async (_commitment, bid) =>
			bid === OURS_ON_FIRST
				? { success: false, error: "Transaction failed after 3 attempts" }
				: { success: true, extrinsicHash: "0x01" as HexString },
		)

		await ctx.simplex.limitOrders.cancel(ctx.id)
		await ctx.drained()

		const rows = await ctx.data.bids.byCommitments([FIRST, SECOND])
		expect(rows.filter((row) => row.dead).map((row) => row.bid)).toEqual([OURS_ON_FIRST])
		expect((await ctx.data.bids.expiredUnretracted(60 * 60 * 1000)).map((row) => row.bid)).toEqual([OURS_ON_FIRST])
		// The other limit order's bid on the same swap order is left alone.
		expect(rows.find((row) => row.bid === ANOTHER_ORDERS)).toMatchObject({ retracted: false, dead: false })
	})

	it("retracts exactly its own bids when it expires", async () => {
		const ctx = await build()

		await ctx.service.expireStale(new Date(Date.now() + 2 * 3600 * 1000))
		await ctx.drained()

		expect(ctx.retracted()).toEqual([
			[FIRST, OURS_ON_FIRST],
			[SECOND, OURS_ON_SECOND],
		])
		const standing = (await ctx.data.bids.byCommitments([SECOND])).filter((row) => !row.retracted)
		expect(standing.map((row) => row.bid)).toEqual([ANOTHER_ORDERS])
	})

	it("retracts its bids when the tally reconciliation closes it, which never passes through settlement", async () => {
		const ctx = await build()
		// The chain has counted the order's whole budget: nothing is left to work down to.
		;(ctx.service as any).contractService.limitOrderSpent = async () => 10n ** 40n

		expect(await ctx.service.reconcileTallies()).toBe(1)
		await ctx.drained()

		expect((await ctx.store.get(ctx.id))!.status).toBe("filled")
		expect(ctx.retracted()).toEqual([
			[FIRST, OURS_ON_FIRST],
			[SECOND, OURS_ON_SECOND],
		])
		const standing = (await ctx.data.bids.byCommitments([SECOND])).filter((row) => !row.retracted)
		expect(standing.map((row) => row.bid)).toEqual([ANOTHER_ORDERS])
	})

	it("retracts its bids when the orderbook refuses its posting", async () => {
		const ctx = await build()
		// `listen` holds one listener and `Simplex` already has it, so watch what is reported.
		const report = vi.spyOn(ctx.service as any, "report")
		ctx.results.push({ kind: "rejected", code: "BAD_SIGNATURE", message: "bad" })

		await ctx.service.repost(await ctx.store.get(ctx.id))
		await ctx.drained()

		expect((await ctx.store.get(ctx.id))!.status).toBe("rejected")
		expect(report.mock.calls.map((call) => (call[0] as { kind: string }).kind)).toEqual(["rejected"])
		expect(ctx.retracted()).toEqual([
			[FIRST, OURS_ON_FIRST],
			[SECOND, OURS_ON_SECOND],
		])
	})
})
