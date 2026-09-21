import { describe, expect, it, vi } from "vitest"
import { decodeAddress } from "@polkadot/util-crypto"
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
/** The identifier Hyperbridge files our bid under: keccak256 of its calldata. */
const OUR_BID = `0x${"b1".repeat(32)}` as HexString
/** Alice and Bob, as SS58: the filler's own Hyperbridge account, and another solver's. */
const OUR_SUBSTRATE_ADDRESS = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
const SOMEONE_ELSE = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty"

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
		// The pallet files our bid under an identifier the row does not record, so the filler reads
		// it back from storage for its own account before retracting.
		const keyPair = { publicKey: decodeAddress(OUR_SUBSTRATE_ADDRESS) }
		const getBidStorageEntries = vi.fn(async () => [
			{ commitment: COMMITMENT, filler: OUR_SUBSTRATE_ADDRESS, bid: OUR_BID, deposit: 1n },
			{ commitment: COMMITMENT, filler: SOMEONE_ELSE, bid: `0x${"b2".repeat(32)}`, deposit: 1n },
		])
		;(filler as any).hyperbridge = Promise.resolve({
			retractBid,
			getBidStorageEntries,
			getKeyPair: () => keyPair,
		})

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
		// Our own bid, under the identifier it holds, and nobody else's.
		expect(retractBid).toHaveBeenCalledTimes(1)
		expect(retractBid).toHaveBeenCalledWith(COMMITMENT, OUR_BID)
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
})
