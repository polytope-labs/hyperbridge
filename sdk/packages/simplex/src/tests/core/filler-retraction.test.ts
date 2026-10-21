import { describe, expect, it, vi } from "vitest"
import type { BidSubmissionResult, HexString } from "@hyperbridge/sdk"
import { IntentFiller } from "@/core/filler"
import { MemoryDataStore } from "@/data/memory"
import { stubOrderScanner } from "../helpers/stub-scanner"

/**
 * Regression tests for #1074: the filler's handling of retraction outcomes.
 *
 * The incident: a retraction's watch handle didn't confirm, the SDK's retry bounced off its own
 * pooled copy (1014), the pooled original then executed, and the late duplicate's `BidNotFound`
 * was recorded as an ERROR. Because only observed successes ran `markBidAsRetracted`, the bid
 * stayed "unretracted" in SQLite and the sweep re-retracted it every cycle after the TTL —
 * `BidNotFound` every 5 minutes, forever.
 *
 * The rules under test, driven through handleOrderFilledOnChain with a stubbed coprocessor:
 *   - `BidNotFound` is terminal: nothing is left to reclaim, mark retracted (no zombie sweeps).
 *   - `pending` (in-flight) is not failure: leave unretracted; the bid is dead, so the sweep
 *     re-checks next cycle and the follow-up's BidNotFound closes it out.
 *   - OrderFilled marks the bid dead, so failed retractions retry on the next sweep cycle
 *     regardless of the bid's age.
 *   - A commitment already retracted at dequeue time is skipped, not re-submitted.
 */

const COMMITMENT = "0x4380111111111111111111111111111111111111111111111111111111114818" as HexString
const OUR_ADDRESS = "0xAAAA00000000000000000000000000000000AAAA" as HexString
const OTHER_FILLER = "0xBBBB00000000000000000000000000000000BBBB"
const HOUR_MS = 60 * 60 * 1000
/** The identifier Hyperbridge files our bid under, keccak256 of its calldata, recorded at placement. */
const OUR_BID = `0x${"b1".repeat(32)}` as HexString
const OUR_OTHER_BID = `0x${"b2".repeat(32)}` as HexString

describe("IntentFiller bid retraction", () => {
	function build(results: BidSubmissionResult[], rebalancingService?: { rebalancePortfolio(): Promise<unknown> }) {
		const bidStorage = new MemoryDataStore().bids

		const retractBid = vi.fn(async (): Promise<BidSubmissionResult> => {
			const next = results.shift()
			if (!next) throw new Error("unexpected retractBid call")
			return next
		})

		const configService = {
			getHyperbridgeWsUrl: () => undefined,
			getSubstratePrivateKey: () => undefined,
		} as any

		const filler = new IntentFiller(
			[],
			[],
			{},
			configService,
			{} as any, // ChainClientManager — unused with no chains configured
			{} as any, // ContractInteractionService — unused on the retraction path
			{ address: OUR_ADDRESS } as any,
			{ orders: stubOrderScanner() },
			rebalancingService as any,
			bidStorage,
		)
		// Retraction names each bid by the identifier recorded when it was placed; nothing is read
		// back from Hyperbridge.
		;(filler as any).hyperbridge = Promise.resolve({ retractBid })

		return { filler, bidStorage, retractBid }
	}

	async function orderFilled(filler: IntentFiller, commitment: HexString): Promise<void> {
		// Awaited: the handler reads the bid store before it can enqueue anything, so
		// `onIdle()` on a queue nothing has reached yet would resolve immediately. In
		// production the monitor's listener fires this without awaiting and the enqueue
		// lands a microtask later; only the test needs the ordering pinned.
		await (filler as any).handleOrderFilledOnChain(commitment, OTHER_FILLER, 8453)
		await (filler as any).retractionQueue.onIdle()
	}

	it("marks the bid retracted on a successful retraction", async () => {
		const { filler, bidStorage, retractBid } = build([
			{ success: true, extrinsicHash: "0xretract" as HexString, blockHash: "0xblock" as HexString },
		])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

		await orderFilled(filler, COMMITMENT)

		const bid = await bidStorage.byCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(true)
		expect(bid!.retractExtrinsicHash).toBe("0xretract")
		// Our own bid, under the identifier it holds, and nobody else's.
		expect(retractBid).toHaveBeenCalledTimes(1)
		expect(retractBid).toHaveBeenCalledWith(COMMITMENT, OUR_BID)
	})

	it("treats BidNotFound as terminal: marks retracted so the sweep never re-attempts", async () => {
		const { filler, bidStorage, retractBid } = build([
			{ success: false, error: "Dispatch error: intentsCoprocessor::BidNotFound" },
		])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

		await orderFilled(filler, COMMITMENT)

		const bid = await bidStorage.byCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(true)
		expect(bid!.retractExtrinsicHash).toBeNull()
		expect(await bidStorage.expiredUnretracted(0)).toHaveLength(0)
		expect(retractBid).toHaveBeenCalledTimes(1)
	})

	it("leaves an in-flight retraction unretracted, then closes it out via the sweep's BidNotFound", async () => {
		const { filler, bidStorage, retractBid } = build([
			{
				success: false,
				pending: true,
				error: "Transaction timed out after 30000ms while in the transaction pool",
			},
			{ success: false, error: "Dispatch error: intentsCoprocessor::BidNotFound" },
		])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

		await orderFilled(filler, COMMITMENT)

		// In flight: not retracted yet, but dead — due on the next sweep cycle despite its age.
		let bid = await bidStorage.byCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(false)
		expect(bid!.dead).toBe(true)
		expect((await bidStorage.expiredUnretracted(HOUR_MS)).map((b) => b.commitment)).toEqual([COMMITMENT])

		// Next cycle: the pooled original landed meanwhile, so the sweep's attempt sees
		// BidNotFound and marks the bid retracted. The zombie loop from #1074 ends here.
		await (filler as any).sweepExpiredBids(HOUR_MS)
		await (filler as any).retractionQueue.onIdle()

		bid = await bidStorage.byCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(true)
		expect(retractBid).toHaveBeenCalledTimes(2)
	})

	it("marks the bid dead on OrderFilled so a failed retraction retries next sweep, not after the TTL", async () => {
		const { filler, bidStorage } = build([{ success: false, error: "Transaction failed after 3 attempts" }])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

		await orderFilled(filler, COMMITMENT)

		const bid = await bidStorage.byCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(false)
		expect(bid!.dead).toBe(true)
		// Minutes old at most, yet already due for the next sweep.
		expect((await bidStorage.expiredUnretracted(HOUR_MS)).map((b) => b.commitment)).toEqual([COMMITMENT])
	})

	it("skips a queued duplicate whose bid was retracted by the time it dequeues", async () => {
		const { filler, bidStorage, retractBid } = build([
			{ success: true, extrinsicHash: "0xretract" as HexString, blockHash: "0xblock" as HexString },
		])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

		// OrderFilled and a sweep race to enqueue the same commitment.
		;(filler as any).handleOrderFilledOnChain(COMMITMENT, OTHER_FILLER, 8453)
		;(filler as any).enqueueRetraction(COMMITMENT)
		await (filler as any).retractionQueue.onIdle()

		expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(true)
		expect(retractBid).toHaveBeenCalledTimes(1)
	})

	it("retracts a bid still in the pool by the identifier it was placed under", async () => {
		// Its place_bid has not landed, so Hyperbridge's storage does not hold it yet. The
		// retraction names it from the row regardless, goes out after it from the same account, and
		// so reclaims its deposit once it lands, instead of marking it retracted and abandoning it.
		const { filler, bidStorage, retractBid } = build([
			{ success: true, extrinsicHash: "0xretract" as HexString, blockHash: "0xblock" as HexString },
		])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: false, pending: true })

		await orderFilled(filler, COMMITMENT)

		expect(retractBid).toHaveBeenCalledWith(COMMITMENT, OUR_BID)
		expect((await bidStorage.byCommitment(COMMITMENT))!.retractExtrinsicHash).toBe("0xretract")
	})

	it("retracts every bid placed on the commitment, past one already gone", async () => {
		const { filler, bidStorage, retractBid } = build([
			{ success: false, error: "Dispatch error: intentsCoprocessor::BidNotFound" },
			{ success: true, extrinsicHash: "0xretract" as HexString, blockHash: "0xblock" as HexString },
		])
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
		await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })

		await orderFilled(filler, COMMITMENT)

		expect(retractBid.mock.calls.map((call) => (call as unknown[])[1]).sort()).toEqual([OUR_BID, OUR_OTHER_BID])
		expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(true)
	})

	it("marks a bid with no recorded identifier retracted without calling Hyperbridge", async () => {
		// A row from before bids carried an identifier names nothing the current calls can retract.
		const { filler, bidStorage, retractBid } = build([])
		await bidStorage.store({ commitment: COMMITMENT, success: true })

		await orderFilled(filler, COMMITMENT)

		expect(retractBid).not.toHaveBeenCalled()
		expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(true)
	})

	it("reports work that stop drains without counting unmatched pending retractions", () => {
		const { filler } = build([])
		;(filler as any).globalQueue = { size: 2, pending: 1 }
		;(filler as any).chainQueues = new Map([
			[1, { size: 3, pending: 1 }],
			[2, { size: 4, pending: 2 }],
		])
		;(filler as any).pendingRetractions = new Set([COMMITMENT])
		;(filler as any).retractionQueue = { size: 5, pending: 1 }

		expect(filler.getWorkSnapshot()).toEqual({
			queuedEvaluations: 2,
			evaluating: 1,
			queuedFills: 7,
			activeFills: 3,
			retractions: 6,
			rebalancing: 0,
		})
	})

	it("reports and drains a rebalance already in flight before stopping", async () => {
		vi.useFakeTimers()
		let finishRebalance!: (value: { success: boolean; transfers: never[]; executedTransfers: never[] }) => void
		const rebalancePortfolio = vi.fn(
			() =>
				new Promise<{ success: boolean; transfers: never[]; executedTransfers: never[] }>((resolve) => {
					finishRebalance = resolve
				}),
		)
		const { filler } = build([], { rebalancePortfolio })
		try {
			filler.start()
			await vi.advanceTimersByTimeAsync(30_000)
			expect(rebalancePortfolio).toHaveBeenCalledOnce()
			expect(filler.getWorkSnapshot().rebalancing).toBe(1)

			let stopped = false
			const stop = filler.stop().then(() => {
				stopped = true
			})
			await Promise.resolve()
			expect(stopped).toBe(false)

			finishRebalance({ success: true, transfers: [], executedTransfers: [] })
			await stop
			expect(filler.getWorkSnapshot().rebalancing).toBe(0)
		} finally {
			vi.useRealTimers()
		}
	})

	it("does not start a second rebalance while one is still running", async () => {
		vi.useFakeTimers()
		let finishRebalance!: (value: { success: boolean; transfers: never[]; executedTransfers: never[] }) => void
		const rebalancePortfolio = vi.fn(
			() =>
				new Promise<{ success: boolean; transfers: never[]; executedTransfers: never[] }>((resolve) => {
					finishRebalance = resolve
				}),
		)
		const { filler } = build([], { rebalancePortfolio })
		try {
			filler.start()
			await vi.advanceTimersByTimeAsync(30_000)
			expect(rebalancePortfolio).toHaveBeenCalledTimes(1)

			// A CEX leg can run for half an hour; the 5-minute ticks must not stack
			// behind it and plan the same deficit twice.
			await vi.advanceTimersByTimeAsync(5 * 60 * 1000)
			await vi.advanceTimersByTimeAsync(5 * 60 * 1000)
			expect(rebalancePortfolio).toHaveBeenCalledTimes(1)

			finishRebalance({ success: true, transfers: [], executedTransfers: [] })
			await vi.advanceTimersByTimeAsync(5 * 60 * 1000)
			expect(rebalancePortfolio).toHaveBeenCalledTimes(2)
		} finally {
			vi.useRealTimers()
		}
	})

	describe("one retraction pass", () => {
		it("closes only the rows it read, not one stored while its retraction was in flight", async () => {
			const { filler, bidStorage, retractBid } = build([])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

			let finish!: (result: BidSubmissionResult) => void
			retractBid.mockImplementationOnce(
				() =>
					new Promise<BidSubmissionResult>((resolve) => {
						finish = resolve
					}),
			)
			;(filler as any).enqueueRetraction(COMMITMENT)
			await vi.waitFor(() => expect(retractBid).toHaveBeenCalledTimes(1))

			// A second bid lands on the order while the first one's retraction is out.
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			finish({ success: true, extrinsicHash: "0x01" as HexString })
			await (filler as any).retractionQueue.onIdle()

			const rows = await bidStorage.byCommitments([COMMITMENT])
			expect(rows.find((row) => row.bid === OUR_BID)!.retracted).toBe(true)
			// Never sent a retraction, so it must not be recorded as one: its deposit is still locked.
			expect(rows.find((row) => row.bid === OUR_OTHER_BID)!.retracted).toBe(false)

			// The next pass reaches it.
			retractBid.mockResolvedValueOnce({ success: true, extrinsicHash: "0x02" as HexString })
			;(filler as any).enqueueRetraction(COMMITMENT)
			await (filler as any).retractionQueue.onIdle()
			expect(retractBid).toHaveBeenLastCalledWith(COMMITMENT, OUR_OTHER_BID)
			expect((await bidStorage.byCommitments([COMMITMENT])).every((row) => row.retracted)).toBe(true)
		})

		it("retracts only the named bids and leaves the others on the order standing", async () => {
			const { filler, bidStorage, retractBid } = build([{ success: true, extrinsicHash: "0x01" as HexString }])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			;(filler as any).enqueueRetraction(COMMITMENT, { bids: [OUR_OTHER_BID] })
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).toHaveBeenCalledTimes(1)
			expect(retractBid).toHaveBeenCalledWith(COMMITMENT, OUR_OTHER_BID)
			const rows = await bidStorage.byCommitments([COMMITMENT])
			expect(rows.find((row) => row.bid === OUR_BID)!.retracted).toBe(false)
			expect(rows.find((row) => row.bid === OUR_OTHER_BID)!.retracted).toBe(true)
		})

		it("retracts only the named rows, and skips a pass whose rows are all retracted already", async () => {
			const { filler, bidStorage, retractBid } = build([{ success: true, extrinsicHash: "0x01" as HexString }])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			const second = (await bidStorage.byCommitments([COMMITMENT])).find((row) => row.bid === OUR_OTHER_BID)!
			;(filler as any).enqueueRetraction(COMMITMENT, { rows: [second.id] })
			await (filler as any).retractionQueue.onIdle()
			;(filler as any).enqueueRetraction(COMMITMENT, { rows: [second.id] })
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).toHaveBeenCalledTimes(1)
			expect(retractBid).toHaveBeenCalledWith(COMMITMENT, OUR_OTHER_BID)
			const rows = await bidStorage.byCommitments([COMMITMENT])
			expect(rows.find((row) => row.bid === OUR_BID)!.retracted).toBe(false)
		})

		it("closes the bid that retracted and flags the one that failed dead, by row", async () => {
			const { filler, bidStorage, retractBid } = build([
				{ success: true, extrinsicHash: "0x01" as HexString },
				{ success: false, error: "Transaction failed after 3 attempts" },
			])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			// Another limit order's bid on the same swap order, outside the pass.
			await bidStorage.store({ commitment: COMMITMENT, bid: `0x${"b3".repeat(32)}`, success: true })
			const targeted = (await bidStorage.byCommitments([COMMITMENT]))
				.filter((row) => row.bid === OUR_BID || row.bid === OUR_OTHER_BID)
				.map((row) => row.id)
			;(filler as any).enqueueRetraction(COMMITMENT, { rows: targeted })
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).toHaveBeenCalledTimes(2)
			const rows = await bidStorage.byCommitments([COMMITMENT])
			const retracted = rows.filter((row) => row.retracted)
			expect(retracted).toHaveLength(1)
			expect(retracted[0].bid).toBe((retractBid.mock.calls[0] as unknown[])[1])
			expect(retracted[0].retractExtrinsicHash).toBe("0x01")
			// Only the bid that failed is flagged, so the next sweep retries it and nothing else.
			const dead = rows.filter((row) => row.dead)
			expect(dead.map((row) => row.bid)).toEqual([(retractBid.mock.calls[1] as unknown[])[1]])
			expect((await bidStorage.expiredUnretracted(HOUR_MS)).map((row) => row.id)).toEqual(dead.map((row) => row.id))
		})

		it("stops at a pooled retraction and leaves the rest of the pass open", async () => {
			const { filler, bidStorage, retractBid } = build([
				{ success: false, pending: true, error: "Transaction timed out while in the transaction pool" },
			])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			;(filler as any).enqueueRetraction(COMMITMENT)
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).toHaveBeenCalledTimes(1)
			const rows = await bidStorage.byCommitments([COMMITMENT])
			expect(rows.every((row) => !row.retracted && row.dead)).toBe(true)
		})

		it("closes a legacy row with no identifier alongside the bids it retracts", async () => {
			const { filler, bidStorage, retractBid } = build([{ success: true, extrinsicHash: "0x01" as HexString }])
			await bidStorage.store({ commitment: COMMITMENT, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			;(filler as any).enqueueRetraction(COMMITMENT)
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).toHaveBeenCalledTimes(1)
			expect((await bidStorage.byCommitments([COMMITMENT])).every((row) => row.retracted)).toBe(true)
		})

		it("retracts the bids still standing when the newest bid on the order was retracted on its own", async () => {
			const { filler, bidStorage, retractBid } = build([
				{ success: true, extrinsicHash: "0x01" as HexString },
				{ success: true, extrinsicHash: "0x02" as HexString },
			])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			;(filler as any).enqueueRetraction(COMMITMENT, { bids: [OUR_OTHER_BID] })
			await (filler as any).retractionQueue.onIdle()

			await orderFilled(filler, COMMITMENT)

			expect(retractBid).toHaveBeenLastCalledWith(COMMITMENT, OUR_BID)
			expect((await bidStorage.byCommitments([COMMITMENT])).every((row) => row.retracted)).toBe(true)
		})

		it("sweeps only the due rows of a commitment, not a fresh bid beside them", async () => {
			const { filler, bidStorage, retractBid } = build([{ success: true, extrinsicHash: "0x01" as HexString }])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_OTHER_BID, success: true })
			const due = (await bidStorage.byCommitments([COMMITMENT])).find((row) => row.bid === OUR_BID)!
			await bidStorage.markDead(COMMITMENT, [due.id])

			expect(await (filler as any).sweepExpiredBids(HOUR_MS)).toBe(1)
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).toHaveBeenCalledTimes(1)
			expect(retractBid).toHaveBeenCalledWith(COMMITMENT, OUR_BID)
			const rows = await bidStorage.byCommitments([COMMITMENT])
			expect(rows.find((row) => row.bid === OUR_OTHER_BID)!.retracted).toBe(false)
		})

		it("returns early when Hyperbridge is already disconnected", async () => {
			const { filler, bidStorage, retractBid } = build([])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			;(filler as any).hyperbridge = undefined
			;(filler as any).enqueueRetraction(COMMITMENT)
			await (filler as any).retractionQueue.onIdle()

			expect(retractBid).not.toHaveBeenCalled()
			expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(false)
		})
	})

	describe("stop()", () => {
		it("drains a queued retraction before it disconnects from Hyperbridge", async () => {
			const { filler, bidStorage, retractBid } = build([])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
			const order: string[] = []
			let finish!: (result: BidSubmissionResult) => void
			retractBid.mockImplementationOnce(
				() =>
					new Promise<BidSubmissionResult>((resolve) => {
						finish = (result) => {
							order.push("retracted")
							resolve(result)
						}
					}),
			)
			;(filler as any).hyperbridgeApi = { disconnect: vi.fn(async () => order.push("disconnected")) }
			;(filler as any).enqueueRetraction(COMMITMENT)
			const stopped = filler.stop()
			await vi.waitFor(() => expect(retractBid).toHaveBeenCalledTimes(1))
			expect(order).toEqual([])

			finish({ success: true, extrinsicHash: "0x01" as HexString })
			await stopped

			expect(order).toEqual(["retracted", "disconnected"])
			expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(true)
		})

		it("disconnects anyway once a stuck retraction outlasts the drain timeout", async () => {
			vi.useFakeTimers()
			try {
				const { filler, bidStorage, retractBid } = build([])
				await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })
				retractBid.mockImplementationOnce(() => new Promise<BidSubmissionResult>(() => {}))
				const disconnect = vi.fn(async () => {})
				;(filler as any).hyperbridgeApi = { disconnect }
				;(filler as any).enqueueRetraction(COMMITMENT)
				let stopped = false
				const stop = filler.stop().then(() => {
					stopped = true
				})
				await vi.advanceTimersByTimeAsync(29_000)
				expect(stopped).toBe(false)
				expect(disconnect).not.toHaveBeenCalled()

				await vi.advanceTimersByTimeAsync(1_000)
				await stop
				expect(disconnect).toHaveBeenCalledOnce()
				// Left for the next run's sweep, not marked as if it had been retracted.
				expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(false)
			} finally {
				vi.useRealTimers()
			}
		})
	})

	describe("a rival's fill", () => {
		async function observe(filler: IntentFiller, complete: boolean): Promise<void> {
			;(filler as any).monitor.emit("orderFillObserved", {
				commitment: COMMITMENT,
				filler: OTHER_FILLER,
				chainId: 8453,
				ours: false,
				complete,
			})
			// The listener reads the bid store before anything is enqueued; let it get there.
			await new Promise((resolve) => setTimeout(resolve, 0))
			await (filler as any).retractionQueue.onIdle()
		}

		it("retracts our bid once a rival completes the order", async () => {
			const { filler, bidStorage, retractBid } = build([{ success: true, extrinsicHash: "0x01" as HexString }])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

			await observe(filler, true)

			expect(retractBid).toHaveBeenCalledTimes(1)
			expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(true)
		})

		it("leaves our bid standing after a rival's partial fill, which it may still complete", async () => {
			const { filler, bidStorage, retractBid } = build([])
			await bidStorage.store({ commitment: COMMITMENT, bid: OUR_BID, success: true })

			await observe(filler, false)

			expect(retractBid).not.toHaveBeenCalled()
			expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(false)
		})

		it("does nothing for an order we never bid on", async () => {
			const { filler, retractBid } = build([])

			await observe(filler, true)

			expect(retractBid).not.toHaveBeenCalled()
			expect((filler as any).pendingRetractions.size).toBe(0)
		})
	})
})
