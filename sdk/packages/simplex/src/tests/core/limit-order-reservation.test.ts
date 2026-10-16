import { afterEach, describe, expect, it, vi } from "vitest"
import { DatabaseSync } from "node:sqlite"
import { mkdtempSync, rmSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { SqliteDataStore } from "@/data/sqlite"
import type { BidPricing, BidStore } from "@/data/types"
import { budgetIdFor } from "@/orderbook/amounts"
import { LoggerContext } from "@/services/Logger"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { fakeClient, limitOrderService, limitOrderStore, ORDERBOOK_FIXTURES } from "../helpers/limit-orders"

/**
 * Settling a fill of ours against the limit orders that priced our bids.
 *
 * Nothing is held against a limit order while a bid is out: the order's budget on chain
 * refuses a fill past its cap, in the same calldata as the fill. Each bid row only says
 * which order priced it and on what terms. A fill is attributed to one of those bids and
 * recorded against its order, and every order that priced a bid on the swap is then
 * brought down to its on-chain tally. A sync that fails is left to the tally pass.
 */

const { CHAIN, CNGN, USDC } = ORDERBOOK_FIXTURES
const COMMITMENT = "0x4380111111111111111111111111111111111111111111111111111111114818" as HexString
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
/** The identifiers two bids on one order are filed under: keccak256 of each one's calldata. */
const FIRST_BID = `0x${"b1".repeat(32)}`
const SECOND_BID = `0x${"b2".repeat(32)}`
const LIMIT_ORDER = "3f2b8c1e-9d4a-4f6b-8a7c-5e1d2c3b4a59"
const SECOND_ORDER = "b7a1d2c4-0e5f-4a3b-9c8d-1f2e3d4c5b6a"
/** An order that pays USDC out rather than cNGN: a sell of the base. */
const USDC_ORDER = "c9d8e7f6-1a2b-4c3d-8e4f-5a6b7c8d9e0f"
const OTHER = "0xDDDD00000000000000000000000000000000DDDD" as HexString
const ONE = 10n ** 18n

const dirs: string[] = []
afterEach(() => {
	for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true })
})

/**
 * The real filler and limit order service over two orders paying cNGN on one chain, each of
 * 3,000, with the orderbook and the account's tallies stubbed.
 */
async function build(options: { bids?: BidStore; retract?: BidSubmissionResult } = {}) {
	const bids = options.bids ?? new MemoryDataStore().bids
	const limitOrders = await limitOrderStore([
		{ id: LIMIT_ORDER, base: "USDC", quote: "CNGN", side: "BID", fillChain: CHAIN, price: "1500", size: "3000" },
		{ id: SECOND_ORDER, base: "USDC", quote: "CNGN", side: "BID", fillChain: CHAIN, price: "1450", size: "3000" },
		{ id: USDC_ORDER, base: "USDC", quote: "CNGN", side: "ASK", fillChain: CHAIN, price: "1500", size: "3000" },
	])

	const client = fakeClient([])
	const { service } = limitOrderService(client, limitOrders)
	const spent = new Map<string, bigint>()
	const chain = { readable: true }
	;(service as any).contractService.limitOrderSpent = async (_chain: string, budgetId: string) => {
		if (!chain.readable) throw new Error("rpc down")
		return spent.get(budgetId) ?? 0n
	}
	const syncToChain = vi.spyOn(service, "syncToChain")

	const retractBid = vi.fn(async (): Promise<BidSubmissionResult> => options.retract ?? { success: true })
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
		bids,
		limitOrders,
	) as any
	filler.hyperbridge = Promise.resolve({ retractBid })
	filler.assetRegistry = { getAddress: (symbol: string) => (symbol === "USDC" ? USDC : CNGN) }
	filler.contractService = { getTokenDecimals: async () => 18 }
	filler.setLimitOrderService(service)

	/** A bid of ours on `commitment`, priced by `pricedBy`. */
	const bid = (pricedBy: BidPricing | undefined, bidId = FIRST_BID, commitment = COMMITMENT, success = true) =>
		bids.store({ commitment, bid: bidId, success, pricedBy })

	/** The chain counting `whole` cNGN against a limit order's budget. */
	const spend = (id: string, whole: bigint) => {
		spent.set(budgetIdFor(id), (spent.get(budgetIdFor(id)) ?? 0n) + whole * ONE)
	}

	/** One of our fills reaching the filler, the way the scanner hands it over. */
	const fill = async (
		credited: bigint,
		released: bigint,
		complete: boolean,
		transactionHash?: string,
		logIndex?: number,
	) => {
		await filler.handleOrderFilledOnChain(
			COMMITMENT,
			OUR_ADDRESS,
			8453,
			[{ token: CNGN, amount: credited }],
			[{ token: OTHER, amount: released }],
			complete,
			transactionHash,
			logIndex,
		)
		await filler.retractionQueue.onIdle()
	}

	const remaining = async (id: string) => BigInt((await limitOrders.get(id))!.remaining)
	const recorded = async (id: string) =>
		(await limitOrders.fills(id)).map((row) => [row.commitment, row.bid, row.amount, row.transactionHash])
	const logIndexes = async (id: string) => (await limitOrders.fills(id)).map((row) => row.logIndex)

	return { filler, bids, limitOrders, client, chain, syncToChain, retractBid, bid, spend, fill, remaining, recorded, logIndexes }
}

/** 1,000 out for 100 in on the first order, 900 out for 100 in on the second: rates 10 and 9. */
async function twoLevels(options: Parameters<typeof build>[0] = {}) {
	const ctx = await build(options)
	await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100" }, FIRST_BID)
	await ctx.bid({ limitOrderId: SECOND_ORDER, payout: (900n * ONE).toString(), take: "100" }, SECOND_BID)
	return ctx
}

describe("settling a fill of ours", () => {
	it("records a completing fill against the bid's order, brings it to the chain's tally, and retracts", async () => {
		const ctx = await build()
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100" })
		ctx.spend(LIMIT_ORDER, 1000n)

		await ctx.fill(1000n * ONE, 100n, true, "0xfill")

		expect(await ctx.recorded(LIMIT_ORDER)).toEqual([[COMMITMENT, FIRST_BID, (1000n * ONE).toString(), "0xfill"]])
		expect(ctx.syncToChain).toHaveBeenCalledWith(LIMIT_ORDER)
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
		// Back on the book at its new size.
		expect(ctx.client.submitted).toHaveLength(1)
		expect(ctx.retractBid).toHaveBeenCalled()
	})

	it("records a partial fill against the level that executed and leaves the other level bidding", async () => {
		const ctx = await twoLevels()
		ctx.spend(LIMIT_ORDER, 1000n)

		// The better level fills first and leaves the order open.
		await ctx.fill(950n * ONE, 100n, false, "0xfill1")

		expect(await ctx.recorded(LIMIT_ORDER)).toEqual([[COMMITMENT, FIRST_BID, (1000n * ONE).toString(), "0xfill1"]])
		expect(await ctx.recorded(SECOND_ORDER)).toEqual([])
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
		// Synced too, as an order that priced a bid on this swap; its tally has not moved.
		expect(ctx.syncToChain).toHaveBeenCalledWith(SECOND_ORDER)
		expect(await ctx.remaining(SECOND_ORDER)).toBe(3000n * ONE)
		expect(ctx.retractBid).not.toHaveBeenCalled()

		// The next level fills what is left: the gateway clamps it to 60 of its 100 take,
		// charging 60% of its 900.
		ctx.spend(SECOND_ORDER, 540n)
		await ctx.fill(540n * ONE, 60n, true, "0xfill2")

		expect(await ctx.recorded(SECOND_ORDER)).toEqual([[COMMITMENT, SECOND_BID, (540n * ONE).toString(), "0xfill2"]])
		expect(await ctx.recorded(LIMIT_ORDER)).toHaveLength(1)
		expect(await ctx.remaining(SECOND_ORDER)).toBe(2460n * ONE)
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
		expect(ctx.retractBid).toHaveBeenCalled()
	})

	it("settles fills of one order in the order they were scanned, even when they arrive together", async () => {
		// Both levels fill in one block scan, so both events are handed over before either
		// settlement has run. Side by side, both would name the same bid.
		const ctx = await twoLevels()
		ctx.spend(LIMIT_ORDER, 1000n)
		ctx.spend(SECOND_ORDER, 540n)
		ctx.filler.monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: CNGN, amount: 950n * ONE }],
			inputs: [{ token: OTHER, amount: 100n }],
			complete: false,
		})
		ctx.filler.monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: CNGN, amount: 540n * ONE }],
			inputs: [{ token: OTHER, amount: 60n }],
			complete: true,
		})
		await ctx.filler.settlementQueue.onIdle()
		await ctx.filler.retractionQueue.onIdle()

		expect((await ctx.recorded(LIMIT_ORDER)).map((row) => row[2])).toEqual([(1000n * ONE).toString()])
		expect((await ctx.recorded(SECOND_ORDER)).map((row) => row[2])).toEqual([(540n * ONE).toString()])
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
		expect(await ctx.remaining(SECOND_ORDER)).toBe(2460n * ONE)
	})

	it("records a fill delivered twice only once", async () => {
		// A scanner replaying from a cursor hands the same fill over again.
		const ctx = await twoLevels()
		ctx.spend(LIMIT_ORDER, 1000n)

		await ctx.fill(950n * ONE, 100n, false, "0xfill1")
		await ctx.fill(950n * ONE, 100n, false, "0xfill1")

		expect(await ctx.recorded(LIMIT_ORDER)).toHaveLength(1)
		expect(await ctx.recorded(SECOND_ORDER)).toEqual([])
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
	})

	it("records both of our fills when one transaction carries them, and neither again on a replay", async () => {
		// A bundle can hold several of our UserOps, each with its own fill log: the hash is shared,
		// the log index is what tells the fills apart.
		const ctx = await twoLevels()
		ctx.spend(LIMIT_ORDER, 1000n)
		ctx.spend(SECOND_ORDER, 540n)

		await ctx.fill(950n * ONE, 100n, false, "0xsame", 3)
		await ctx.fill(540n * ONE, 60n, true, "0xsame", 5)

		expect(await ctx.recorded(LIMIT_ORDER)).toEqual([[COMMITMENT, FIRST_BID, (1000n * ONE).toString(), "0xsame"]])
		expect(await ctx.recorded(SECOND_ORDER)).toEqual([[COMMITMENT, SECOND_BID, (540n * ONE).toString(), "0xsame"]])
		expect(await ctx.logIndexes(LIMIT_ORDER)).toEqual([3])
		expect(await ctx.logIndexes(SECOND_ORDER)).toEqual([5])

		// A replay of either records nothing new.
		await ctx.fill(950n * ONE, 100n, false, "0xsame", 3)
		await ctx.fill(540n * ONE, 60n, true, "0xsame", 5)
		expect(await ctx.recorded(LIMIT_ORDER)).toHaveLength(1)
		expect(await ctx.recorded(SECOND_ORDER)).toHaveLength(1)
	})

	it("passes over a bid that failed outright: it never reached Hyperbridge", async () => {
		const ctx = await build()
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100" }, FIRST_BID, COMMITMENT, false)
		await ctx.bid({ limitOrderId: SECOND_ORDER, payout: (900n * ONE).toString(), take: "100" }, SECOND_BID)
		ctx.spend(SECOND_ORDER, 900n)

		await ctx.fill(900n * ONE, 100n, true)

		expect(await ctx.recorded(LIMIT_ORDER)).toEqual([])
		expect((await ctx.recorded(SECOND_ORDER)).map((row) => row[1])).toEqual([SECOND_BID])
	})

	it("does nothing for a swap no limit order priced", async () => {
		const ctx = await build()
		await ctx.bid(undefined)

		await ctx.fill(1n, 1n, true)

		expect(ctx.syncToChain).not.toHaveBeenCalled()
		expect(await ctx.recorded(LIMIT_ORDER)).toEqual([])
	})

	it("records nothing when the fill names none of what the order pays, and still syncs it", async () => {
		const ctx = await build()
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString() })

		await ctx.filler.settleFilledLimitOrder(COMMITMENT, 8453, [{ token: OTHER, amount: 999n * ONE }])

		expect(await ctx.recorded(LIMIT_ORDER)).toEqual([])
		// Still synced: the chain, not the event, says what the order has left.
		expect(ctx.syncToChain).toHaveBeenCalledWith(LIMIT_ORDER)
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(3000n * ONE)
	})
})

describe("when the chain cannot say yet", () => {
	it("does not throw when the tally cannot be read, and leaves the order to the tally pass", async () => {
		const ctx = await build()
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100" })
		ctx.spend(LIMIT_ORDER, 1000n)
		ctx.chain.readable = false

		await expect(
			ctx.filler.settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 1000n * ONE }], [{ token: OTHER, amount: 100n }]),
		).resolves.toBeUndefined()

		// The fill is on record; the order is not lowered yet.
		expect(await ctx.recorded(LIMIT_ORDER)).toHaveLength(1)
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(3000n * ONE)

		ctx.chain.readable = true
		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
	})

	it("leaves the order for the tally pass when the chain does not show the fill yet", async () => {
		const ctx = await build()
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100" })

		await ctx.filler.settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 1000n * ONE }], [{ token: OTHER, amount: 100n }])
		expect(await ctx.recorded(LIMIT_ORDER)).toHaveLength(1)
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(3000n * ONE)

		ctx.spend(LIMIT_ORDER, 1000n)
		await ctx.filler.reconcileLimitOrderTallies()
		expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
	})
})

describe("bids at their own rate", () => {
	/** Two bids at different rates, each with the payout and the take it signed. */
	async function twoRatedBids() {
		const ctx = await build()
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100" }, FIRST_BID)
		await ctx.bid({ limitOrderId: SECOND_ORDER, payout: (980n * ONE).toString(), take: "99" }, SECOND_BID)
		return ctx
	}

	it("attributes the fill to the bid whose take was released, at what it was charged", async () => {
		// The event credits the swapper 980, the ask, while the bid that executed paid its
		// whole 1,000: the rest went to the swapper and the protocol as surplus. Matching the
		// credit to a payout would pick the 980 bid; the released take names the right one.
		const ctx = await twoRatedBids()

		await ctx.filler.settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 980n * ONE }], [{ token: OTHER, amount: 100n }])

		expect((await ctx.recorded(LIMIT_ORDER)).map((row) => row[2])).toEqual([(1000n * ONE).toString()])
		expect(await ctx.recorded(SECOND_ORDER)).toEqual([])
	})

	it("charges a fill the gateway clamped in proportion to the escrow it released", async () => {
		// The order had only part of this bid's credit left, so the gateway released half of
		// its take and charged half of its payout, however the credit rounds. The release fits
		// inside both takes; the executor takes the better rate (10 against 9.9) first.
		const ctx = await twoRatedBids()

		await ctx.filler.settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 490n * ONE }], [{ token: OTHER, amount: 50n }])

		expect((await ctx.recorded(LIMIT_ORDER)).map((row) => row[2])).toEqual([(500n * ONE).toString()])
	})

	it("records what a fill took in, at 1e18, from the decimals its bid carries", async () => {
		const ctx = await build()
		// The first bid's take is counted in a six-decimal token; the second predates fills
		// keeping what they took in, and says nothing about its take's decimals.
		await ctx.bid({ limitOrderId: LIMIT_ORDER, payout: (1000n * ONE).toString(), take: "100000000", takeDecimals: 6 })
		const other = "0xfeed00000000000000000000000000000000000000000000000000000000beef"
		await ctx.bid({ limitOrderId: SECOND_ORDER, payout: (980n * ONE).toString(), take: "99" }, SECOND_BID, other as HexString)

		// Half the first bid's take is released: 50 of the six-decimal token.
		await ctx.filler.settleFilledLimitOrder(COMMITMENT, 8453, [{ token: CNGN, amount: 500n * ONE }], [{ token: OTHER, amount: 50_000_000n }])
		await ctx.filler.settleFilledLimitOrder(other, 8453, [{ token: CNGN, amount: 980n * ONE }], [{ token: OTHER, amount: 99n }])

		expect((await ctx.limitOrders.fills(LIMIT_ORDER)).map((row) => [row.amount, row.amountIn])).toEqual([
			[(500n * ONE).toString(), (50n * ONE).toString()],
		])
		expect((await ctx.limitOrders.fills(SECOND_ORDER)).map((row) => row.amountIn)).toEqual([null])
	})
})

describe("a multi-leg swap priced by limit orders that pay different tokens", () => {
	it("attributes a fill to the bid whose order pays the token it delivered", async () => {
		const ctx = await build()
		// Leg 1, priced by the cNGN-paying order, was stored first; leg 0's bid, priced by the
		// USDC-paying order, is the newest row and so comes first among the candidates.
		await ctx.bid({ limitOrderId: SECOND_ORDER, payout: (900n * ONE).toString(), take: "100" }, SECOND_BID)
		await ctx.bid({ limitOrderId: USDC_ORDER, payout: (500n * ONE).toString(), take: "100" }, FIRST_BID)
		ctx.spend(SECOND_ORDER, 900n)

		// The fill delivers cNGN on leg 1; the escrow it released fits both bids' takes.
		await ctx.fill(900n * ONE, 100n, false, "0xleg1", 2)

		expect(await ctx.recorded(SECOND_ORDER)).toEqual([[COMMITMENT, SECOND_BID, (900n * ONE).toString(), "0xleg1"]])
		expect(await ctx.recorded(USDC_ORDER)).toEqual([])
		expect(await ctx.remaining(SECOND_ORDER)).toBe(2100n * ONE)
		// Both orders priced a bid on the swap, so both are brought to their tally.
		expect(ctx.syncToChain).toHaveBeenCalledWith(USDC_ORDER)
		expect(ctx.syncToChain).toHaveBeenCalledWith(SECOND_ORDER)
	})
})

describe("a bid placed before the upgrade", () => {
	it("still attributes its fill from the reservations its row carries", async () => {
		// A bid row as the build with limit-order holds wrote it: no `priced_by`, the hold in
		// `reservations`, still unsettled when the solver was upgraded.
		const dir = mkdtempSync(join(tmpdir(), "simplex-holds-upgrade-"))
		dirs.push(dir)
		const legacy = new DatabaseSync(join(dir, "bids.db"))
		legacy.exec(`
			CREATE TABLE bids (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				commitment TEXT NOT NULL,
				bid TEXT,
				extrinsic_hash TEXT,
				block_hash TEXT,
				success INTEGER NOT NULL,
				pending INTEGER NOT NULL DEFAULT 0,
				error TEXT,
				created_at TEXT NOT NULL DEFAULT (datetime('now')),
				retracted INTEGER NOT NULL DEFAULT 0,
				retracted_at TEXT,
				retract_extrinsic_hash TEXT,
				dead INTEGER NOT NULL DEFAULT 0,
				reservations TEXT
			);
		`)
		legacy
			.prepare("INSERT INTO bids (commitment, bid, success, reservations) VALUES (?, ?, 1, ?)")
			.run(COMMITMENT, FIRST_BID, JSON.stringify([{ limitOrderId: LIMIT_ORDER, amount: (1000n * ONE).toString(), take: "100" }]))
		legacy.close()

		const data = new SqliteDataStore(dir, new LoggerContext({ level: "warn" }))
		try {
			const ctx = await build({ bids: data.bids })
			ctx.spend(LIMIT_ORDER, 1000n)

			await ctx.fill(980n * ONE, 100n, true, "0xfill")

			expect(await ctx.recorded(LIMIT_ORDER)).toEqual([[COMMITMENT, FIRST_BID, (1000n * ONE).toString(), "0xfill"]])
			expect(await ctx.remaining(LIMIT_ORDER)).toBe(2000n * ONE)
		} finally {
			await data.close()
		}
	})
})
