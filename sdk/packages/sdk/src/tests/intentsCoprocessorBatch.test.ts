import { beforeAll, describe, expect, it } from "vitest"
import { cryptoWaitReady } from "@polkadot/util-crypto"
import { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { HexString } from "@/types"

/**
 * Regression tests for the phantom-bid "logs success but never lands" incident.
 *
 * Two bugs compounded:
 *   1. `submitBidWithRetraction` batched [retractBid, placeBid]. Bids are never pruned on-chain,
 *      so a `BidNotFound` on the leading retract skipped `placeBid` — and because the batch
 *      extrinsic is itself `Ok`, the SDK reported success. The next interval then retracted that
 *      never-placed commitment, failing the same way: a self-sustaining cascade.
 *   2. `sendWithTimeout` only inspected `result.dispatchError`, so a `utility.BatchInterrupted`
 *      event (the way a batch surfaces an inner failure) was reported as success.
 *
 * These tests drive the real code paths through a mock ApiPromise (via the public `fromApi`
 * factory) so no node is required.
 */

const BID = "0x1111111111111111111111111111111111111111111111111111111111111111" as HexString
const RETRACT = "0x2222222222222222222222222222222222222222222222222222222222222222" as HexString
const USER_OP = "0xdeadbeef" as HexString

const inBlockStatus = {
	isInBlock: true,
	isFinalized: false,
	isDropped: false,
	isInvalid: false,
	isUsurped: false,
	isFinalityTimeout: false,
	asInBlock: { toHex: () => "0xblockhash" },
	type: "InBlock",
}

/** A `utility.BatchInterrupted { index, error }` event with a module error at the given index. */
const batchInterruptedEvent = (index: number) => ({
	event: {
		section: "utility",
		method: "BatchInterrupted",
		data: [{ toString: () => String(index) }, { isModule: true, asModule: {} }],
	},
})

/**
 * Builds a mock ApiPromise. `events` is what the inBlock callback reports; `capturedBatch`
 * receives the exact call list handed to `utility.batch` so we can assert ordering.
 */
function mockApi(events: unknown[], capturedBatch: { calls: any[] | null }) {
	const sendable = {
		hash: { toHex: () => "0xextrinsichash" },
		signAndSend: (_keyPair: unknown, _opts: unknown, cb: (result: unknown) => void) => {
			queueMicrotask(() => cb({ dispatchError: undefined, status: inBlockStatus, events }))
			return Promise.resolve(() => {})
		},
	}
	return {
		tx: {
			utility: {
				batch: (calls: any[]) => {
					capturedBatch.calls = calls
					return sendable
				},
			},
			intentsCoprocessor: {
				placeBid: (commitment: HexString, userOp: HexString) => ({ call: "placeBid", commitment, userOp }),
				retractBid: (commitment: HexString) => ({ call: "retractBid", commitment }),
			},
		},
		registry: {
			findMetaError: () => ({ section: "intentsCoprocessor", name: "BidNotFound" }),
		},
	} as any
}

describe("submitBidWithRetraction", () => {
	beforeAll(async () => {
		await cryptoWaitReady()
	})

	it("places the new bid FIRST and retracts second, so the bid lands even if the retract fails", async () => {
		const captured: { calls: any[] | null } = { calls: null }
		const coproc = IntentsCoprocessor.fromApi(mockApi([], captured), "//Alice")

		await coproc.submitBidWithRetraction(RETRACT, BID, USER_OP)

		expect(captured.calls?.map((c) => c.call)).toEqual(["placeBid", "retractBid"])
		expect(captured.calls?.[0]).toMatchObject({ call: "placeBid", commitment: BID })
		expect(captured.calls?.[1]).toMatchObject({ call: "retractBid", commitment: RETRACT })
	})

	it("reports SUCCESS when only the trailing retract (index 1) is interrupted — the bid still landed", async () => {
		const captured: { calls: any[] | null } = { calls: null }
		const coproc = IntentsCoprocessor.fromApi(mockApi([batchInterruptedEvent(1)], captured), "//Alice")

		const result = await coproc.submitBidWithRetraction(RETRACT, BID, USER_OP)

		expect(result.success).toBe(true)
		expect(result.blockHash).toBe("0xblockhash")
	})

	it("reports FAILURE when the primary placeBid (index 0) is interrupted — nothing landed", async () => {
		const captured: { calls: any[] | null } = { calls: null }
		const coproc = IntentsCoprocessor.fromApi(mockApi([batchInterruptedEvent(0)], captured), "//Alice")

		const result = await coproc.submitBidWithRetraction(RETRACT, BID, USER_OP)

		expect(result.success).toBe(false)
		expect(result.error).toContain("BidNotFound")
	})

	it("reports SUCCESS for a clean batch with no interruption", async () => {
		const captured: { calls: any[] | null } = { calls: null }
		const coproc = IntentsCoprocessor.fromApi(mockApi([], captured), "//Alice")

		const result = await coproc.submitBidWithRetraction(RETRACT, BID, USER_OP)

		expect(result.success).toBe(true)
	})
})
