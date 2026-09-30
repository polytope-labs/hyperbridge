import { describe, expect, it } from "vitest"
import { bytes20ToBytes32, type HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { budgetFor, toRaw, toScaled } from "@/orderbook/amounts"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { fakeClient, limitOrderService, limitOrderStore, ORDERBOOK_FIXTURES } from "../helpers/limit-orders"

/**
 * The solver's account tallies what each limit order has paid out, and the store keeps
 * its own `remaining`. The tally pass lowers `remaining` to the room the tally leaves.
 * It runs beside fills that are settling, and a fill the chain has already counted but
 * the store has not drawn down yet must not be taken off the order twice.
 */

const { CHAIN, USDC, CNGN, ONE } = ORDERBOOK_FIXTURES
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const LIMIT_ORDER = "3f2b8c1e-9d4a-4f6b-8a7c-5e1d2c3b4a59"
const DECIMALS = 6
const UNIT = 10n ** BigInt(DECIMALS)
/** What the swapper pays for 1 USDC, in cNGN's own units. */
const RATE = 1500n * ONE

/**
 * The real filler and the real limit order service over one order paying out 1,000 USDC,
 * with the account's tally and the orderbook stubbed. `slow` puts a delay in front of
 * every tally read and every posting, which is where two passes would cross over.
 */
async function build(options: { slow?: boolean } = {}) {
	const pause = () => (options.slow ? new Promise((resolve) => setTimeout(resolve, 5)) : Promise.resolve())
	const data = new MemoryDataStore()
	const limitOrders = await limitOrderStore([
		{ id: LIMIT_ORDER, base: "USDC", quote: "CNGN", side: "ASK", fillChain: CHAIN, price: "1500", size: "1000" },
	])

	const client = fakeClient([])
	const submit = client.submitOrder
	client.submitOrder = async (userOp) => {
		await pause()
		return submit(userOp)
	}
	const { service } = limitOrderService(client, limitOrders)
	let tally = 0n
	;(service as any).contractService.limitOrderSpent = async () => {
		await pause()
		return tally
	}

	const filler = new IntentFiller(
		[],
		[],
		{},
		{
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
			// A paymaster on the chain, so the fill path skips the EntryPoint top-up.
			getSimplexPaymasterAddress: () => "0x0000000000000000000000000000000000000001",
		} as any,
		{} as any,
		{} as any,
		{ address: OUR_ADDRESS } as any,
		{ orders: stubOrderScanner() },
		undefined,
		data.bids,
		limitOrders,
	) as any
	filler.assetRegistry = { getAddress: () => USDC }
	filler.contractService = { getTokenDecimals: async () => DECIMALS }
	filler.setLimitOrderService(service)

	const trace: string[] = []
	const traced =
		(name: string, inner: (...args: any[]) => Promise<unknown>) =>
		async (...args: unknown[]) => {
			trace.push(`${name}:start`)
			try {
				return await inner(...args)
			} finally {
				trace.push(`${name}:end`)
			}
		}
	service.reconcileTallies = traced(
		"tallies",
		service.reconcileTallies.bind(service),
	) as typeof service.reconcileTallies
	filler.settleFilledLimitOrder = traced("fill", filler.settleFilledLimitOrder.bind(filler))

	const { cap } = budgetFor((await limitOrders.get(LIMIT_ORDER))!, USDC, DECIMALS)
	let bids = 0

	/** A bid of ours holding `whole` USDC against the order, not yet executed. */
	const placeBid = async (whole: bigint) => {
		const commitment = `0x${(++bids).toString(16).padStart(64, "0")}` as HexString
		const amount = (whole * ONE).toString()
		const take = whole * RATE
		expect(await limitOrders.reserve(LIMIT_ORDER, amount)).toBe(true)
		await data.bids.store({
			commitment,
			bid: `0x${"b".repeat(63)}${bids}`,
			success: true,
			reservations: [{ limitOrderId: LIMIT_ORDER, amount, take: take.toString() }],
		})
		return { commitment, whole, take }
	}

	/**
	 * The chain executing that bid and counting it, not yet settled here. `charged` is less
	 * than the hold when the swap had less of its ask open than the bid offered.
	 */
	const land = (bid: Awaited<ReturnType<typeof placeBid>>, charged: bigint = bid.whole) => {
		tally += charged * UNIT
		return { ...bid, charged }
	}

	const executeBid = async (whole: bigint) => land(await placeBid(whole))

	/** The fill event for that bid reaching the filler, which queues its settlement. */
	const scanFill = (fill: ReturnType<typeof land>) => {
		filler.monitor.emit("orderFilledOnChain", {
			commitment: fill.commitment,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: bytes20ToBytes32(USDC), amount: fill.charged * UNIT }],
			inputs: [{ token: bytes20ToBytes32(CNGN), amount: (fill.take * fill.charged) / fill.whole }],
			complete: true,
		})
	}

	return {
		cap,
		client,
		filler,
		trace,
		placeBid,
		land,
		executeBid,
		scanFill,
		/** A bid that never executed giving its hold back, as its retraction does. */
		release: (bid: { commitment: HexString }) => filler.releaseReservation(bid.commitment) as Promise<void>,
		/** A payout the account counted that never reached the store. */
		miss: (whole: bigint) => {
			tally += whole * UNIT
		},
		tally: () => tally,
		order: async () => (await limitOrders.get(LIMIT_ORDER))!,
		idle: () => filler.settlementQueue.onIdle() as Promise<void>,
	}
}

describe("a limit order's tally against a fill that has not settled yet", () => {
	it("leaves the order alone while the fill is held, and agrees with the chain once it settles", async () => {
		const ctx = await build()
		const bid = await ctx.executeBid(250n)

		// The tally leaves room for 750, and the 250 the bid holds accounts for the rest.
		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.order()).toMatchObject({
			remaining: (1000n * ONE).toString(),
			reserved: (250n * ONE).toString(),
			status: "open",
		})
		expect(ctx.client.submitted).toEqual([])

		ctx.scanFill(bid)
		await ctx.idle()
		const settled = await ctx.order()
		expect(toRaw(BigInt(settled.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())
		expect(settled.reserved).toBe("0")
		expect(ctx.client.submitted).toHaveLength(1)

		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.order()).toEqual(settled)
		expect(ctx.client.submitted).toHaveLength(1)
	})

	it("takes off only what the store missed when a fill is held beside it", async () => {
		const ctx = await build()
		ctx.miss(100n)
		const bid = await ctx.executeBid(250n)

		await ctx.filler.reconcileLimitOrderTallies()
		expect((await ctx.order()).remaining).toBe((900n * ONE).toString())

		ctx.scanFill(bid)
		await ctx.idle()
		const settled = await ctx.order()
		expect(toRaw(BigInt(settled.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())

		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.order()).toEqual(settled)
	})

	it("finishes the correction once a fill that charged less than it held has settled", async () => {
		// The bid holds 250 and the gateway charges it 200. Until it settles the pass
		// can only see the hold, so it leaves the 50 the fill will give back.
		const ctx = await build()
		ctx.miss(100n)
		const fill = ctx.land(await ctx.placeBid(250n), 200n)

		await ctx.filler.reconcileLimitOrderTallies()
		expect((await ctx.order()).remaining).toBe((950n * ONE).toString())

		ctx.scanFill(fill)
		await ctx.idle()
		expect(await ctx.order()).toMatchObject({ remaining: (750n * ONE).toString(), reserved: "0" })

		await ctx.filler.reconcileLimitOrderTallies()
		const corrected = await ctx.order()
		expect(toRaw(BigInt(corrected.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())
		expect(corrected.remaining).toBe((700n * ONE).toString())

		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.order()).toEqual(corrected)
	})

	it("never goes under the true room while other bids hold the order, and reaches it once they let go", async () => {
		const ctx = await build()
		ctx.miss(100n)
		const pending = [await ctx.placeBid(250n), await ctx.placeBid(250n)]
		const fill = await ctx.executeBid(250n)
		const room = () => toScaled(ctx.cap - ctx.tally(), DECIMALS)

		// Any of the three holds could be the fill the tally counted, so none of them
		// is taken off.
		await ctx.filler.reconcileLimitOrderTallies()
		expect((await ctx.order()).remaining).toBe((1000n * ONE).toString())

		ctx.scanFill(fill)
		await ctx.idle()
		await ctx.filler.reconcileLimitOrderTallies()
		const held = await ctx.order()
		expect(held.reserved).toBe((500n * ONE).toString())
		expect(BigInt(held.remaining)).toBeGreaterThanOrEqual(room())

		for (const bid of pending) await ctx.release(bid)
		await ctx.filler.reconcileLimitOrderTallies()
		const corrected = await ctx.order()
		expect(corrected.reserved).toBe("0")
		expect(toRaw(BigInt(corrected.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())
		expect(corrected.remaining).toBe((650n * ONE).toString())
	})
})

describe("the tally pass and a fill's settlement", () => {
	/** A payout the store missed and a fill waiting to settle, with the two queued in either order. */
	const queued = async (first: "tallies" | "fill") => {
		const ctx = await build({ slow: true })
		ctx.miss(100n)
		const bid = await ctx.executeBid(250n)

		let pass: Promise<void>
		if (first === "tallies") {
			pass = ctx.filler.reconcileLimitOrderTallies()
			ctx.scanFill(bid)
		} else {
			ctx.scanFill(bid)
			pass = ctx.filler.reconcileLimitOrderTallies()
		}
		await pass
		// The pass has run by the time its promise resolves.
		expect(ctx.trace).toContain("tallies:end")
		await ctx.idle()

		const { remaining, reserved, status } = await ctx.order()
		return {
			trace: ctx.trace,
			state: { remaining, reserved, status },
			room: toScaled(ctx.cap - ctx.tally(), DECIMALS),
		}
	}

	it("run one after the other, and leave the order the same whichever went first", async () => {
		const talliesFirst = await queued("tallies")
		const fillFirst = await queued("fill")

		expect(talliesFirst.trace).toEqual(["tallies:start", "tallies:end", "fill:start", "fill:end"])
		expect(fillFirst.trace).toEqual(["fill:start", "fill:end", "tallies:start", "tallies:end"])

		expect(talliesFirst.state).toEqual({
			remaining: talliesFirst.room.toString(),
			reserved: "0",
			status: "open",
		})
		expect(fillFirst.state).toEqual(talliesFirst.state)
	})
})
