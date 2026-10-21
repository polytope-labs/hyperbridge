import { describe, expect, it, vi } from "vitest"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
import { BRIDGE_BALANCE_TTL_MS, BidVenues, BridgeBalanceGate } from "@/core/bid-venues"
import type { SubmitBidResult } from "@/orderbook/types"

/**
 * Where a bid goes. Every bid is posted to the HyperFX orderbook; it is posted to Hyperbridge as
 * well when the account holds BRIDGE for the deposit, and to the orderbook alone when it does not.
 */

const COMMITMENT = `0x${"c0".repeat(32)}` as HexString
const USER_OP = "0x0102" as HexString
const BID = `0x${"b1".repeat(32)}` as HexString

const ACCEPTED: SubmitBidResult = {
	kind: "accepted",
	bid: {
		commitment: COMMITMENT,
		leg: 0,
		side: "BID",
		fillChain: "EVM-97",
		sourceChain: "EVM-97",
		price: "1500",
		amountIn: "1",
		amountOut: "1500",
		expiresAt: "2026-10-21T00:10:00Z",
	},
}
const LANDED: BidSubmissionResult = { success: true, extrinsicHash: "0xext" as HexString, blockHash: "0xblk" as HexString }

const logger = { trace: vi.fn(), debug: vi.fn(), info: vi.fn(), warn: vi.fn(), error: vi.fn(), fatal: vi.fn() }

function venuesWith(options: {
	hyperbridge?: BidSubmissionResult
	orderbook?: SubmitBidResult | Error
	canPlaceBid?: boolean
}) {
	const submitBid = vi.fn(async () => options.hyperbridge ?? LANDED)
	const postBid = vi.fn(async () => {
		if (options.orderbook instanceof Error) throw options.orderbook
		return options.orderbook ?? ACCEPTED
	})
	const gate = {
		canPlaceBid: vi.fn(async () => options.canPlaceBid ?? true),
		spent: vi.fn(),
		exhausted: vi.fn(),
		freeBalance: 0n,
	}
	const venues = new BidVenues(
		{
			hyperbridge: "hyperbridge" in options && options.hyperbridge === undefined ? undefined : ({ submitBid } as never),
			orderbook: { submitBid: postBid } as never,
			bridgeBalance: gate as never,
		},
		logger as never,
	)
	return { venues, submitBid, postBid, gate }
}

describe("BidVenues", () => {
	it("posts to Hyperbridge and the orderbook together when the account holds BRIDGE", async () => {
		const { venues, submitBid, postBid, gate } = venuesWith({})

		const placed = await venues.place(COMMITMENT, USER_OP, BID)

		expect(submitBid).toHaveBeenCalledWith(COMMITMENT, USER_OP, BID)
		// The same bytes Hyperbridge takes: the signed op, SCALE-encoded.
		expect(postBid).toHaveBeenCalledWith(USER_OP)
		expect(placed).toEqual({ hyperbridge: LANDED, orderbook: ACCEPTED })
		expect(gate.spent).toHaveBeenCalledOnce()
	})

	it("posts to the orderbook alone when the account holds no BRIDGE, and says so once", async () => {
		const { venues, submitBid, postBid, gate } = venuesWith({ canPlaceBid: false })

		expect(await venues.place(COMMITMENT, USER_OP, BID)).toEqual({ orderbook: ACCEPTED })
		expect(await venues.place(COMMITMENT, USER_OP, BID)).toEqual({ orderbook: ACCEPTED })

		expect(submitBid).not.toHaveBeenCalled()
		expect(postBid).toHaveBeenCalledTimes(2)
		expect(gate.spent).not.toHaveBeenCalled()
		expect(logger.warn).toHaveBeenCalledTimes(1)
		expect(logger.warn.mock.calls[0][1]).toMatch(/orderbook alone/)
	})

	it("zeroes the balance reading when Hyperbridge refuses the bid for want of BRIDGE", async () => {
		const refused: BidSubmissionResult = { success: false, error: "Dispatch error: intentsCoprocessor::InsufficientBalance" }
		const { venues, gate } = venuesWith({ hyperbridge: refused })

		const placed = await venues.place(COMMITMENT, USER_OP, BID)

		expect(placed).toEqual({ hyperbridge: refused, orderbook: ACCEPTED })
		expect(gate.exhausted).toHaveBeenCalledOnce()
		expect(gate.spent).not.toHaveBeenCalled()
	})

	it("counts a pooled bid as spent, and any other failure as nothing spent", async () => {
		const pooled: BidSubmissionResult = { success: false, pending: true, error: "timed out in the pool" }
		expect((await venuesWith({ hyperbridge: pooled }).venues.place(COMMITMENT, USER_OP, BID)).hyperbridge).toBe(pooled)

		const failed = venuesWith({ hyperbridge: { success: false, error: "Transaction failed after 3 attempts" } })
		await failed.venues.place(COMMITMENT, USER_OP, BID)
		expect(failed.gate.spent).not.toHaveBeenCalled()
		expect(failed.gate.exhausted).not.toHaveBeenCalled()
	})

	it("renders an orderbook request that got no answer as a failed submission, keeping Hyperbridge's", async () => {
		const { venues } = venuesWith({ orderbook: new Error("Could not reach the orderbook") })

		const placed = await venues.place(COMMITMENT, USER_OP, BID)

		expect(placed.hyperbridge).toEqual(LANDED)
		expect(placed.orderbook).toEqual({
			kind: "failed",
			code: "REQUEST_FAILED",
			message: "Could not reach the orderbook",
			retryable: true,
		})
	})

	it("passes the orderbook's refusal through", async () => {
		const rejected: SubmitBidResult = { kind: "rejected", code: "NO_LIMIT_ORDER", message: "no limit order on the book" }
		const { venues } = venuesWith({ orderbook: rejected })

		expect((await venues.place(COMMITMENT, USER_OP, BID)).orderbook).toBe(rejected)
	})

	it("without a Hyperbridge connection posts to the orderbook alone, without a word about balance", async () => {
		const warnings = logger.warn.mock.calls.length
		const postBid = vi.fn(async () => ACCEPTED)
		const venues = new BidVenues({ orderbook: { submitBid: postBid } as never }, logger as never)

		expect(await venues.place(COMMITMENT, USER_OP, BID)).toEqual({ orderbook: ACCEPTED })
		expect(logger.warn.mock.calls.length).toBe(warnings)
	})
})

describe("BridgeBalanceGate", () => {
	const DEPOSIT = 1_000n
	const EXISTENTIAL = 100n
	const FEE = 10n
	const COST = DEPOSIT + EXISTENTIAL + FEE

	/** A Hyperbridge whose account holds `free`, with the pallet's deposit as a constant or set on chain. */
	function hyperbridgeWith(state: { free: bigint; storedDeposit?: bigint; failReads?: boolean }) {
		const reads = { account: 0 }
		const api = {
			query: {
				system: {
					account: async () => {
						reads.account++
						if (state.failReads) throw new Error("node away")
						return { data: { free: { toString: () => state.free.toString() } } }
					},
				},
				intentsCoprocessor: {
					storageDepositFee: async () => ({ toString: () => (state.storedDeposit ?? 0n).toString() }),
				},
			},
			consts: {
				intentsCoprocessor: { storageDepositFee: { toString: () => DEPOSIT.toString() } },
				balances: { existentialDeposit: { toString: () => EXISTENTIAL.toString() } },
			},
			tx: {
				intentsCoprocessor: {
					placeBid: () => ({ paymentInfo: async () => ({ partialFee: { toString: () => FEE.toString() } }) }),
				},
			},
		}
		const hyperbridge = { queryApi: async () => api, getKeyPair: () => ({ address: "5Filler" }) } as never
		return { hyperbridge, reads }
	}

	it("lets a bid through while the free balance covers the deposit, the existential deposit and the fee", async () => {
		let now = 1_000_000
		const { hyperbridge, reads } = hyperbridgeWith({ free: COST })
		const gate = new BridgeBalanceGate(hyperbridge, logger as never, () => now)

		expect(await gate.canPlaceBid()).toBe(true)
		expect(gate.freeBalance).toBe(COST)
		// One bid spent: the next read has not seen it, so the reading is lowered here.
		gate.spent()
		expect(await gate.canPlaceBid()).toBe(false)
		expect(reads.account).toBe(1)

		// The reading stands for its TTL, then is read again.
		now += BRIDGE_BALANCE_TTL_MS + 1
		expect(await gate.canPlaceBid()).toBe(true)
		expect(reads.account).toBe(2)
	})

	it("prefers the deposit set on chain over the runtime constant", async () => {
		const { hyperbridge } = hyperbridgeWith({ free: COST, storedDeposit: DEPOSIT * 2n })
		expect(await new BridgeBalanceGate(hyperbridge, logger as never).canPlaceBid()).toBe(false)
	})

	it("holds bids back after Hyperbridge refused one for want of balance, until the next read", async () => {
		let now = 1_000_000
		const { hyperbridge } = hyperbridgeWith({ free: COST * 10n })
		const gate = new BridgeBalanceGate(hyperbridge, logger as never, () => now)
		expect(await gate.canPlaceBid()).toBe(true)

		gate.exhausted()
		expect(await gate.canPlaceBid()).toBe(false)
		now += BRIDGE_BALANCE_TTL_MS + 1
		expect(await gate.canPlaceBid()).toBe(true)
	})

	it("lets bids through to Hyperbridge when the balance cannot be read", async () => {
		const { hyperbridge } = hyperbridgeWith({ free: 0n, failReads: true })
		expect(await new BridgeBalanceGate(hyperbridge, logger as never).canPlaceBid()).toBe(true)
	})

	it("reads once for concurrent bids", async () => {
		const { hyperbridge, reads } = hyperbridgeWith({ free: COST * 10n })
		const gate = new BridgeBalanceGate(hyperbridge, logger as never)
		await Promise.all([gate.canPlaceBid(), gate.canPlaceBid(), gate.canPlaceBid()])
		expect(reads.account).toBe(1)
	})
})
