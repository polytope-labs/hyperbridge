import { describe, expect, it, vi } from "vitest"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
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
/** The identifiers two bids on one order are filed under: keccak256 of each one's calldata. */
const FIRST_BID = `0x${"b1".repeat(32)}`
const SECOND_BID = `0x${"b2".repeat(32)}`
const LIMIT_ORDER = "limit-0"
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
	;(filler as any).hyperbridge = Promise.resolve({ retractBid })

	const reserved = async () => (await limitOrders.get(LIMIT_ORDER))!.reserved

	return { filler, bids: data.bids, limitOrders, reserved, retractBid }
}

/** A bid that went out holding `PAYOUT` against the limit order. */
async function placeBid(ctx: Awaited<ReturnType<typeof build>>) {
	expect(await ctx.limitOrders.reserve(LIMIT_ORDER, PAYOUT)).toBe(true)
	await ctx.bids.store({
		commitment: COMMITMENT,
		bid: FIRST_BID,
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
			bid: FIRST_BID,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount: (400n * 10n ** 18n).toString() }],
		})
		await ctx.bids.store({
			commitment: COMMITMENT,
			bid: SECOND_BID,
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

	describe("bids at their own rate", () => {
		/** Two bids at different rates, each holding its payout and the take it signed beside it. */
		async function twoRatedBids() {
			const ctx = await build()
			await ctx.limitOrders.reserve(LIMIT_ORDER, (1000n * 10n ** 18n).toString())
			await ctx.limitOrders.reserve(SECOND_ORDER, (980n * 10n ** 18n).toString())
			await ctx.bids.store({
				commitment: COMMITMENT,
				bid: FIRST_BID,
				success: true,
				reservations: [{ limitOrderId: LIMIT_ORDER, amount: (1000n * 10n ** 18n).toString(), take: "100" }],
			})
			await ctx.bids.store({
				commitment: COMMITMENT,
				bid: SECOND_BID,
				success: true,
				reservations: [{ limitOrderId: SECOND_ORDER, amount: (980n * 10n ** 18n).toString(), take: "99" }],
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
			return { ...ctx, settled }
		}

		it("settles the bid whose take was released and draws it down by what it was charged", async () => {
			// The event credits the swapper 980, the ask, while the bid that executed paid its
			// whole 1,000: the rest went to the swapper and the protocol as surplus. Matching the
			// credit to a hold would pick the 980 bid; the released take names the right one.
			const ctx = await twoRatedBids()

			// biome-ignore lint/suspicious/noExplicitAny: the settlement path is private
			await (ctx.filler as any).settleFilledLimitOrder(
				COMMITMENT,
				8453,
				[{ token: CNGN, amount: 980n * 10n ** 18n }],
				[{ token: OTHER, amount: 100n }],
			)

			expect(ctx.settled).toEqual([[LIMIT_ORDER, 1000n * 10n ** 18n]])
			expect((await ctx.limitOrders.get(SECOND_ORDER))!.reserved).toBe("0")
		})

		it("charges a fill the gateway clamped in proportion to the escrow it released", async () => {
			// The order had only part of this bid's credit left, so the gateway released half
			// of its take and charged half of its payout, however the credit rounds. The release
			// fits inside both takes; the executor takes the better rate (10 against 9.9) first.
			const ctx = await twoRatedBids()

			// biome-ignore lint/suspicious/noExplicitAny: the settlement path is private
			await (ctx.filler as any).settleFilledLimitOrder(
				COMMITMENT,
				8453,
				[{ token: CNGN, amount: 490n * 10n ** 18n }],
				[{ token: OTHER, amount: 50n }],
			)

			expect(ctx.settled).toEqual([[LIMIT_ORDER, 500n * 10n ** 18n]])
		})
	})

	it("claims the holds of every bid on the order, so none is left behind", async () => {
		const ctx = await build()
		await ctx.bids.store({
			commitment: COMMITMENT,
			bid: FIRST_BID,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount: PAYOUT }],
		})
		await ctx.bids.store({
			commitment: COMMITMENT,
			bid: SECOND_BID,
			success: true,
			reservations: [{ limitOrderId: SECOND_ORDER, amount: PAYOUT }],
		})

		// Naming a bid takes that bid's hold alone.
		expect(await ctx.bids.claimReservation(COMMITMENT, SECOND_BID)).toEqual([{ limitOrderId: SECOND_ORDER, amount: PAYOUT }])
		// Without one, whatever is still outstanding on the commitment.
		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([{ limitOrderId: LIMIT_ORDER, amount: PAYOUT }])
		expect(await ctx.bids.claimReservation(COMMITMENT)).toEqual([])
	})
})

describe("one solver's bids at several price levels on one order", () => {
	/**
	 * Two limit orders of ours at different prices, each with its own bid on the same
	 * order: 1,000 out for 100 in (rate 10) and 900 out for 100 in (rate 9). The executor
	 * takes the better rate first.
	 */
	async function twoLevels() {
		const ctx = await build()
		await ctx.limitOrders.reserve(LIMIT_ORDER, (1000n * 10n ** 18n).toString())
		await ctx.limitOrders.reserve(SECOND_ORDER, (900n * 10n ** 18n).toString())
		await ctx.bids.store({
			commitment: COMMITMENT,
			bid: FIRST_BID,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount: (1000n * 10n ** 18n).toString(), take: "100" }],
		})
		await ctx.bids.store({
			commitment: COMMITMENT,
			bid: SECOND_BID,
			success: true,
			reservations: [{ limitOrderId: SECOND_ORDER, amount: (900n * 10n ** 18n).toString(), take: "100" }],
		})
		const drawn: Array<[string, bigint]> = []
		// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for the fill path
		const filler = ctx.filler as any
		filler.assetRegistry = { getAddress: () => CNGN }
		filler.contractService = { getTokenDecimals: async () => 18 }
		// A paymaster on the chain, so the fill path skips the EntryPoint top-up.
		filler.configService.getSimplexPaymasterAddress = () => "0x0000000000000000000000000000000000000001"
		filler.limitOrderService = {
			resize: async (order: { id: string }, amount: bigint) => {
				drawn.push([order.id, amount])
				return null
			},
		}
		const reservedOn = async (id: string) => (await ctx.limitOrders.get(id))!.reserved
		const fill = async (credited: bigint, released: bigint, complete: boolean) => {
			await filler.handleOrderFilledOnChain(
				COMMITMENT,
				OUR_ADDRESS,
				8453,
				[{ token: CNGN, amount: credited }],
				[{ token: OTHER, amount: released }],
				complete,
			)
			await filler.retractionQueue.onIdle()
		}
		return { ...ctx, drawn, reservedOn, fill }
	}

	it("settles only the bid a partial fill executed and leaves the other level bidding", async () => {
		const ctx = await twoLevels()

		// The better level fills first and leaves the order open.
		await ctx.fill(950n * 10n ** 18n, 100n, false)

		expect(ctx.drawn).toEqual([[LIMIT_ORDER, 1000n * 10n ** 18n]])
		expect(await ctx.reservedOn(LIMIT_ORDER)).toBe("0")
		// The other level still holds its payout and its bid is still on Hyperbridge.
		expect(await ctx.reservedOn(SECOND_ORDER)).toBe((900n * 10n ** 18n).toString())
		expect(ctx.retractBid).not.toHaveBeenCalled()
	})

	it("settles fills of one order in the order they were scanned, even when they arrive together", async () => {
		// Both levels fill in one block scan, so both events are handed over before
		// either settlement has run. Settled side by side, both read the same holds and
		// the first release was matched to the same bid as the second.
		const ctx = await twoLevels()
		// biome-ignore lint/suspicious/noExplicitAny: driving the monitor's events directly
		const filler = ctx.filler as any
		filler.monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: CNGN, amount: 950n * 10n ** 18n }],
			inputs: [{ token: OTHER, amount: 100n }],
			complete: false,
		})
		filler.monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: CNGN, amount: 540n * 10n ** 18n }],
			inputs: [{ token: OTHER, amount: 60n }],
			complete: true,
		})
		await filler.settlementQueue.onIdle()
		await filler.retractionQueue.onIdle()

		expect(ctx.drawn).toEqual([
			[LIMIT_ORDER, 1000n * 10n ** 18n],
			[SECOND_ORDER, 540n * 10n ** 18n],
		])
	})

	it("works the next level down when it completes the order, then retracts", async () => {
		const ctx = await twoLevels()
		await ctx.fill(950n * 10n ** 18n, 100n, false)

		// The next level fills what is left: the gateway clamps it to 60 of its 100 take,
		// charging 60% of its 900.
		await ctx.fill(540n * 10n ** 18n, 60n, true)

		expect(ctx.drawn).toEqual([
			[LIMIT_ORDER, 1000n * 10n ** 18n],
			[SECOND_ORDER, 540n * 10n ** 18n],
		])
		expect(await ctx.reservedOn(SECOND_ORDER)).toBe("0")
		// The order is complete, so our bids on it are retracted.
		expect(ctx.retractBid).toHaveBeenCalled()
		expect((await ctx.bids.byCommitment(COMMITMENT))!.retracted).toBe(true)
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
