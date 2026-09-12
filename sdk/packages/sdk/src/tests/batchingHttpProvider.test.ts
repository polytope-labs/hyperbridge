import http from "node:http"
import type { AddressInfo } from "node:net"
import { afterEach, beforeEach, describe, expect, it } from "vitest"
import { BatchingHttpProvider } from "@/utils/batchingHttpProvider"
import { TokenBucket } from "@/utils/rateLimiter"

/**
 * The endpoint's limit is counted in HTTP requests, and the reads this SDK makes arrive as bursts of
 * concurrent calls — one offchain read per configured chain on a phantom order interval, one block
 * hash per block in a scan. JSON-RPC 2.0 lets a burst travel as one request. These pin that it does,
 * that a lone call is still sent the way the base provider sends it, and that a server which refuses
 * batches costs correctness nothing.
 */

/** One HTTP request as the node saw it: the parsed body, and whether it was an array. */
interface Seen {
	batched: boolean
	methods: string[]
}

interface Call {
	id: number
	method: string
	params: unknown[]
}

interface StubOptions {
	/** Batch policy, mirroring `--rpc-disable-batch-requests` and `--rpc-max-batch-request-len`. */
	mode?: "ok" | "disabled" | { maxLen: number }
	/** The result for a call that succeeds. */
	respond?: (call: Call) => unknown
	/** A JSON-RPC error for this call instead of a result. */
	errorFor?: (call: Call) => { code: number; message: string } | undefined
	/** A non-200 status for every request, instead of answering. */
	status?: { code: number; message: string }
}

/** A node that answers JSON-RPC over HTTP, recording each request. */
function stubNode(options: StubOptions = {}) {
	const { mode = "ok", respond = (call: Call) => `result-${call.method}-${call.id}`, errorFor, status } = options
	const seen: Seen[] = []

	const server = http.createServer((req, res) => {
		if (status) {
			res.statusCode = status.code
			res.statusMessage = status.message
			res.end()
			return
		}
		let body = ""
		req.on("data", (chunk: Buffer) => {
			body += chunk
		})
		req.on("end", () => {
			const parsed = JSON.parse(body) as Call | Call[]
			const batched = Array.isArray(parsed)
			const calls = batched ? parsed : [parsed]
			seen.push({ batched, methods: calls.map((call) => call.method) })

			res.setHeader("content-type", "application/json")

			// jsonrpsee answers a refused batch with a single error object, not an array.
			if (batched && mode === "disabled") {
				res.end(
					JSON.stringify({
						jsonrpc: "2.0",
						id: null,
						error: { code: -32005, message: "Batched requests are not supported by this server" },
					}),
				)
				return
			}
			if (batched && typeof mode === "object" && calls.length > mode.maxLen) {
				res.end(
					JSON.stringify({
						jsonrpc: "2.0",
						id: null,
						error: { code: -32010, message: "The batch request was too large" },
					}),
				)
				return
			}

			// Answered in reverse, because the spec says a client matches on id, not position.
			const responses = calls
				.map((call) => {
					const error = errorFor?.(call)
					return error
						? { jsonrpc: "2.0", id: call.id, error }
						: { jsonrpc: "2.0", id: call.id, result: respond(call) }
				})
				.reverse()
			res.end(JSON.stringify(batched ? responses : responses[0]))
		})
	})

	return {
		seen,
		requests: () => seen.length,
		listen: () =>
			new Promise<number>((resolve) =>
				server.listen(0, "127.0.0.1", () => resolve((server.address() as AddressInfo).port)),
			),
		// Idempotent: a test that closes the node to force a failure is also torn down afterwards.
		close: () =>
			new Promise<void>((resolve, reject) => {
				if (!server.listening) return resolve()
				server.closeAllConnections()
				server.close((err) => (err ? reject(err) : resolve()))
			}),
	}
}

/** A bucket wide enough that nothing in these tests waits on it. */
const openBucket = () => new TokenBucket(10_000)

describe("BatchingHttpProvider", () => {
	let node: ReturnType<typeof stubNode>
	let port: number
	let provider: BatchingHttpProvider | undefined

	const start = async (options: Parameters<typeof stubNode>[0] = {}) => {
		node = stubNode(options)
		port = await node.listen()
		return `http://127.0.0.1:${port}`
	}

	beforeEach(() => {
		provider = undefined
	})

	afterEach(async () => {
		await provider?.disconnect()
		await node?.close()
	})

	it("sends concurrent calls as one batched request", async () => {
		provider = new BatchingHttpProvider(await start(), {}, openBucket())

		const results = await Promise.all([
			provider.send<string>("chain_getBlockHash", [1]),
			provider.send<string>("chain_getBlockHash", [2]),
			provider.send<string>("chain_getBlockHash", [3]),
		])

		expect(node.requests()).toBe(1)
		expect(node.seen[0].batched).toBe(true)
		expect(node.seen[0].methods).toHaveLength(3)
		expect(results).toHaveLength(3)
	})

	// Ids, not positions: a server is free to answer a batch in any order, and this stub does.
	it("routes each reply to the call that asked for it", async () => {
		provider = new BatchingHttpProvider(await start({ respond: (call) => call.params[0] }), {}, openBucket())

		const results = await Promise.all([
			provider.send<number>("chain_getBlockHash", [10]),
			provider.send<number>("chain_getBlockHash", [20]),
			provider.send<number>("chain_getBlockHash", [30]),
		])

		expect(results).toEqual([10, 20, 30])
	})

	// The common case must not depend on the server supporting batches at all.
	it("sends a lone call as a plain request object", async () => {
		provider = new BatchingHttpProvider(await start(), {}, openBucket())

		await provider.send("chain_getHeader", [])

		expect(node.requests()).toBe(1)
		expect(node.seen[0].batched).toBe(false)
	})

	// Sequential callers have nothing to coalesce — the window cannot invent concurrency.
	it("does not coalesce calls that are awaited one after another", async () => {
		provider = new BatchingHttpProvider(await start(), {}, openBucket())

		await provider.send("chain_getHeader", [])
		await provider.send("chain_getHeader", [])

		expect(node.requests()).toBe(2)
		expect(node.seen.every((request) => !request.batched)).toBe(true)
	})

	it("splits a burst larger than the batch size across requests", async () => {
		const batching = new BatchingHttpProvider(await start(), {}, openBucket(), 4)
		provider = batching

		const results = await Promise.all(
			Array.from({ length: 10 }, (_, i) => batching.send<number>("chain_getBlockHash", [i])),
		)

		expect(results).toHaveLength(10)
		expect(node.requests()).toBe(3)
		expect(node.seen.map((request) => request.methods.length)).toEqual([4, 4, 2])
	})

	// `--rpc-disable-batch-requests`. Every call must still be answered.
	it("falls back to one request per call when the server refuses batches", async () => {
		provider = new BatchingHttpProvider(await start({ mode: "disabled" }), {}, openBucket())

		const results = await Promise.all([
			provider.send<string>("chain_getBlockHash", [1]),
			provider.send<string>("chain_getBlockHash", [2]),
		])

		expect(results).toHaveLength(2)
		expect(provider.batchingSupported).toBe(false)
		// The refused batch, then the two singles it was retried as.
		expect(node.seen.map((request) => request.batched)).toEqual([true, false, false])
	})

	it("stops attempting batches once refused", async () => {
		provider = new BatchingHttpProvider(await start({ mode: "disabled" }), {}, openBucket())

		await Promise.all([provider.send("a", []), provider.send("b", [])])
		const afterFirst = node.requests()
		await Promise.all([provider.send("c", []), provider.send("d", [])])

		expect(node.seen.slice(afterFirst).every((request) => !request.batched)).toBe(true)
	})

	// `--rpc-max-batch-request-len`. The batch shrinks rather than the calls failing.
	it("halves the batch and retries when the server says it was too large", async () => {
		const batching = new BatchingHttpProvider(await start({ mode: { maxLen: 3 } }), {}, openBucket(), 8)
		provider = batching

		const results = await Promise.all(
			Array.from({ length: 8 }, (_, i) => batching.send<number>("chain_getBlockHash", [i])),
		)

		expect(results).toHaveLength(8)
		// 8 refused, then 4 refused, then 2 + 2 + 2 + 2 accepted.
		expect(node.seen.map((request) => request.methods.length)).toEqual([8, 4, 2, 2, 2, 2])
	})

	it("charges the rate limiter once per request, not once per call", async () => {
		const batching = new BatchingHttpProvider(await start(), {}, new TokenBucket(4, 1))
		provider = batching

		const started = Date.now()
		await Promise.all(Array.from({ length: 12 }, (_, i) => batching.send("chain_getBlockHash", [i])))

		// Twelve calls, one request, one token — so no waiting. Twelve tokens would have taken ~3s.
		expect(node.requests()).toBe(1)
		expect(Date.now() - started).toBeLessThan(500)
	})

	it("reports a call's own error without failing the rest of the batch", async () => {
		const url = await start({
			errorFor: (call) =>
				call.method === "bad" ? { code: -32601, message: "Method not found" } : undefined,
		})
		provider = new BatchingHttpProvider(url, {}, openBucket())

		const [bad, good] = await Promise.allSettled([provider.send("bad", []), provider.send("good", [])])

		expect(bad.status).toBe("rejected")
		const reason = (bad as PromiseRejectedResult).reason as Error & { code?: number }
		// `classifySubmissionError` reads `code` off the error, so the shape has to survive.
		expect(reason.code).toBe(-32601)
		expect(reason.message).toContain("Method not found")
		expect(good).toMatchObject({ status: "fulfilled", value: "result-good-2" })
	})

	// The poll recognises a rate limit by this message shape, so it has to survive the rewrite.
	it("keeps the base provider's message shape for a non-200 response", async () => {
		const url = await start({ status: { code: 429, message: "Too Many Requests" } })
		provider = new BatchingHttpProvider(url, {}, openBucket())

		await expect(provider.send("chain_getHeader", [])).rejects.toThrow("[429]: Too Many Requests")
	})

	it("fails every call in a batch when the request itself fails", async () => {
		provider = new BatchingHttpProvider(await start(), {}, openBucket())
		await node.close()

		const settled = await Promise.allSettled([provider.send("a", []), provider.send("b", [])])

		expect(settled.every((result) => result.status === "rejected")).toBe(true)
	})
})
