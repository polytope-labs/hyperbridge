import { beforeAll, describe, expect, it, vi } from "vitest"
import { cryptoWaitReady } from "@polkadot/util-crypto"
import type { ApiPromise } from "@polkadot/api"
import { IntentsCoprocessor } from "@/chains/intentsCoprocessor"
import type { BidSubmissionResult, HexString } from "@/types"

/**
 * Regression tests for the "retraction races its own resubmission" incident.
 *
 * A retraction entered the Hyperbridge tx pool but its submitAndWatch handle never confirmed.
 * The retry loop then re-signed the same call: the copies bounced off the pooled original with
 * RPC 1014 ("priority is too low"), and once the original executed — retracting the bid — a late
 * duplicate landed on-chain only to fail with `BidNotFound`. On-chain cleanup had succeeded;
 * locally it was recorded as a failure.
 *
 * The fix: an extrinsic known to be in the pool is in flight, not failed. It is never re-signed
 * under a fresh nonce. A submission bounce (1013/1014) returns `pending` immediately, and a watch
 * that times out after a pool-entry status is only ever retried as a *replacement* — the same nonce
 * the stalled extrinsic was signed with, at double the tip — so at most one copy can execute. When
 * the bumps run out, the result is still `pending` and the caller confirms the outcome later.
 *
 * These tests drive the real code paths through a mock ApiPromise (via the public `fromApi`
 * factory) so no node is required.
 */

const COMMITMENT = "0x4380111111111111111111111111111111111111111111111111111111114818" as HexString
/** The identifier this filler files its bid under. */
const BID = `0x${"b1".repeat(32)}` as HexString

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

/** Held by the pool behind a missing lower nonce of the same account. */
const futureStatus = { ...readyStatus, isFuture: true, isReady: false, type: "Future" }

const inBlockStatus = {
	...readyStatus,
	isReady: false,
	isInBlock: true,
	asInBlock: { toHex: () => "0xblockhash" },
	type: "InBlock",
}

/** The exact shape of a pool rejection as polkadot-js surfaces it: an RpcError with a code. */
function rpcError(code: number, message: string): Error {
	const err = new Error(message) as Error & { code: number }
	err.code = code
	return err
}

/** What `system_accountNextIndex` reports: the next nonce clear of the ready queue. */
const POOL_NONCE = 41

/** The nonce a signed extrinsic reports, which a replacement must be pinned to. */
const SIGNED_NONCE = 7

/**
 * Builds a mock ApiPromise whose retractBid extrinsic behaves per `behaviour` on each
 * successive signAndSend attempt. `calls` counts signAndSend invocations and records the sign
 * options each one went out with, and those of every `system.remark` sent to fill a nonce gap.
 * `remark.send` decides what the node does with a remark. `index.value` is what
 * `system_accountNextIndex` reports and `index.reads` counts its reads. `signedNonce` is what a
 * signed extrinsic reports as its nonce; pass null to model an extrinsic whose nonce cannot be read.
 */
function mockApi(
	behaviour: (attempt: number, cb: (result: unknown) => void) => Promise<() => void>,
	signedNonce: number | null = SIGNED_NONCE,
) {
	const calls: { count: number; options: SignOptions[]; remarks: SignOptions[] } = {
		count: 0,
		options: [],
		remarks: [],
	}
	const index = { value: POOL_NONCE, reads: 0 }
	const remark = { send: (): Promise<unknown> => Promise.resolve({ toHex: () => "0xremarkhash" }) }
	const sendable = {
		hash: { toHex: () => "0xextrinsichash" },
		// Present on a signed extrinsic; the retry loop reads it to pin a replacement.
		nonce: signedNonce === null ? undefined : { toNumber: () => signedNonce },
		signAndSend: (_keyPair: unknown, opts: { tip?: bigint; nonce?: number }, cb: (result: unknown) => void) => {
			calls.count++
			calls.options.push(opts)
			return behaviour(calls.count, cb)
		},
	}
	const api = {
		// A live socket: submission only diverts to the HTTP fallback when this is false.
		isConnected: true,
		// The pool-aware nonce source: the chain nonce plus this account's contiguous ready
		// transactions, as the node serving the read sees them.
		rpc: {
			system: {
				accountNextIndex: async () => {
					index.reads++
					return { toString: () => String(index.value) }
				},
			},
		},
		tx: {
			intentsCoprocessor: {
				retractBid: () => sendable,
			},
			system: {
				remark: () => ({
					signAndSend: (_keyPair: unknown, opts: SignOptions) => {
						calls.remarks.push(opts)
						return remark.send()
					},
				}),
			},
		},
		registry: {
			findMetaError: () => ({ section: "intentsCoprocessor", name: "BidNotFound" }),
		},
	} as any
	return { api, calls, index, remark }
}

type SignOptions = { tip?: bigint; nonce?: number }

/** Runs retractBid with a short watch timeout so timeout paths don't take 30s. */
async function retractWithShortTimeout(coproc: IntentsCoprocessor, timeoutMs: number): Promise<BidSubmissionResult> {
	return await (coproc as any).signAndSendExtrinsic(
		(api: any) => api.tx.intentsCoprocessor.retractBid(COMMITMENT, BID),
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

		const result = await coproc.retractBid(COMMITMENT, BID)

		expect(result.success).toBe(false)
		expect(result.pending).toBe(true)
		expect(calls.count).toBe(1)
	})

	it("returns pending on a 1013 already-imported bounce, matching by message when no code is set", async () => {
		const { api, calls } = mockApi(() => Promise.reject(new Error("Transaction is already in the pool")))
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await coproc.retractBid(COMMITMENT, BID)

		expect(result.pending).toBe(true)
		expect(calls.count).toBe(1)
	})

	it("replaces a stalled extrinsic under its own nonce at a doubled tip, and ends pending", async () => {
		const { api, calls, index } = mockApi((_attempt, cb) => {
			// Ready arrives, then the watch goes silent — the production failure mode.
			index.value = POOL_NONCE + 1
			queueMicrotask(() => cb({ dispatchError: undefined, status: readyStatus, events: [] }))
			return Promise.resolve(() => {})
		})
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 100)

		expect(calls.count).toBe(3)
		// A stall in Ready has no gap below it to look for.
		expect(index.reads).toBe(1)
		expect(calls.remarks).toEqual([])
		// The first attempt takes the pool-aware nonce; every bump is pinned to the nonce that attempt
		// was signed with, so the pool replaces the stalled copy instead of queueing behind it.
		expect(calls.options.map((opts) => opts.nonce)).toEqual([POOL_NONCE, POOL_NONCE, POOL_NONCE])
		const tips = calls.options.map((opts) => opts.tip)
		expect(tips[1]).toBe(tips[0]! * 2n)
		expect(tips[2]).toBe(tips[0]! * 4n)
		// Bumps ran out, but a replacement may still be sitting in the pool — in flight, not failed.
		expect(result.success).toBe(false)
		expect(result.pending).toBe(true)
		expect(result.extrinsicHash).toBe("0xextrinsichash")
		expect(result.error).toContain("in the transaction pool")
	})

	it("reports success when a bumped replacement of a stalled extrinsic lands", async () => {
		const inBlock = {
			...readyStatus,
			isReady: false,
			isInBlock: true,
			asInBlock: { toHex: () => "0xblockhash" },
			type: "InBlock",
		}
		const { api, calls } = mockApi((attempt, cb) => {
			queueMicrotask(() =>
				cb({ dispatchError: undefined, status: attempt === 1 ? readyStatus : inBlock, events: [] }),
			)
			return Promise.resolve(() => {})
		})
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 100)

		expect(calls.count).toBe(2)
		expect(calls.options[1].nonce).toBe(POOL_NONCE)
		expect(result.success).toBe(true)
		expect(result.blockHash).toBe("0xblockhash")
	})

	/**
	 * The replacement is only safe because it is pinned to the stalled extrinsic's nonce. Without
	 * one, an unpinned retry could take the next nonce and land a second copy alongside the first,
	 * which is exactly the duplicate this whole path exists to prevent.
	 */
	it("does not retry a stalled extrinsic whose nonce is neither supplied nor readable", async () => {
		const { api, calls } = mockApi((_attempt, cb) => {
			queueMicrotask(() => cb({ dispatchError: undefined, status: readyStatus, events: [] }))
			return Promise.resolve(() => {})
		}, null)
		// No accountNextIndex to ask, so the submission signs against on-chain state and the only
		// nonce a replacement could pin is the one the signed extrinsic does not report.
		api.rpc = {}
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 100)

		expect(calls.count).toBe(1)
		expect(result.pending).toBe(true)
		expect(result.extrinsicHash).toBe("0xextrinsichash")
	})

	it("pins replacements to the signed extrinsic's nonce when the node has no accountNextIndex to ask", async () => {
		const { api, calls } = mockApi((_attempt, cb) => {
			queueMicrotask(() => cb({ dispatchError: undefined, status: readyStatus, events: [] }))
			return Promise.resolve(() => {})
		})
		api.rpc = {}
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 100)

		expect(calls.options.map((opts) => opts.nonce)).toEqual([undefined, SIGNED_NONCE, SIGNED_NONCE])
		const tips = calls.options.map((opts) => opts.tip)
		expect(tips[1]).toBe(tips[0]! * 2n)
		expect(tips[2]).toBe(tips[0]! * 4n)
		expect(result.success).toBe(false)
		expect(result.pending).toBe(true)
	})

	it("keeps a bounced replacement pending, reporting the stalled extrinsic still in flight", async () => {
		const { api, calls } = mockApi((attempt, cb) => {
			if (attempt === 1) {
				queueMicrotask(() => cb({ dispatchError: undefined, status: readyStatus, events: [] }))
				return Promise.resolve(() => {})
			}
			// The pool refused the replacement, so the original is still the one in flight.
			return Promise.reject(rpcError(1014, "1014: Priority is too low"))
		})
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const result = await retractWithShortTimeout(coproc, 100)

		expect(calls.count).toBe(2)
		expect(result.pending).toBe(true)
		expect(result.extrinsicHash).toBe("0xextrinsichash")
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

		const result = await coproc.retractBid(COMMITMENT, BID)

		expect(result.success).toBe(false)
		expect(result.pending).toBeUndefined()
		expect(calls.count).toBe(3)
	})

	/**
	 * The default watch is sized against the bid window: an extrinsic still pooled after 20s has
	 * missed several blocks, and bumping it beats waiting on it.
	 */
	it("watches for inclusion for 20s per attempt by default", async () => {
		vi.useFakeTimers()
		try {
			const { api, calls } = mockApi((_attempt, cb) => {
				queueMicrotask(() => cb({ dispatchError: undefined, status: readyStatus, events: [] }))
				return Promise.resolve(() => {})
			})
			const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

			const submission = coproc.retractBid(COMMITMENT, BID)
			await vi.advanceTimersByTimeAsync(20_000)
			expect(calls.count).toBe(2)

			await vi.advanceTimersByTimeAsync(40_000)
			const result = await submission

			expect(calls.count).toBe(3)
			expect(result.pending).toBe(true)
			expect(result.error).toContain("timed out after 20000ms")
		} finally {
			vi.useRealTimers()
		}
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

		const result = await coproc.retractBid(COMMITMENT, BID)

		expect(result.success).toBe(false)
		expect(result.pending).toBeUndefined()
		expect(result.error).toContain("BidNotFound")
		expect(calls.count).toBe(1)
	})

	/**
	 * The queue serialises submissions but a watch gives up while its extrinsic is still pooled,
	 * and neither the on-chain nonce nor the ready queue has moved on from it yet. A second,
	 * unrelated extrinsic signed against either would take the same nonce and bounce off the first
	 * (1013/1014) instead of queueing behind it, which is how several bids on one order lost their
	 * auction. `signAndSend`'s own `nonce: -1` is no help: on a runtime carrying `AccountNonceApi`,
	 * as Hyperbridge has, polkadot-js reads that runtime call and never sees the pool.
	 */
	it("moves each new extrinsic past the nonce it last put in the pool", async () => {
		const { api, calls } = mockApi(async (_attempt, cb) => {
			cb({ status: { isReady: true }, dispatchError: undefined })
			return () => {}
		})
		const coproc = IntentsCoprocessor.fromApi(api, "//Alice")

		const first = await retractWithShortTimeout(coproc, 20)
		const second = await retractWithShortTimeout(coproc, 20)

		// Both stalled in the pool, and the RPC still reports the same index for the second.
		expect(first.pending).toBe(true)
		expect(second.pending).toBe(true)
		expect(calls.options[0].nonce).toBe(POOL_NONCE)
		expect(calls.options.at(-1)?.nonce).toBe(POOL_NONCE + 1)
	})

	it("signs against on-chain state when the node has no accountNextIndex to ask", async () => {
		const inBlock = {
			...readyStatus,
			isReady: false,
			isInBlock: true,
			asInBlock: { toHex: () => "0xblockhash" },
			type: "InBlock",
		}
		const { api, calls } = mockApi((_attempt, cb) => {
			queueMicrotask(() => cb({ dispatchError: undefined, status: inBlock, events: [] }))
			return Promise.resolve(() => {})
		})
		api.rpc = {}

		const result = await retractWithShortTimeout(IntentsCoprocessor.fromApi(api, "//Alice"), 100)

		expect(result.success).toBe(true)
		expect(calls.options[0].nonce).toBeUndefined()
	})
})

/** A coprocessor that last put `lastPooledNonce` into the pool. */
function coprocessorAfter(api: ApiPromise, lastPooledNonce: number): IntentsCoprocessor {
	const coproc = IntentsCoprocessor.fromApi(api, "//Alice")
	;(coproc as unknown as { lastPooledNonce: number }).lastPooledNonce = lastPooledNonce
	return coproc
}

/**
 * When the pool drops an extrinsic of ours without including it, the next one, signed past it,
 * waits in Future behind the missing nonce. The node reports that nonce through
 * `system_accountNextIndex`, and a `system.remark` fills it, so the extrinsic already pooled above
 * it lands in the order it was signed.
 */
describe("nonce gap below a Future stall", () => {
	const N = POOL_NONCE

	beforeAll(async () => {
		await cryptoWaitReady()
	})

	it("fills the nonce a dropped extrinsic left with a remark, and the call stalled above it lands", async () => {
		const node = mockApi((_attempt, cb) => {
			const nonce = node.calls.options.at(-1)?.nonce
			let status: unknown = futureStatus
			if (nonce === N) {
				node.index.value = N + 1
				status = readyStatus
			} else if (node.calls.remarks.length > 0) {
				status = inBlockStatus
			}
			queueMicrotask(() => cb({ dispatchError: undefined, status, events: [] }))
			return Promise.resolve(() => {})
		})
		const coproc = IntentsCoprocessor.fromApi(node.api, "//Alice")

		const dropped = await retractWithShortTimeout(coproc, 20)
		node.index.value = N
		const result = await retractWithShortTimeout(coproc, 20)

		expect(dropped.pending).toBe(true)
		expect(node.calls.options.map((opts) => opts.nonce)).toEqual([N, N, N, N + 1, N + 1])
		expect(node.calls.remarks).toEqual([{ tip: 0n, nonce: N }])
		expect(result.success).toBe(true)
	})

	it("sends no remark when the node's index has caught up to the stalled nonce", async () => {
		const node = mockApi((_attempt, cb) => {
			node.index.value = N + 1
			queueMicrotask(() => cb({ dispatchError: undefined, status: futureStatus, events: [] }))
			return Promise.resolve(() => {})
		})
		const coproc = coprocessorAfter(node.api, N)

		const result = await retractWithShortTimeout(coproc, 20)

		expect(node.calls.options.map((opts) => opts.nonce)).toEqual([N + 1, N + 1, N + 1])
		expect(node.calls.remarks).toEqual([])
		expect(result.pending).toBe(true)
	})

	it("treats a stall as Future only when Future was the last status the watch saw", async () => {
		const node = mockApi((_attempt, cb) => {
			queueMicrotask(() => {
				cb({ dispatchError: undefined, status: futureStatus, events: [] })
				// The missing nonce arrived, so this one moved up to Ready.
				node.index.value = N + 2
				cb({ dispatchError: undefined, status: readyStatus, events: [] })
			})
			return Promise.resolve(() => {})
		})
		const coproc = coprocessorAfter(node.api, N)

		const result = await retractWithShortTimeout(coproc, 20)

		expect((result as { future?: boolean }).future).toBeUndefined()
		expect(node.index.reads).toBe(1)
		expect(node.calls.remarks).toEqual([])
		expect(node.calls.options.map((opts) => opts.nonce)).toEqual([N + 1, N + 1, N + 1])
	})

	it("keeps replacing the call as before when the remark cannot be sent", async () => {
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
		try {
			const node = mockApi((_attempt, cb) => {
				queueMicrotask(() => cb({ dispatchError: undefined, status: futureStatus, events: [] }))
				return Promise.resolve(() => {})
			})
			node.remark.send = () =>
				Promise.reject(rpcError(1010, "1010: Invalid Transaction: Inability to pay some fees"))
			const coproc = coprocessorAfter(node.api, N)

			const result = await retractWithShortTimeout(coproc, 20)

			expect(node.calls.options.map((opts) => opts.nonce)).toEqual([N + 1, N + 1, N + 1])
			expect(node.calls.remarks.map((opts) => opts.nonce)).toEqual([N, N, N])
			expect(warn).toHaveBeenCalledTimes(3)
			expect(result.success).toBe(false)
			expect(result.pending).toBe(true)
			expect(result.extrinsicHash).toBe("0xextrinsichash")
			expect(result.error).toContain("in the transaction pool")
		} finally {
			warn.mockRestore()
		}
	})

	it("gives up on a remark that never reaches the pool within the attempt's timeout", async () => {
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
		try {
			const node = mockApi((_attempt, cb) => {
				queueMicrotask(() => cb({ dispatchError: undefined, status: futureStatus, events: [] }))
				return Promise.resolve(() => {})
			})
			node.remark.send = () => new Promise(() => {})
			const coproc = coprocessorAfter(node.api, N)

			const result = await retractWithShortTimeout(coproc, 20)

			expect(node.calls.options.map((opts) => opts.nonce)).toEqual([N + 1, N + 1, N + 1])
			expect(node.calls.remarks.map((opts) => opts.nonce)).toEqual([N, N, N])
			expect(warn).toHaveBeenCalledTimes(3)
			expect(String(warn.mock.calls[0][1])).toContain("Timed out after 20ms")
			expect(result.pending).toBe(true)
		} finally {
			warn.mockRestore()
		}
	})
})
