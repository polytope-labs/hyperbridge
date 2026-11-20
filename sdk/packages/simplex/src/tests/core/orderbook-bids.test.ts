import { describe, expect, it, vi } from "vitest"
import { bytes20ToBytes32, type HexString } from "@hyperbridge/sdk"
import { BridgeBalanceGate, type BidPlacer } from "@/core/bid-venues"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import type { SubmitBidResult } from "@/orderbook/types"
import { CacheService } from "@/services/CacheService"
import type { FillResult } from "@/strategies/base"
import { stubOrderScanner } from "../helpers/stub-scanner"
import { fakeClient, limitOrderService, limitOrderStore, ORDERBOOK_FIXTURES } from "../helpers/limit-orders"

/**
 * A bid the orderbook holds is a bid a swapper can execute, whatever Hyperbridge said: a filler
 * without BRIDGE for the deposit bids on the orderbook alone. Such a bid is recorded as live
 * there, counted as a placed bid, left alone by the retraction sweep (there is no deposit to
 * reclaim and the orderbook serves it until its TTL), and a fill it wins is settled against the
 * limit order that priced it like any other.
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

describe("a bid the orderbook holds", () => {
	/**
	 * A filler whose strategy answers every order with `result`, or, given `through`, places
	 * one signed op through the venues the filler hands it and answers with what they said.
	 */
	function fillerWith(result: Omit<FillResult, "commitment" | "bid"> | { through: true }) {
		const cache = new CacheService()
		const strategy = {
			name: "test",
			canFill: async () => true,
			calculateProfitability: async () => 1,
			executeOrder: async (_order: unknown, venues?: BidPlacer) => {
				if (!("through" in result)) return { ...result, commitment: COMMITMENT, bid: BID }
				const placed = await venues!.place(COMMITMENT, USER_OP, BID)
				return {
					success: placed.hyperbridge?.success === true,
					txHash: placed.hyperbridge?.extrinsicHash,
					error: placed.hyperbridge?.error,
					orderbook: placed.orderbook,
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
				getHyperbridgeWsUrl: () => undefined,
				getSubstratePrivateKey: () => undefined,
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
		const executed: { success: boolean; error?: string }[] = []
		filler.monitor.on("orderExecuted", (event: { success: boolean; error?: string }) => executed.push(event))
		const run = async (id = "0xorder") => {
			;(filler as any).executeOrder({ id, source: "EVM-1", destination: "EVM-1" }, strategy, true, { toNumber: () => 1 }, 1)
			await (filler as any).chainQueues.get(1).onIdle()
		}
		return { filler, bids, executed, run }
	}

	/** A Hyperbridge whose account holds `free`, for the gate to read. */
	function hyperbridgeHolding(free: bigint, submitBid = vi.fn(async () => ({ success: true, extrinsicHash: "0xext" }))) {
		const api = {
			query: {
				system: { account: async () => ({ data: { free: { toString: () => free.toString() } } }) },
				intentsCoprocessor: { storageDepositFee: async () => ({ toString: () => "0" }) },
			},
			consts: {
				intentsCoprocessor: { storageDepositFee: { toString: () => "1000" } },
				balances: { existentialDeposit: { toString: () => "100" } },
			},
			tx: { intentsCoprocessor: { placeBid: () => ({ paymentInfo: async () => ({ partialFee: { toString: () => "10" } }) }) } },
		}
		return { queryApi: async () => api, getKeyPair: () => ({ address: "5Filler" }), submitBid }
	}

	it("warns once across bids while the account holds no BRIDGE, and posts each to the orderbook", async () => {
		const { filler, bids, run } = fillerWith({ through: true })
		const hyperbridge = hyperbridgeHolding(0n)
		const postBid = vi.fn(async () => ACCEPTED)
		;(filler as any).hyperbridge = Promise.resolve(hyperbridge)
		;(filler as any).bridgeBalance = new BridgeBalanceGate(hyperbridge as never, logger as never)
		filler.setOrderbook({ submitBid: postBid } as never)
		logger.warn.mockClear()

		await run("0xorder-1")
		await run("0xorder-2")

		expect(postBid).toHaveBeenCalledTimes(2)
		expect(hyperbridge.submitBid).not.toHaveBeenCalled()
		expect(logger.warn.mock.calls.filter((call) => /orderbook alone/.test(call[1]))).toHaveLength(1)
		expect((await bids.byCommitments([COMMITMENT])).map((row) => row.onOrderbook)).toEqual([true, true])
	})

	it("without a Hyperbridge key still delegates the SolverAccount and bids on the orderbook alone", async () => {
		const { filler, bids, executed, run } = fillerWith({ through: true })
		const setupDelegationOnChains = vi.fn(async () => ({ success: true, results: { "EVM-1": true } }))
		;(filler as any).delegation = () => ({ setupDelegationOnChains })
		const postBid = vi.fn(async () => ACCEPTED)
		filler.setOrderbook({ submitBid: postBid } as never)

		await (filler as any).setupSolverSelection(["EVM-1"], { requireAll: true })
		expect(setupDelegationOnChains).toHaveBeenCalledWith(["EVM-1"])

		await run()
		expect(postBid).toHaveBeenCalledWith(USER_OP)
		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({ success: false, onOrderbook: true })
		expect(executed).toEqual([expect.objectContaining({ success: true })])
	})

	it("is stored as live on the orderbook, counted as placed, and left to expire there", async () => {
		const { bids, executed, run } = fillerWith({ success: false, orderbook: ACCEPTED })
		await run()

		const row = await bids.byCommitment(COMMITMENT)
		expect(row).toMatchObject({ bid: BID, success: false, pending: false, onOrderbook: true, retracted: false })
		expect(executed).toEqual([expect.objectContaining({ success: true, error: undefined })])
		// No deposit on Hyperbridge, so nothing for the sweep to reclaim.
		expect(await bids.expiredUnretracted(0)).toEqual([])
		expect(await bids.unretractedReclaimable()).toEqual([])
	})

	it("records a bid Hyperbridge took as well, with the deposit to reclaim", async () => {
		const { bids, run } = fillerWith({ success: true, txHash: "0xext", orderbook: ACCEPTED })
		await run()

		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({ success: true, onOrderbook: true, extrinsicHash: "0xext" })
		expect(await bids.unretractedReclaimable()).toHaveLength(1)
	})

	it("is a failed bid when the orderbook refused it and Hyperbridge did not take it", async () => {
		const { bids, executed, run } = fillerWith({
			success: false,
			error: "Dispatch error: intentsCoprocessor::InsufficientBalance",
			orderbook: { kind: "rejected", code: "NO_LIMIT_ORDER", message: "no limit order" },
		})
		await run()

		expect(await bids.byCommitment(COMMITMENT)).toMatchObject({ success: false, onOrderbook: false })
		expect(executed).toEqual([
			expect.objectContaining({ success: false, error: "Dispatch error: intentsCoprocessor::InsufficientBalance" }),
		])
	})

	it("settles the fill it wins against the limit order that priced it", async () => {
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
				getHyperbridgeWsUrl: () => undefined,
				getSubstratePrivateKey: () => undefined,
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

		// Posted to the orderbook alone: no BRIDGE for the deposit.
		await data.bids.store({
			commitment: COMMITMENT,
			bid: BID,
			success: false,
			onOrderbook: true,
			pricedBy: { limitOrderId: LIMIT_ORDER, payout: (charged * ONE).toString(), take: (charged * 1500n * ONE).toString() },
		})

		filler.monitor.emit("orderFilledOnChain", {
			commitment: COMMITMENT,
			filler: OUR_ADDRESS,
			chainId: 8453,
			outputs: [{ token: bytes20ToBytes32(USDC), amount: charged * UNIT }],
			inputs: [{ token: bytes20ToBytes32(ORDERBOOK_FIXTURES.CNGN), amount: charged * 1500n * ONE }],
			complete: true,
		})
		await filler.settlementQueue.onIdle()

		const fills = await limitOrders.fills(LIMIT_ORDER)
		expect(fills).toHaveLength(1)
		expect(fills[0]).toMatchObject({ commitment: COMMITMENT, bid: BID, amount: (charged * ONE).toString() })
		expect((await limitOrders.get(LIMIT_ORDER))!.remaining).toBe((750n * ONE).toString())
	})
})
