import { describe, expect, it, vi } from "vitest"
import { bytes20ToBytes32, type HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { budgetFor, toRaw, toScaled } from "@/orderbook/amounts"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { fakeClient, limitOrderService, limitOrderStore, ORDERBOOK_FIXTURES } from "../helpers/limit-orders"

/**
 * The solver's account tallies what each limit order has paid out, and that tally is
 * the order's budget: `remaining` follows it. A fill's settlement lowers the order to
 * the room the tally leaves, and the periodic tally pass does the same for anything
 * settlement missed. Bids still waiting count for nothing until the chain counts them.
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
	// The block each tally read was pinned to, undefined for a read of the latest block.
	const reads: Array<bigint | undefined> = []
	;(service as any).contractService.limitOrderSpent = async (_chain: string, _budgetId: HexString, blockNumber?: bigint) => {
		reads.push(blockNumber)
		await pause()
		return tally
	}
	// What each posting offered to pay out, in the paid token's own units.
	const posted: bigint[] = []
	const prepare = (service as any).contractService.prepareLimitOrderUserOp
	;(service as any).contractService.prepareLimitOrderUserOp = async (params: { outputAmount: bigint }) => {
		posted.push(params.outputAmount)
		return prepare(params)
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

	/** A bid of ours offering `whole` USDC from the order, not yet executed. */
	const placeBid = async (whole: bigint) => {
		const commitment = `0x${(++bids).toString(16).padStart(64, "0")}` as HexString
		const payout = (whole * ONE).toString()
		const take = whole * RATE
		await data.bids.store({
			commitment,
			bid: `0x${"b".repeat(63)}${bids}`,
			success: true,
			pricedBy: { limitOrderId: LIMIT_ORDER, payout, take: take.toString() },
		})
		return { commitment, whole, take }
	}

	/**
	 * The chain executing that bid and counting it, not yet settled here. `charged` is less
	 * than the payout when the swap had less of its ask open than the bid offered.
	 */
	const land = (bid: Awaited<ReturnType<typeof placeBid>>, charged: bigint = bid.whole) => {
		tally += charged * UNIT
		return { ...bid, charged }
	}

	const executeBid = async (whole: bigint) => land(await placeBid(whole))

	/** The fill event for that bid reaching the filler, which queues its settlement. */
	const scanFill = (fill: ReturnType<typeof land>, blockNumber?: bigint) => {
		filler.monitor.emit("orderFilledOnChain", {
			commitment: fill.commitment,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: bytes20ToBytes32(USDC), amount: fill.charged * UNIT }],
			inputs: [{ token: bytes20ToBytes32(CNGN), amount: (fill.take * fill.charged) / fill.whole }],
			complete: true,
			blockNumber,
		})
	}

	return {
		cap,
		client,
		service,
		reads,
		posted,
		filler,
		trace,
		placeBid,
		land,
		executeBid,
		scanFill,
		/** A payout the account counted that never reached the store. */
		miss: (whole: bigint) => {
			tally += whole * UNIT
		},
		tally: () => tally,
		order: async () => (await limitOrders.get(LIMIT_ORDER))!,
		idle: () => filler.settlementQueue.onIdle() as Promise<void>,
	}
}

describe("a limit order's remaining against its on-chain tally", () => {
	it("follows the chain as soon as it counts a fill, and settlement leaves it there", async () => {
		const ctx = await build()
		const bid = await ctx.executeBid(250n)

		// The chain has counted the fill: the tally pass lowers the order before the fill
		// event even reaches the filler.
		await ctx.filler.reconcileLimitOrderTallies()
		const lowered = await ctx.order()
		expect(toRaw(BigInt(lowered.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())
		expect(lowered.status).toBe("open")
		expect(ctx.client.submitted).toHaveLength(1)

		ctx.scanFill(bid)
		await ctx.idle()
		expect(await ctx.order()).toEqual(lowered)
		expect(ctx.client.submitted).toHaveLength(1)
	})

	it("lowers the order to the tally when the fill settles, taking off what the store missed too", async () => {
		const ctx = await build()
		ctx.miss(100n)
		const bid = await ctx.executeBid(250n)

		ctx.scanFill(bid)
		await ctx.idle()
		const settled = await ctx.order()
		expect(settled.remaining).toBe((650n * ONE).toString())
		expect(toRaw(BigInt(settled.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())

		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.order()).toEqual(settled)
	})

	it("takes off what the gateway charged, not what the bid offered", async () => {
		// The bid offers 250 and the gateway charges it 200: the chain counted 200.
		const ctx = await build()
		const fill = ctx.land(await ctx.placeBid(250n), 200n)

		ctx.scanFill(fill)
		await ctx.idle()
		expect((await ctx.order()).remaining).toBe((800n * ONE).toString())
	})

	it("ignores bids still waiting: only what the chain counted comes off", async () => {
		const ctx = await build()
		await ctx.placeBid(250n)
		await ctx.placeBid(250n)
		const fill = await ctx.executeBid(250n)

		ctx.scanFill(fill)
		await ctx.idle()
		await ctx.filler.reconcileLimitOrderTallies()
		const after = await ctx.order()
		expect(after.remaining).toBe((750n * ONE).toString())
		expect(toRaw(BigInt(after.remaining), DECIMALS)).toBe(ctx.cap - ctx.tally())
	})
})

describe("the tally a fill is settled against", () => {
	it("is read at the block the fill landed in, and the periodic pass at the latest", async () => {
		// A load-balanced endpoint can lag the one that saw the fill, and a read at the
		// latest block would then return the tally from before it.
		const ctx = await build()
		const fill = await ctx.executeBid(250n)

		ctx.scanFill(fill, 12_345n)
		await ctx.idle()
		// The sync is the first read. The repost it triggers checks the tally again before
		// it posts, with nothing to pin it to.
		expect(ctx.reads).toEqual([12_345n, undefined])
		expect((await ctx.order()).remaining).toBe((750n * ONE).toString())

		ctx.reads.length = 0
		await ctx.filler.reconcileLimitOrderTallies()
		expect(ctx.reads).toEqual([undefined])
	})

	it("is read at the latest block when the scanner reported no block", async () => {
		const ctx = await build()
		ctx.scanFill(await ctx.executeBid(250n))
		await ctx.idle()
		expect(ctx.reads[0]).toBeUndefined()
		expect(ctx.reads.length).toBeGreaterThan(0)
	})
})

describe("a tally read pinned to the fill's block that fails", () => {
	/** The chain answers the latest block but not the pinned one, as a node that pruned that state would. */
	const prunedAt = (ctx: Awaited<ReturnType<typeof build>>, answers = true) => {
		;(ctx.service as any).contractService.limitOrderSpent = async (
			_chain: string,
			_budgetId: HexString,
			blockNumber?: bigint,
		) => {
			ctx.reads.push(blockNumber)
			return blockNumber === undefined && answers ? ctx.tally() : null
		}
	}

	it("is read once more at the latest block, which lowers the order", async () => {
		const ctx = await build()
		prunedAt(ctx)
		const fill = await ctx.executeBid(250n)

		ctx.scanFill(fill, 12_345n)
		await ctx.idle()

		// Pinned and failed, then the latest; the repost it triggers reads the latest once more.
		expect(ctx.reads).toEqual([12_345n, undefined, undefined])
		expect((await ctx.order()).remaining).toBe((750n * ONE).toString())
		expect(ctx.posted.at(-1)).toBe(750n * UNIT)
	})

	it("leaves the order where it is when the latest block cannot be read either", async () => {
		const ctx = await build()
		prunedAt(ctx, false)
		const fill = await ctx.executeBid(250n)
		const submitted = ctx.client.submitted.length

		ctx.scanFill(fill, 12_345n)
		await ctx.idle()

		expect(ctx.reads).toEqual([12_345n, undefined])
		expect((await ctx.order()).remaining).toBe((1000n * ONE).toString())
		expect(ctx.client.submitted).toHaveLength(submitted)
	})

	it("is not read twice when nothing was pinned", async () => {
		const ctx = await build()
		prunedAt(ctx, false)

		expect(await ctx.service.syncToChain(LIMIT_ORDER)).toBe(false)
		expect(ctx.reads).toEqual([undefined])
	})
})

describe("a fill's sync and a repost that takes the row first", () => {
	it("leaves the row to the repost, which is the only one to post", async () => {
		const ctx = await build()
		await ctx.executeBid(250n)
		const submitted = ctx.client.submitted.length
		// The sync reads the row `open`, then the tally; a repost claims the row in between.
		const read = (ctx.service as any).contractService.limitOrderSpent
		;(ctx.service as any).contractService.limitOrderSpent = async (...args: unknown[]) => {
			await (ctx.service as any).store.setStatus(LIMIT_ORDER, "resizing")
			return read(...args)
		}

		expect(await ctx.service.syncToChain(LIMIT_ORDER, 12_345n)).toBe(false)

		const held = await ctx.order()
		expect(held.status).toBe("resizing")
		expect(held.remaining).toBe((1000n * ONE).toString())
		expect(ctx.client.submitted).toHaveLength(submitted)
	})

	it("lowers the row the repost holds, which is how it applies a fill that landed meanwhile", async () => {
		const ctx = await build()
		await (ctx.service as any).store.setStatus(LIMIT_ORDER, "resizing")
		await ctx.executeBid(250n)

		const reposted = await ctx.service.repost(await ctx.order())

		expect(reposted?.remaining).toBe((750n * ONE).toString())
		expect(ctx.posted.at(-1)).toBe(750n * UNIT)
	})
})

describe("a fill on a bid row that records no provenance", () => {
	it("settles nothing and fails nothing, for a row whose pricedBy is left out", async () => {
		// A custom `BidStore` holding rows from before bids carried `pricedBy` can return it
		// undefined rather than null; both mean the same thing.
		const ctx = await build()
		const fill = await ctx.executeBid(250n)
		const bids = (ctx.filler as any).bidStorage
		const byCommitments = bids.byCommitments.bind(bids)
		bids.byCommitments = async (commitments: HexString[]) =>
			(await byCommitments(commitments)).map((row: object) => ({ ...row, pricedBy: undefined }))
		const error = vi.spyOn((ctx.filler as any).logger, "error")

		ctx.scanFill(fill)
		await ctx.idle()

		expect(error).not.toHaveBeenCalled()
		expect(ctx.reads).toEqual([])
	})
})

describe("a fill that lands while the order is being reposted", () => {
	it("is applied by the repost, which goes out at the chain's room", async () => {
		const ctx = await build()
		const resizing = await ctx.order()
		// A renewal, a resize or a re-sign holds the row in `resizing` while it works.
		await (ctx.service as any).store.setStatus(resizing.id, "resizing")
		const fill = await ctx.executeBid(250n)

		// The fill settles while the repost is under way: the order is not `open`, so
		// the sync leaves it, and the row still advertises the pre-fill size.
		ctx.scanFill(fill)
		await ctx.idle()
		expect((await ctx.order()).remaining).toBe((1000n * ONE).toString())

		const reposted = await ctx.service.repost(await ctx.order())

		expect(reposted?.remaining).toBe((750n * ONE).toString())
		expect(reposted?.status).toBe("open")
		expect((await ctx.order()).remaining).toBe((750n * ONE).toString())
		expect(ctx.posted.at(-1)).toBe(750n * UNIT)
	})

	it("closes the order instead when the room left is under the dust floor", async () => {
		const ctx = await build()
		const resizing = await ctx.order()
		await (ctx.service as any).store.setStatus(resizing.id, "resizing")
		// The chain has counted nearly all of it, below what the orderbook takes.
		;(ctx.service as any).dustFloor = async () => 100n * ONE
		ctx.miss(950n)
		const submitted = ctx.client.submitted.length

		expect(await ctx.service.repost(await ctx.order())).toBeNull()

		expect((await ctx.order()).status).toBe("filled")
		expect(ctx.client.submitted).toHaveLength(submitted)
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

		const { remaining, status } = await ctx.order()
		return {
			trace: ctx.trace,
			state: { remaining, status },
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
			status: "open",
		})
		expect(fillFirst.state).toEqual(talliesFirst.state)
	})
})
