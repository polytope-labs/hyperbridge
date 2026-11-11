import { describe, expect, it, vi } from "vitest"
import { bytes20ToBytes32, type HexString } from "@hyperbridge/sdk"
import { type BidPlacer, bidRefusal, orderbookHoldsBid } from "@/core/bid-placer"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore, sqliteDatetime } from "@/data/memory"
import type { SubmitBidResult } from "@/orderbook/types"
import { CacheService } from "@/services/CacheService"
import type { FillResult } from "@/strategies/base"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { fakeClient, limitOrderService, limitOrderStore, ORDERBOOK_FIXTURES } from "../helpers/limit-orders"

/**
 * Simplex bids on the HyperFX orderbook alone. A bid is placed when the orderbook holds it: it is
 * recorded as placed, reported as a successful execution, and a fill it wins is settled against
 * the limit order that priced it. A bid the orderbook refuses, or a post that gets no answer, is a
 * failed bid carrying the orderbook's reason.
 */

const { CHAIN, USDC, ONE } = ORDERBOOK_FIXTURES
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const COMMITMENT = `0x${"c0".repeat(32)}` as HexString
const BID = `0x${"b1".repeat(32)}` as HexString
const LIMIT_ORDER = "3f2b8c1e-9d4a-4f6b-8a7c-5e1d2c3b4a59"
const DECIMALS = 6
const UNIT = 10n ** BigInt(DECIMALS)

const ACCEPTED: SubmitBidResult = {
	kind: "accepted",
	bid: {
		commitment: COMMITMENT,
		leg: 0,
		side: "ASK",
		fillChain: CHAIN,
		sourceChain: CHAIN,
		price: (1500n * ONE).toString(),
		amountIn: (375_000n * ONE).toString(),
		amountOut: (250n * ONE).toString(),
		expiresAt: "2026-10-21T00:10:00Z",
	},
}

const USER_OP = "0x0102" as HexString
const logger = { trace: vi.fn(), debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn(), fatal: vi.fn() }

describe("bidding on the orderbook", () => {
	/**
	 * A filler whose strategy places one signed op through the placer the filler hands it and
	 * answers as the FX strategy does: placed when the orderbook holds it.
	 */
	function bidding() {
		const cache = new CacheService()
		const placers: (BidPlacer | undefined)[] = []
		const strategy = {
			name: "test",
			canFill: async () => true,
			calculateProfitability: async () => 1,
			executeOrder: async (_order: unknown, placer?: BidPlacer): Promise<FillResult> => {
				placers.push(placer)
				if (!placer) return { success: false, error: "no placer" }
				const answer = await placer.place(COMMITMENT, USER_OP)
				return {
					success: orderbookHoldsBid(answer),
					error: bidRefusal(answer),
					orderbook: answer,
					commitment: COMMITMENT,
					bid: BID,
				}
			},
			getOrderUsdValue: async () => ({ inputUsd: { toNumber: () => 1 } }),
		}
		const bids = new MemoryDataStore().bids
		const filler = new IntentFiller(
			[{ chainId: 1 } as never],
			[strategy as never],
			{ maxConcurrentOrders: 1 } as never,
			{
				loggers: { get: () => logger },
				getIntentGatewayAddress: () => "0xGATE",
				getRpcUrls: () => ["https://rpc.example"],
				getTargetGasUnits: () => 1n,
				getSimplexPaymasterAddress: () => "0x0000000000000000000000000000000000000001",
			} as never,
			{} as never,
			{ cacheService: cache } as never,
			{ address: OUR_ADDRESS } as never,
			{ orders: stubOrderScanner([1]) },
			undefined,
			bids,
		)
		const executed: { success: boolean; error?: string; txHash?: string }[] = []
		const filled: unknown[] = []
		filler.monitor.on("orderExecuted", (event: { success: boolean; error?: string }) => executed.push(event))
		filler.monitor.on("orderFilled", (event: unknown) => filled.push(event))
		const run = async (id = "0xorder") => {
			;(filler as any).executeOrder({ id, source: "EVM-1", destination: "EVM-1" }, strategy, true, { toNumber: () => 1 }, 1)
			await (filler as any).chainQueues.get(1).onIdle()
		}
		return { filler, bids, executed, filled, placers, run }
	}

	it("posts the signed op to the orderbook and records the bid placed", async () => {
		const { filler, bids, executed, filled, run } = bidding()
		const postBid = vi.fn(async () => ACCEPTED)
		filler.setOrderbook({ submitBid: postBid } as never)

		await run()

		expect(postBid).toHaveBeenCalledExactlyOnceWith(USER_OP)
		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({
			bid: BID,
			success: true,
			onOrderbook: true,
			pending: false,
			extrinsicHash: null,
			error: null,
		})
		expect(executed).toEqual([expect.objectContaining({ success: true, error: undefined, txHash: undefined })])
		// A bid is not a fill: the event names the commitment, which the activity feed reads as a bid.
		expect(filled).toEqual([expect.objectContaining({ commitment: COMMITMENT })])
		expect(await bids.stats()).toEqual({ total: 1, successful: 1, failed: 0 })
	})

	it("counts a bid the orderbook already held as placed", async () => {
		const { filler, bids, executed, run } = bidding()
		filler.setOrderbook({ submitBid: async () => ({ ...ACCEPTED, kind: "unchanged" }) } as never)

		await run()

		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({ success: true, onOrderbook: true })
		expect(executed).toEqual([expect.objectContaining({ success: true })])
	})

	it("records a bid the orderbook refused as failed, with its reason", async () => {
		const { filler, bids, executed, filled, run } = bidding()
		filler.setOrderbook({
			submitBid: async () => ({ kind: "rejected", code: "NO_LIMIT_ORDER", message: "no limit order" }),
		} as never)

		await run()

		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({
			success: false,
			onOrderbook: false,
			error: "NO_LIMIT_ORDER: no limit order",
		})
		expect(executed).toEqual([expect.objectContaining({ success: false, error: "NO_LIMIT_ORDER: no limit order" })])
		expect(filled).toEqual([])
		expect(await bids.failed()).toHaveLength(1)
	})

	it("records a post that got no answer as a failed bid instead of throwing", async () => {
		const { filler, bids, executed, run } = bidding()
		filler.setOrderbook({
			submitBid: async () => {
				throw new Error("Could not reach the orderbook")
			},
		} as never)

		await run()

		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({
			success: false,
			error: "REQUEST_FAILED: Could not reach the orderbook",
		})
		expect(executed).toEqual([expect.objectContaining({ success: false })])
	})

	it("delegates the SolverAccount and funds the EntryPoint with no Hyperbridge account at all", async () => {
		const { filler, run } = bidding()
		const setupDelegationOnChains = vi.fn(async () => ({ success: true, results: { "EVM-1": true } }))
		;(filler as any).delegation = () => ({ setupDelegationOnChains })
		const topUpEntryPointDeposit = vi.fn(async () => undefined)
		;(filler as any).contractService.topUpEntryPointDeposit = topUpEntryPointDeposit
		;(filler as any).configService.getSimplexPaymasterAddress = () => undefined
		const postBid = vi.fn(async () => ACCEPTED)
		filler.setOrderbook({ submitBid: postBid } as never)

		await (filler as any).setupSolverSelection(["EVM-1"], { requireAll: true })
		expect(setupDelegationOnChains).toHaveBeenCalledWith(["EVM-1"])
		expect(topUpEntryPointDeposit).toHaveBeenCalledWith("EVM-1", 1n)

		await run()
		expect(postBid).toHaveBeenCalledWith(USER_OP)
	})

	it("hands the strategy no placer for a destination that selects no solver", async () => {
		const { filler, placers } = bidding()
		filler.setOrderbook({ submitBid: async () => ACCEPTED } as never)
		const strategy = (filler as any).strategies[0]

		;(filler as any).executeOrder({ id: "0xorder", source: "EVM-1", destination: "EVM-1" }, strategy, false, { toNumber: () => 1 }, 1)
		await (filler as any).chainQueues.get(1).onIdle()

		expect(placers).toEqual([undefined])
	})
})

describe("settling a fill an orderbook bid won", () => {
	async function settlingFiller() {
		const data = new MemoryDataStore()
		const limitOrders = await limitOrderStore([
			{ id: LIMIT_ORDER, base: "USDC", quote: "CNGN", side: "ASK", fillChain: CHAIN, price: "1500", size: "1000" },
		])
		const { service } = limitOrderService(fakeClient([]), limitOrders)
		const charged = 250n
		;(service as any).contractService.limitOrderSpent = async () => charged * UNIT

		const filler = new IntentFiller(
			[],
			[],
			{},
			{
				getBidValiditySeconds: () => 300,
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

		const fill = () => {
			filler.monitor.emit("orderFilledOnChain", {
				commitment: COMMITMENT,
				filler: OUR_ADDRESS,
				chainId: 8453,
				outputs: [{ token: bytes20ToBytes32(USDC), amount: charged * UNIT }],
				inputs: [{ token: bytes20ToBytes32(ORDERBOOK_FIXTURES.CNGN), amount: charged * 1500n * ONE }],
				complete: true,
			})
			return filler.settlementQueue.onIdle()
		}
		const pricedBy = { limitOrderId: LIMIT_ORDER, payout: (charged * ONE).toString(), take: (charged * 1500n * ONE).toString() }
		return { data, limitOrders, filler, fill, pricedBy, charged }
	}

	it("records the fill against the limit order that priced the bid", async () => {
		const { data, limitOrders, fill, pricedBy, charged } = await settlingFiller()
		await data.bids.store({ commitment: COMMITMENT, bid: BID, success: true, onOrderbook: true, pricedBy })

		await fill()

		const fills = await limitOrders.fills(LIMIT_ORDER)
		expect(fills).toHaveLength(1)
		expect(fills[0]).toMatchObject({ commitment: COMMITMENT, bid: BID, amount: (charged * ONE).toString() })
		expect((await limitOrders.get(LIMIT_ORDER))!.remaining).toBe((750n * ONE).toString())
	})

	it("does not attribute a fill to a bid placed longer ago than its validity window", async () => {
		const { data, limitOrders, fill, pricedBy } = await settlingFiller()
		await data.bids.store({ commitment: COMMITMENT, bid: BID, success: true, onOrderbook: true, pricedBy })
		// Older than bidValiditySeconds plus the discovery pad and the permit skew: its op expired.
		const [row] = await data.bids.byCommitments([COMMITMENT])
		;(data.bids as any).rows.find((stored: { id: number }) => stored.id === row.id).createdAt = sqliteDatetime(
			new Date(Date.now() - 60 * 60 * 1000),
		)

		await fill()

		expect(await limitOrders.fills(LIMIT_ORDER)).toEqual([])
	})
})
