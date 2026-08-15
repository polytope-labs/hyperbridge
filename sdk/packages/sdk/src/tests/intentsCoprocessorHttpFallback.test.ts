import { beforeAll, describe, expect, it } from "vitest"
import { cryptoWaitReady } from "@polkadot/util-crypto"
import { IntentsCoprocessor, deriveHttpUrl } from "@/chains/intentsCoprocessor"
import type { HexString } from "@/types"

/**
 * A bid is only worth anything inside its window, so a websocket that is down when the bid is ready
 * to sign means, in practice, not bidding at all. HTTP is the last resort for exactly that moment:
 * `author_submitExtrinsic` gets the extrinsic into the node's pool, and — having no subscription to
 * watch it with — the result says `pending`, which is the existing contract for "in flight, outcome
 * unknown, do not re-sign".
 */

const COMMITMENT = "0x4380111111111111111111111111111111111111111111111111111111114818" as HexString
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
	asInBlock: { toHex: () => "0xwsblockhash" },
	type: "InBlock",
}

/** The exact shape of a pool rejection as polkadot-js surfaces it: an RpcError with a code. */
function rpcError(code: number, message: string): Error {
	const err = new Error(message) as Error & { code: number }
	err.code = code
	return err
}

interface Submission {
	via: "ws" | "http"
	tip: bigint
}

/**
 * A websocket api that watches submissions to inclusion, and an HTTP api that only accepts them
 * into the pool. `httpResult` decides what the HTTP node does with the extrinsic it is handed.
 */
function mockNode(options: { wsConnected: boolean; httpResult?: () => Promise<unknown> }) {
	const submissions: Submission[] = []

	const wsSendable = {
		hash: { toHex: () => "0xwsextrinsichash" },
		signAndSend: (_keyPair: unknown, opts: { tip: bigint }, cb: (result: unknown) => void) => {
			submissions.push({ via: "ws", tip: opts.tip })
			queueMicrotask(() => cb({ dispatchError: undefined, status: inBlockStatus, events: [] }))
			return Promise.resolve(() => {})
		},
	}

	const httpSendable = {
		hash: { toHex: () => "0xhttpextrinsichash" },
		// No callback overload: HTTP has no subscriptions, so this resolves to the extrinsic hash.
		signAndSend: (_keyPair: unknown, opts: { tip: bigint }) => {
			submissions.push({ via: "http", tip: opts.tip })
			return options.httpResult?.() ?? Promise.resolve({ toHex: () => "0xhttpextrinsichash" })
		},
	}

	const txFor = (sendable: unknown) => ({
		intentsCoprocessor: {
			placeBid: () => sendable,
			retractBid: () => sendable,
		},
		utility: { batch: () => sendable },
	})

	const registry = { findMetaError: () => ({ section: "intentsCoprocessor", name: "BidNotFound" }) }

	return {
		submissions,
		wsApi: { isConnected: options.wsConnected, tx: txFor(wsSendable), registry } as any,
		httpApi: { isConnected: true, tx: txFor(httpSendable), registry } as any,
	}
}

/** Builds a coprocessor with the HTTP api already connected, so no real endpoint is dialled. */
function coprocessorWithHttp(node: ReturnType<typeof mockNode>): IntentsCoprocessor {
	const coproc = IntentsCoprocessor.fromApi(node.wsApi, "//Alice")
	;(coproc as any).httpApi = Promise.resolve(node.httpApi)
	return coproc
}

describe("HTTP submission fallback", () => {
	beforeAll(async () => {
		await cryptoWaitReady()
	})

	it("submits over HTTP when the websocket is disconnected, reporting the extrinsic as pending", async () => {
		const node = mockNode({ wsConnected: false })
		const coproc = coprocessorWithHttp(node)

		const result = await coproc.submitBid(COMMITMENT, USER_OP)

		expect(node.submissions.map((s) => s.via)).toEqual(["http"])
		expect(result.success).toBe(false)
		// Unwatchable, not failed: the caller must confirm later rather than re-sign the nonce.
		expect(result.pending).toBe(true)
		expect(result.extrinsicHash).toBe("0xhttpextrinsichash")
	})

	it("keeps using the websocket while it is connected", async () => {
		const node = mockNode({ wsConnected: true })
		const coproc = coprocessorWithHttp(node)

		const result = await coproc.submitBid(COMMITMENT, USER_OP)

		expect(node.submissions.map((s) => s.via)).toEqual(["ws"])
		expect(result.success).toBe(true)
		expect(result.blockHash).toBe("0xwsblockhash")
	})

	it("routes a batched bid+retraction over HTTP too, built against the HTTP api", async () => {
		const node = mockNode({ wsConnected: false })
		const coproc = coprocessorWithHttp(node)

		const result = await coproc.submitBidWithRetraction(COMMITMENT, COMMITMENT, USER_OP)

		expect(node.submissions.map((s) => s.via)).toEqual(["http"])
		expect(result.pending).toBe(true)
	})

	// The fallback must fail fast rather than hang: a caller blocked forever on an unreachable
	// endpoint is worse than the websocket outage it was meant to cover.
	it("gives up with an error when the derived HTTP endpoint cannot be reached", async () => {
		const node = mockNode({ wsConnected: false })
		// Port 1 is never listening, so the connection is refused instead of left open.
		node.wsApi._rpcCore = { provider: { endpoint: "ws://127.0.0.1:1" } }
		const coproc = IntentsCoprocessor.fromApi(node.wsApi, "//Alice")

		const result = await coproc.submitBid(COMMITMENT, USER_OP)

		expect(node.submissions).toEqual([])
		expect(result.success).toBe(false)
		expect(result.error).toContain("127.0.0.1:1")
	})

	// Same rule as the websocket path: a copy already pooled is in flight, not failed.
	it("reports a pooled duplicate rejected over HTTP as pending", async () => {
		const node = mockNode({
			wsConnected: false,
			httpResult: () => Promise.reject(rpcError(1014, "1014: Priority is too low")),
		})
		const coproc = coprocessorWithHttp(node)

		const result = await coproc.submitBid(COMMITMENT, USER_OP)

		expect(result.success).toBe(false)
		expect(result.pending).toBe(true)
	})

	it("returns a terminal failure when the HTTP node rejects the extrinsic outright", async () => {
		const node = mockNode({
			wsConnected: false,
			httpResult: () => Promise.reject(rpcError(1010, "1010: Invalid Transaction: Inability to pay some fees")),
		})
		const coproc = coprocessorWithHttp(node)

		const result = await coproc.submitBid(COMMITMENT, USER_OP)

		expect(result.success).toBe(false)
		expect(result.pending).toBeUndefined()
		// One shot: HTTP is the fallback, and a node that refused the extrinsic said so definitively.
		expect(node.submissions).toHaveLength(1)
	})
})

describe("RPC queries", () => {
	/** A websocket that fails loudly on any use: queries must never reach it. */
	const tripwireWebsocket = () =>
		new Proxy(
			{ isConnected: true },
			{
				get: (target, prop) => {
					if (prop === "isConnected") return true
					throw new Error("queries must not touch the websocket")
				},
			},
		) as any

	/** An HTTP api serving one bid, both through the custom RPC and through raw storage. */
	function httpNode() {
		const calls: string[] = []
		const filler = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"
		return {
			calls,
			api: {
				_rpcCore: {
					provider: {
						send: async (method: string) => {
							calls.push(method)
							throw new Error("intents RPC not available on this node")
						},
					},
				},
				query: {
					intentsCoprocessor: {
						bids: {
							entries: async () => {
								calls.push("bids.entries")
								return [[{ args: [null, { toString: () => filler }] }, { toString: () => "42" }]]
							},
						},
					},
				},
				rpc: {
					offchain: {
						localStorageGet: async () => {
							calls.push("offchain.localStorageGet")
							return { isNone: true }
						},
					},
				},
			} as any,
		}
	}

	it("reads bid storage entries over HTTP", async () => {
		const node = httpNode()
		const coproc = IntentsCoprocessor.fromApi(tripwireWebsocket(), "//Alice")
		;(coproc as any).httpApi = Promise.resolve(node.api)

		const entries = await coproc.getBidStorageEntries(COMMITMENT)

		expect(entries).toEqual([{ commitment: COMMITMENT, filler: expect.any(String), deposit: 42n }])
		expect(node.calls).toEqual(["bids.entries"])
	})

	// Both routes — the custom RPC and the storage fallback it degrades to — stay on HTTP.
	it("reads bids over HTTP through the RPC and the storage fallback alike", async () => {
		const node = httpNode()
		const coproc = IntentsCoprocessor.fromApi(tripwireWebsocket(), "//Alice")
		;(coproc as any).httpApi = Promise.resolve(node.api)

		const bids = await coproc.getBidsForOrder(COMMITMENT)

		expect(bids).toEqual([])
		expect(node.calls).toEqual(["intents_getBidsForOrder", "bids.entries", "offchain.localStorageGet"])
	})
})

describe("deriveHttpUrl", () => {
	it("maps a websocket endpoint onto HTTP on the same host and port", () => {
		expect(deriveHttpUrl("wss://nexus.rpc.example.com")).toBe("https://nexus.rpc.example.com")
		expect(deriveHttpUrl("ws://127.0.0.1:9944")).toBe("http://127.0.0.1:9944")
		expect(deriveHttpUrl("wss://example.com:443/rpc")).toBe("https://example.com:443/rpc")
	})

	it("throws for anything that is not a websocket url, rather than guessing", () => {
		expect(() => deriveHttpUrl("https://example.com")).toThrow()
		expect(() => deriveHttpUrl("")).toThrow()
	})
})
