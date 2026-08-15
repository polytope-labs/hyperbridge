import { beforeAll, describe, expect, it } from "vitest"
import { cryptoWaitReady } from "@polkadot/util-crypto"
import { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { BidSubmissionResult, HexString } from "@/types"

/**
 * Regression tests for the "retraction races its own resubmission" incident (#1074).
 *
 * A retraction entered the Hyperbridge tx pool but its submitAndWatch handle never confirmed.
 * The retry loop then re-signed the same call: the copies bounced off the pooled original with
 * RPC 1014 ("priority is too low"), and once the original executed — retracting the bid — a late
 * duplicate landed on-chain only to fail with `BidNotFound`. On-chain cleanup had succeeded;
 * locally it was recorded as a failure.
 *
 * The fix: an extrinsic known to be in the pool is in flight, not failed. Submission bounces
 * 1013/1014 and watch timeouts after a pool-entry status now return `pending` immediately instead
 * of re-signing, and callers confirm the outcome later.
 *
 * These tests drive the real code paths through a mock ApiPromise (via the public `fromApi`
 * factory) so no node is required.
 */

const COMMITMENT = "0x4380111111111111111111111111111111111111111111111111111111114818" as HexString

const readyStatus = {
	isFuture: false,
	isReady: true,
	isBroadcast: false,
	isRetracted: false,
	isInBlock: false,
	isFinalized: false,
	isDropped: false,
	isInvalid: false,
	isUsurped: false,
	isFinalityTimeout: false,
	type: "Ready",
}

/** The exact shape of a pool rejection as polkadot-js surfaces it: an RpcError with a code. */
function rpcError(code: number, message: string): Error {
	const err = new Error(message) as Error & { code: number }
	err.code = code
	return err
}

/**
 * Builds a mock ApiPromise whose retractBid extrinsic behaves per `behaviour` on each
 * successive signAndSend attempt. `calls` counts signAndSend invocations.
 */
function mockApi(behaviour: (attempt: number, cb: (result: unknown) => void) => Promise<() => void>) {
	const calls = { count: 0 }
	const sendable = {
		hash: { toHex: () => "0xextrinsichash" },
		signAndSend: (_keyPair: unknown, _opts: unknown, cb: (result: unknown) => void) => {
			calls.count++
			return behaviour(calls.count, cb)
		},
	}
	const api = {
		// A live socket: submission only diverts to the HTTP fallback when this is false.
		isConnected: true,
		tx: {
			intentsCoprocessor: {
				retractBid: () => sendable,
			},
		},
		registry: {
			findMetaError: () => ({ section: "intentsCoprocessor", name: "BidNotFound" }),
		},
	} as any
	return { api, calls }
}

/** Runs retractBid with a short watch timeout so timeout paths don't take 30s. */
async function retractWithShortTimeout(coproc: IntentsCoprocessor, timeoutMs: number): Promise<BidSubmissionResult> {
	return await (coproc as any).signAndSendExtrinsic(
		(api: any) => api.tx.intentsCoprocessor.retractBid(COMMITMENT),
		3,
		timeoutMs,
	)
}

describe("in-flight extrinsic handling", () => {
	beforeAll(async () => {
		await cryptoWaitReady()
	})

	it("returns pending on a 1014 pool bounce without re-signing — the pooled original is in flight", async () => {
		const { api, calls } = mockApi(() =>
			Promise.reject(
				rpcError(
					1014,
					"1014: Priority is too low: (733000000733 vs 726000000726): The transaction has too low priority to replace another transaction already in the pool.",
				),
			),
		)
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await coproc.retractBid(COMMITMENT)

		expect(result.success).toBe(false)
		expect(result.pending).toBe(true)
		expect(calls.count).toBe(1)
	})

	it("returns pending on a 1013 already-imported bounce, matching by message when no code is set", async () => {
		const { api, calls } = mockApi(() => Promise.reject(new Error("Transaction is already in the pool")))
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await coproc.retractBid(COMMITMENT)

		expect(result.pending).toBe(true)
		expect(calls.count).toBe(1)
	})

	it("returns pending when the watch times out after the extrinsic entered the pool", async () => {
		const { api, calls } = mockApi((_attempt, cb) => {
			// Ready arrives, then the watch goes silent — the production failure mode.
			queueMicrotask(() => cb({ dispatchError: undefined, status: readyStatus, events: [] }))
			return Promise.resolve(() => {})
		})
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 100)

		expect(result.success).toBe(false)
		expect(result.pending).toBe(true)
		expect(result.extrinsicHash).toBe("0xextrinsichash")
		expect(result.error).toContain("in the transaction pool")
		expect(calls.count).toBe(1)
	})

	it("still retries when the watch times out with no pool status at all", async () => {
		const { api, calls } = mockApi(() => Promise.resolve(() => {}))
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 50)

		expect(result.success).toBe(false)
		expect(result.pending).toBeUndefined()
		expect(result.error).toBe("Transaction failed after 3 attempts")
		expect(calls.count).toBe(3)
	})

	it("still retries on submission errors that do not indicate a pooled copy", async () => {
		const { api, calls } = mockApi(() =>
			Promise.reject(rpcError(1010, "1010: Invalid Transaction: Inability to pay some fees")),
		)
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await coproc.retractBid(COMMITMENT)

		expect(result.success).toBe(false)
		expect(result.pending).toBeUndefined()
		expect(calls.count).toBe(3)
	})

	it("reports a dispatch error as terminal failure, not pending", async () => {
		const inBlockWithError = {
			...readyStatus,
			isReady: false,
			isInBlock: true,
			asInBlock: { toHex: () => "0xblockhash" },
			type: "InBlock",
		}
		const { api, calls } = mockApi((_attempt, cb) => {
			queueMicrotask(() =>
				cb({ dispatchError: { isModule: true, asModule: {} }, status: inBlockWithError, events: [] }),
			)
			return Promise.resolve(() => {})
		})
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await coproc.retractBid(COMMITMENT)

		expect(result.success).toBe(false)
		expect(result.pending).toBeUndefined()
		expect(result.error).toContain("BidNotFound")
		expect(calls.count).toBe(1)
	})
})
