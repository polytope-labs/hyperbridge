import { beforeAll, describe, expect, it, vi } from "vitest"
import { cryptoWaitReady } from "@polkadot/util-crypto"
import { INCLUSION_TIMEOUT_MS, IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { HexString } from "@/types"

/**
 * One interval's phantom bids ride in a single `utility.force_batch`.
 *
 * `force_batch`, not `batch`: `batch` stops at the first failing call, so one rejected bid — a
 * closed window, a duplicate, an insufficient deposit — would silently drop every bid after it.
 * `force_batch` runs them all, and reports each outcome as an `ItemCompleted` or `ItemFailed`
 * event. Those events carry no index: pallet-utility emits exactly one per call, in call order,
 * so attribution is positional. These tests pin that mapping, because an off-by-one here records
 * a bid as landed when it was not (or the reverse, which strands its deposit).
 */

const COMMITMENT_A = `0x${"aa".repeat(32)}` as HexString
const COMMITMENT_B = `0x${"bb".repeat(32)}` as HexString
const COMMITMENT_C = `0x${"cc".repeat(32)}` as HexString
const PREV_A = `0x${"11".repeat(32)}` as HexString
const USER_OP = "0xdeadbeef" as HexString

const inBlockStatus = {
	isFuture: false,
	isReady: false,
	isBroadcast: false,
	isRetracted: false,
	isInBlock: true,
	isFinalized: false,
	isDropped: false,
	isInvalid: false,
	isUsurped: false,
	isFinalityTimeout: false,
	asInBlock: { toHex: () => "0xblockhash" },
	type: "InBlock",
}

const itemCompleted = { event: { section: "utility", method: "ItemCompleted", data: [] } }
/** `ItemFailed` carries the DispatchError only — no index. */
const itemFailed = (name: string) => ({
	event: {
		section: "utility",
		method: "ItemFailed",
		data: [{ isModule: true, asModule: { name } }],
	},
})
const unrelatedEvent = { event: { section: "balances", method: "Transfer", data: [] } }

/** A terminal pool status: the extrinsic is gone and never reached a block. */
const droppedStatus = { ...inBlockStatus, isInBlock: false, isDropped: true, type: "Dropped" }

/** A pool-entry status with no follow-up: the extrinsic is stuck in the pool. */
const readyStatus = { ...inBlockStatus, isInBlock: false, isReady: true, type: "Ready" }

/** The nonce the batch reports once signed, which a replacement must be pinned to. */
const SIGNED_NONCE = 4

/**
 * A mock node that reports `events` for the batch it is handed, and records the calls that went
 * into it. `dropped` makes the extrinsic die in the pool instead of reaching a block.
 * `stallAttempts` makes that many leading attempts reach the pool and then go silent, which is what
 * a submission that never gets included looks like from here.
 */
function mockNode(options: { events?: unknown[]; dropped?: boolean; stallAttempts?: number } = {}) {
	const batches: { call: string; args: unknown[] }[][] = []
	const attempts: { tip?: bigint; nonce?: number }[] = []

	const sendable = {
		hash: { toHex: () => "0xextrinsichash" },
		nonce: { toNumber: () => SIGNED_NONCE },
		signAndSend: (_keyPair: unknown, opts: { tip?: bigint; nonce?: number }, cb: (result: unknown) => void) => {
			attempts.push(opts)
			const stalls = attempts.length <= (options.stallAttempts ?? 0)
			queueMicrotask(() =>
				cb({
					dispatchError: undefined,
					status: stalls ? readyStatus : options.dropped ? droppedStatus : inBlockStatus,
					events: stalls ? [] : [unrelatedEvent, ...(options.events ?? [])],
				}),
			)
			return Promise.resolve(() => {})
		},
	}

	const api = {
		isConnected: true,
		tx: {
			utility: {
				forceBatch: (calls: { call: string; args: unknown[] }[]) => {
					batches.push(calls)
					return sendable
				},
				// Present so a stray `batch` would be visible rather than throwing something opaque.
				batch: () => {
					throw new Error("phantom bids must use force_batch")
				},
			},
			intentsCoprocessor: {
				placeBid: (commitment: HexString, userOp: HexString) => ({
					call: "placeBid",
					args: [commitment, userOp],
				}),
				retractBid: (commitment: HexString) => ({ call: "retractBid", args: [commitment] }),
			},
		},
		registry: {
			findMetaError: (module: { name: string }) => ({ section: "intentsCoprocessor", name: module.name }),
		},
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
	} as any

	return { batches, attempts, api }
}

describe("submitPhantomBids", () => {
	beforeAll(async () => {
		await cryptoWaitReady()
	})

	it("places every bid and its retraction in one force_batch", async () => {
		const node = mockNode({ events: [itemCompleted, itemCompleted, itemCompleted] })
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		const result = await coproc.submitPhantomBids([
			{ commitment: COMMITMENT_A, userOp: USER_OP, retractCommitment: PREV_A },
			{ commitment: COMMITMENT_B, userOp: USER_OP },
		])

		expect(node.batches).toHaveLength(1)
		expect(node.batches[0].map((c) => c.call)).toEqual(["placeBid", "retractBid", "placeBid"])
		expect(node.batches[0].map((c) => c.args[0])).toEqual([COMMITMENT_A, PREV_A, COMMITMENT_B])
		expect(result.bids).toEqual([
			{ commitment: COMMITMENT_A, success: true, error: undefined },
			{ commitment: COMMITMENT_B, success: true, error: undefined },
		])
		expect(result.blockHash).toBe("0xblockhash")
	})

	// The whole point of force_batch: one rejected bid must not take the others down with it.
	it("attributes a failed item to its own bid and leaves the rest placed", async () => {
		const node = mockNode({
			events: [itemCompleted, itemFailed("PhantomOrderBidWindowClosed"), itemCompleted],
		})
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		const result = await coproc.submitPhantomBids([
			{ commitment: COMMITMENT_A, userOp: USER_OP },
			{ commitment: COMMITMENT_B, userOp: USER_OP },
			{ commitment: COMMITMENT_C, userOp: USER_OP },
		])

		expect(result.bids.map((b) => b.success)).toEqual([true, false, true])
		expect(result.bids[1].error).toContain("PhantomOrderBidWindowClosed")
	})

	// A retraction is best-effort: the previous interval's bid may already be gone.
	it("keeps a bid placed when only its retraction failed", async () => {
		const node = mockNode({ events: [itemCompleted, itemFailed("BidNotFound"), itemCompleted] })
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		const result = await coproc.submitPhantomBids([
			{ commitment: COMMITMENT_A, userOp: USER_OP, retractCommitment: PREV_A },
			{ commitment: COMMITMENT_B, userOp: USER_OP },
		])

		expect(result.bids.map((b) => b.success)).toEqual([true, true])
	})

	it("marks every bid failed when the batch itself never lands", async () => {
		const node = mockNode({ dropped: true })
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		const result = await coproc.submitPhantomBids([
			{ commitment: COMMITMENT_A, userOp: USER_OP },
			{ commitment: COMMITMENT_B, userOp: USER_OP },
		])

		// Nothing landed, so there is nothing to attribute — every bid shares the batch's fate.
		expect(result.bids).toEqual([
			{ commitment: COMMITMENT_A, success: false, error: result.error },
			{ commitment: COMMITMENT_B, success: false, error: result.error },
		])
		expect(result.error).toBeTruthy()
	})

	/**
	 * Off-by-one insurance. If the item events cannot be lined up with the calls, reporting bids as
	 * placed is the safe direction: the next interval retracts them and a retraction of a bid that
	 * never existed fails harmlessly, whereas a bid wrongly recorded as failed is never retracted
	 * and leaves its deposit reserved.
	 */
	it("does not attribute outcomes when the item events do not line up with the calls", async () => {
		const node = mockNode({ events: [itemCompleted] })
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		const result = await coproc.submitPhantomBids([
			{ commitment: COMMITMENT_A, userOp: USER_OP },
			{ commitment: COMMITMENT_B, userOp: USER_OP },
		])

		expect(result.bids.map((b) => b.success)).toEqual([true, true])
		expect(result.error).toContain("not attributed")
	})

	/**
	 * A bid is worth nothing once its window closes, so a batch that reaches the pool and sits there
	 * is bumped rather than waited on: same nonce, double the tip, which is what the pool needs to
	 * swap the stalled copy for this one. Only one of the two can ever execute.
	 */
	it("bumps a batch that stalls in the pool and attributes the replacement's items", async () => {
		vi.useFakeTimers()
		try {
			const node = mockNode({ events: [itemCompleted, itemCompleted], stallAttempts: 1 })
			const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

			const submission = coproc.submitPhantomBids([
				{ commitment: COMMITMENT_A, userOp: USER_OP },
				{ commitment: COMMITMENT_B, userOp: USER_OP },
			])
			// Run out the inclusion watch on the first attempt so the bump goes out.
			await vi.advanceTimersByTimeAsync(INCLUSION_TIMEOUT_MS)
			const result = await submission

			expect(node.attempts).toHaveLength(2)
			expect(node.attempts[1].nonce).toBe(SIGNED_NONCE)
			expect(node.attempts[1].tip).toBe(node.attempts[0].tip! * 2n)
			// The bid calls are built once and re-signed, not batched twice.
			expect(node.batches).toHaveLength(1)
			expect(result.bids.map((bid) => bid.success)).toEqual([true, true])
			expect(result.blockHash).toBe("0xblockhash")
		} finally {
			vi.useRealTimers()
		}
	})

	it("is a no-op for an empty list", async () => {
		const node = mockNode()
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		expect(await coproc.submitPhantomBids([])).toEqual({ bids: [] })
		expect(node.batches).toEqual([])
	})
})
