import { describe, expect, it, vi } from "vitest"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
import { decodeAddress } from "@polkadot/util-crypto"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import type { LimitOrderStore } from "@/data/types"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { limitOrderStore } from "../helpers/limit-orders"

/**
 * What a bid holds against the limit order that priced it, from the moment it is
 * sent to the moment it settles.
 *
 * The hold is what stops two chains bidding at once from between them promising
 * more output than the order has. It has to be taken before the bid goes out and
 * given back exactly once, by whichever of the two settlement routes gets there
 * first: a losing bid is retracted, and a winning one is also retracted
 * eventually, by the stale sweep, after the fill has already drawn the order
 * down.
 */

const COMMITMENT = "0x4380111111111111111111111111111111111111111111111111111111114818" as HexString
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
/** The filler's own Hyperbridge account (Alice), as the pallet's storage reports it. */
const OUR_SUBSTRATE_ADDRESS = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
const LIMIT_ORDER = "limit-0"
/** The identifier the bid was placed under, which retracting it names. */
const OUR_BID = `0x${"b1".repeat(32)}` as HexString
/** A second resting order, for the bid that draws on more than one. */
const SECOND_ORDER = "limit-1"
const CNGN = "0xCCCC00000000000000000000000000000000CCCC" as HexString
const OTHER = "0xDDDD00000000000000000000000000000000DDDD" as HexString
/** 1,000 of an 18-decimal token, the payout each test's bid holds. */
const PAYOUT = (1000n * 10n ** 18n).toString()

async function build(options: { retract?: BidSubmissionResult } = {}) {
	const data = new MemoryDataStore()
	const limitOrders: LimitOrderStore = await limitOrderStore([
		{
			id: SECOND_ORDER,
			base: "USDC",
			quote: "CNGN",
			side: "BID",
			fillChain: "EVM-8453",
			price: "1450",
			size: "3000",
		},
		{
			id: LIMIT_ORDER,
			base: "USDC",
			quote: "CNGN",
			side: "BID",
			fillChain: "EVM-8453",
			price: "1500",
			size: "3000",
		},
	])

	const retractBid = vi.fn(async (): Promise<BidSubmissionResult> => options.retract ?? { success: true })
	const filler = new IntentFiller(
		[],
		[],
		{},
		{ getHyperbridgeWsUrl: () => undefined, getSubstratePrivateKey: () => undefined } as any,
		{} as any,
		{} as any,
		{ address: OUR_ADDRESS } as any,
		{ orders: stubOrderScanner() },
		undefined,
		data.bids,
		limitOrders,
	)
	// Retraction reads back the sequences our account holds on the commitment; this
	// one holds a single bid, at the first.
	;(filler as any).hyperbridge = Promise.resolve({
		retractBid,
		getKeyPair: () => ({ publicKey: decodeAddress(OUR_SUBSTRATE_ADDRESS) }),
		getBidStorageEntries: async (commitment: HexString) => [
			{ commitment, filler: OUR_SUBSTRATE_ADDRESS, sequence: 0n, deposit: 1n },
		],
	})

	const reserved = async () => (await limitOrders.get(LIMIT_ORDER))!.reserved

	return { filler, bids: data.bids, limitOrders, reserved, retractBid }
}

/** A bid that went out holding `PAYOUT` against the limit order. */
async function placeBid(ctx: Awaited<ReturnType<typeof build>>) {
	expect(await ctx.limitOrders.reserve(LIMIT_ORDER, PAYOUT)).toBe(true)
	await ctx.bids.store({
		commitment: COMMITMENT,
		bid: OUR_BID,
		success: true,
		reservations: [{ limitOrderId: LIMIT_ORDER, amount: PAYOUT }],
	})
}

describe("a bid's hold on its limit order", () => {
	it("is given back when the bid is retracted", async () => {
		const ctx = await build()
		await placeBid(ctx)
		expect(await ctx.reserved()).toBe(PAYOUT)

		await (ctx.filler as any).enqueueRetraction(COMMITMENT)
		await (ctx.filler as any).retractionQueue.onIdle()

		expect(await ctx.reserved()).toBe("0")
		expect((await ctx.bids.byCommitment(COMMITMENT))?.retracted).toBe(true)
	})

	it("is given back when the pallet says there was no bid to retract", async () => {
		const ctx = await build({ retract: { success: false, error: "BidNotFound" } })
		await placeBid(ctx)

		await (ctx.filler as any).enqueueRetraction(COMMITMENT)
		await (ctx.filler as any).retractionQueue.onIdle()

		expect(await ctx.reserved()).toBe("0")
	})

	it("stays held while the retraction is only pooled", async () => {
		// A pooled extrinsic may still land and reclaim the deposit, so the bid is
		// not settled yet and neither is what it holds.
		const ctx = await build({ retract: { success: false, pending: true, error: "1014" } })
		await placeBid(ctx)

		await (ctx.filler as any).enqueueRetraction(COMMITMENT)
		await (ctx.filler as any).retractionQueue.onIdle()

		expect(await ctx.reserved()).toBe(PAYOUT)
	})

	it("is given back once, however many times the bid settles", async () => {
		// The sweep retracts a bid that already settled, and a won bid is retracted
		// after its fill drew the order down. Neither may give the hold back twice.
		const ctx = await build()
		await placeBid(ctx)
		expect(await ctx.limitOrders.reserve(LIMIT_ORDER, PAYOUT)).toBe(true) // a second bid, still open

		await (ctx.filler as any).releaseReservation(COMMITMENT)
		await (ctx.filler as any).releaseReservation(COMMITMENT)
		await (ctx.filler as any).releaseReservation(COMMITMENT)

		expect(await ctx.reserved()).toBe(PAYOUT)
	})

	it("is nothing to give back for a bid that never held one", async () => {
		const ctx = await build()
		await ctx.bids.store({ commitment: COMMITMENT, success: true })

		await (ctx.filler as any).releaseReservation(COMMITMENT)

		expect(await ctx.reserved()).toBe("0")
	})
})

describe("a fill's draw-down", () => {
	/** The filler's fill path, with the registry and decimals a draw-down needs. */
	async function onFill(outputs: Array<{ token: HexString; amount: bigint }>) {
		const ctx = await build()
		await placeBid(ctx)
		const resize = vi.fn(async () => null)
		;(ctx.filler as any).assetRegistry = { getAddress: () => CNGN }
		;(ctx.filler as any).contractService = { getTokenDecimals: async () => 18 }
		;(ctx.filler as any).limitOrderService = { resize }

		await (ctx.filler as any).settleFilledLimitOrder(COMMITMENT, 8453, outputs)
		return { ...ctx, resize }
	}

	it("gives the hold back and works the order down by what went out", async () => {
		const ctx = await onFill([{ token: CNGN, amount: 400n * 10n ** 18n }])

		expect(await ctx.reserved()).toBe("0")
		expect(ctx.resize).toHaveBeenCalledWith(expect.objectContaining({ id: LIMIT_ORDER }), 400n * 10n ** 18n)
	})

	it("sums every output of the token the order pays", async () => {
		const ctx = await onFill([
			{ token: CNGN, amount: 100n * 10n ** 18n },
			{ token: OTHER, amount: 999n * 10n ** 18n },
			{ token: CNGN, amount: 250n * 10n ** 18n },
		])

		expect(ctx.resize).toHaveBeenCalledWith(expect.objectContaining({ id: LIMIT_ORDER }), 350n * 10n ** 18n)
	})

	it("leaves the order alone when the fill names none of what it pays", async () => {
		// Better an order that looks unchanged until reconciliation notices than
		// one drawn down by a guess.
		const ctx = await onFill([{ token: OTHER, amount: 999n * 10n ** 18n }])

		expect(ctx.resize).not.toHaveBeenCalled()
		// The hold still goes back: the bid is settled either way.
		expect(await ctx.reserved()).toBe("0")
	})

	it("does nothing for a bid that held nothing", async () => {
		const ctx = await build()
		await ctx.bids.store({ commitment: COMMITMENT, success: true })
		const resize = vi.fn()
		// biome-ignore lint/suspicious/noExplicitAny: narrow stub for this path
		;(ctx.filler as any).limitOrderService = { resize }

		await (ctx.filler as any).settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 1n }])
		expect(resize).not.toHaveBeenCalled()
	})
})

describe("a bid drawing on several limit orders", () => {
	it("gives every hold back when the bid is retracted", async () => {
		const ctx = await build()
		expect(await ctx.limitOrders.reserve(LIMIT_ORDER, PAYOUT)).toBe(true)
		await ctx.bids.store({
			commitment: COMMITMENT,
			success: true,
			reservations: [
				{ limitOrderId: LIMIT_ORDER, amount: PAYOUT },
				{ limitOrderId: LIMIT_ORDER, amount: PAYOUT },
			],
		})
		expect(await ctx.limitOrders.reserve(LIMIT_ORDER, PAYOUT)).toBe(true)

		// biome-ignore lint/suspicious/noExplicitAny: the settlement path is private
		await (ctx.filler as any).releaseReservation(COMMITMENT)
		expect(await ctx.reserved()).toBe("0")
	})

	it("works down the bid that filled and gives the other bids their holds back", async () => {
		// Two bids on one incoming order, one per limit order, each holding its own
		// payout. One of them fills; the other can only revert with `Filled()`, so
		// its hold comes straight back rather than being drawn down.
		const ctx = await build()
		await ctx.limitOrders.reserve(LIMIT_ORDER, PAYOUT)
		await ctx.bids.store({
			commitment: COMMITMENT,
			sequence: 0,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount: (400n * 10n ** 18n).toString() }],
		})
		await ctx.bids.store({
			commitment: COMMITMENT,
			sequence: 1,
			success: true,
			reservations: [{ limitOrderId: SECOND_ORDER, amount: (600n * 10n ** 18n).toString() }],
		})
		const settled: Array<[string, bigint]> = []
		// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for this path
		;(ctx.filler as any).assetRegistry = { getAddress: () => CNGN }
		// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for this path
		;(ctx.filler as any).contractService = { getTokenDecimals: async () => 18 }
		// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for this path
		;(ctx.filler as any).limitOrderService = {
			resize: async (order: { id: string }, amount: bigint) => {
				settled.push([order.id, amount])
				return null
			},
		}

		// 600 delivered, which is the second bid's own payout exactly: that bid is the
		// one that executed, so its limit order is the one worked down.
		// biome-ignore lint/suspicious/noExplicitAny: the settlement path is private
		await (ctx.filler as any).settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 600n * 10n ** 18n }])

		expect(settled).toEqual([[SECOND_ORDER, 600n * 10n ** 18n]])
	})

	it("claims the holds of every bid on the order, so none is left behind", async () => {
		const ctx = await build()
		await ctx.bids.store({
			commitment: COMMITMENT,
			sequence: 0,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount: PAYOUT }],
		})
		await ctx.bids.store({
			commitment: COMMITMENT,
			sequence: 1,
			success: true,
			reservations: [{ limitOrderId: SECOND_ORDER, amount: PAYOUT }],
		})

		// Naming a bid takes that bid's hold alone.
		expect(await ctx.bids.claimReservation(COMMITMENT, 1)).toEqual([{ limitOrderId: SECOND_ORDER, amount: PAYOUT }])
		// Without one, whatever is still outstanding on the commitment.
		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([{ limitOrderId: LIMIT_ORDER, amount: PAYOUT }])
		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([])
	})
})

describe("claiming a bid's reservation", () => {
	it("hands it over exactly once", async () => {
		const ctx = await build()
		await placeBid(ctx)

		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([{ limitOrderId: LIMIT_ORDER, amount: PAYOUT }])
		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([])
	})

	it("answers null for a bid that was never priced by a limit order", async () => {
		const ctx = await build()
		await ctx.bids.store({ commitment: COMMITMENT, success: true })
		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([])
	})
})
