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

describe("IntentFiller bid retraction", () => {
	function build(results: BidSubmissionResult[]) {
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
			undefined,
			bidStorage,
		)
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
		await bidStorage.store({ commitment: COMMITMENT, success: true })

		await orderFilled(filler, COMMITMENT)

		const bid = await bidStorage.byCommitment(COMMITMENT)
		expect(bid!.retracted).toBe(true)
		expect(bid!.retractExtrinsicHash).toBe("0xretract")
		expect(retractBid).toHaveBeenCalledTimes(1)
	})

	it("treats BidNotFound as terminal: marks retracted so the sweep never re-attempts", async () => {
		const { filler, bidStorage, retractBid } = build([
			{ success: false, error: "Dispatch error: intentsCoprocessor::BidNotFound" },
		])
		await bidStorage.store({ commitment: COMMITMENT, success: true })

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
		await bidStorage.store({ commitment: COMMITMENT, success: true })

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
		await bidStorage.store({ commitment: COMMITMENT, success: true })

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
		await bidStorage.store({ commitment: COMMITMENT, success: true })

		// OrderFilled and a sweep race to enqueue the same commitment.
		;(filler as any).handleOrderFilledOnChain(COMMITMENT, OTHER_FILLER, 8453)
		;(filler as any).enqueueRetraction(COMMITMENT)
		await (filler as any).retractionQueue.onIdle()

		expect((await bidStorage.byCommitment(COMMITMENT))!.retracted).toBe(true)
		expect(retractBid).toHaveBeenCalledTimes(1)
	})
})
