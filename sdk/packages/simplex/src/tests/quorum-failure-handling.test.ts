import { describe, it, expect, beforeEach } from "vitest"
import {
	QuorumPublicClient,
	QuorumError,
	benchFor,
	isMalformedResponse,
	isStaleHead,
	isSuspendableRateLimit,
	MALFORMED_RESPONSE_SUSPENSION_MS,
	STALE_HEAD_SUSPENSION_MS,
} from "@/services/QuorumPublicClient"

/**
 * Failure handling in the quorum client, offline.
 *
 * Every case here came from a 24h mainnet run whose BSC scanner stopped for six
 * minutes with nothing in the log: a slow minority held the call open while the
 * scanner's own retry loop repeated it, and two failure shapes kept endpoints in
 * the query set that could not answer.
 */

const BASE_CHAIN_ID = 8453
const URLS = ["https://a.example", "https://b.example", "https://c.example"]

/** A viem-shaped error: the provider's words live in `details`, not `message`. */
function providerError(details: string): Error {
	return Object.assign(new Error("HTTP request failed."), { details })
}

describe("benchable failure classification", () => {
	it("treats a non-JSON body as malformed rather than a mystery failure", () => {
		// thirdweb answers its quota notice in plain text; viem reports the parse failure.
		const err = new Error(`Unexpected token 'Y', "You are usi"... is not valid JSON`)
		expect(isMalformedResponse(err)).toBe(true)
		// Not a rate limit as far as the classifier can tell — the body never reached JSON.
		expect(isSuspendableRateLimit(err)).toBe(false)
		expect(benchFor(err)).toEqual({
			durationMs: MALFORMED_RESPONSE_SUSPENSION_MS,
			reason: "returned a malformed response",
		})
	})

	it("treats a lagging head as its own case, benched only briefly", () => {
		const err = providerError("block 123347517 is beyond the latest block 123347510")
		expect(isStaleHead(err)).toBe(true)
		expect(benchFor(err)).toEqual({ durationMs: STALE_HEAD_SUSPENSION_MS, reason: "behind the quorum head" })
	})

	it("keeps rate limits ahead of the new cases, with the provider's own Retry-After", () => {
		const err = Object.assign(new Error("HTTP request failed."), {
			status: 429,
			details: "too many requests",
			headers: { get: (name: string) => (name === "retry-after" ? "2" : null) },
		})
		expect(benchFor(err)).toEqual({ durationMs: 2_000, reason: "rate-limited" })
	})

	it("leaves ordinary failures per-call", () => {
		expect(benchFor(providerError("execution reverted"))).toBeNull()
		expect(benchFor(providerError("Internal server error"))).toBeNull()
		// A result-cap complaint is a property of the query, not of the endpoint.
		expect(benchFor(Object.assign(new Error("limit exceeded"), { code: -32005 }))).toBeNull()
	})
})

describe("quorum call deadline", () => {
	beforeEach(() => {
		QuorumPublicClient.clearAllSuspensions()
	})

	it("fails loudly instead of waiting out a hung endpoint", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, URLS, undefined, 60)
		// Two answer, one never does. The bar is 3-of-3, so it can never be met and
		// the old code would have waited for the transport's full budget.
		const clients = client.clients as unknown as Array<{ getBlockNumber: () => Promise<bigint> }>
		clients[0].getBlockNumber = async () => 100n
		clients[1].getBlockNumber = async () => 100n
		clients[2].getBlockNumber = () => new Promise(() => {})

		const started = Date.now()
		await expect(client.getBlockNumber()).rejects.toThrow(QuorumError)
		expect(Date.now() - started).toBeLessThan(2_000)
	})

	it("names the endpoint it was still waiting on", async () => {
		const client = new QuorumPublicClient(BASE_CHAIN_ID, URLS, undefined, 60)
		const clients = client.clients as unknown as Array<{ getBlockNumber: () => Promise<bigint> }>
		clients[0].getBlockNumber = async () => 100n
		clients[1].getBlockNumber = async () => 100n
		clients[2].getBlockNumber = () => new Promise(() => {})

		await expect(client.getBlockNumber()).rejects.toThrow(/c\.example.*quorum deadline/s)
	})

	it("still decides on the fast responders when they reach the bar", async () => {
		// 2-of-2 over two endpoints: the third URL is absent, so a straggler cannot
		// hold up a call the answering endpoints already settle.
		const client = new QuorumPublicClient(BASE_CHAIN_ID, URLS.slice(0, 2), undefined, 60)
		const clients = client.clients as unknown as Array<{ getBlockNumber: () => Promise<bigint> }>
		clients[0].getBlockNumber = async () => 500n
		clients[1].getBlockNumber = async () => 501n

		// The quorum head is the lower of the two: conservative for every consumer.
		await expect(client.getBlockNumber()).resolves.toBe(500n)
	})
})
