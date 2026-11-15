import { describe, it, expect, afterEach, vi } from "vitest"
import { resolveChainConfigs } from "@/services/FillerConfigService"

/**
 * Startup chain-id resolution.
 *
 * A filler configured with a dozen free endpoints per chain used to fail boot
 * whenever any one of them answered 429 — a state public RPCs are in routinely,
 * and one the quorum client handles by benching the endpoint.
 */

const OK = (chainId: number) =>
	new Response(JSON.stringify({ jsonrpc: "2.0", id: 1, result: `0x${chainId.toString(16)}` }), { status: 200 })

/** Answers per URL: a number resolves, a status code fails the request. */
function stubFetch(answers: Record<string, number | { status: number }>) {
	vi.stubGlobal("fetch", async (url: string | URL) => {
		const answer = answers[String(url)]
		if (answer === undefined) throw new Error(`unexpected fetch to ${url}`)
		if (typeof answer === "number") return OK(answer)
		return new Response("Too Many Requests", { status: answer.status })
	})
}

/** Answers per URL after a delay in milliseconds. Needs fake timers. */
function stubSlowFetch(answers: Record<string, { chainId: number; afterMs: number }>) {
	vi.stubGlobal("fetch", async (url: string | URL) => {
		const { chainId, afterMs } = answers[String(url)]
		await new Promise((resolve) => setTimeout(resolve, afterMs))
		return OK(chainId)
	})
}

afterEach(() => {
	vi.unstubAllGlobals()
	vi.useRealTimers()
})

describe("resolveChainConfigs", () => {
	it("refuses a silent endpoint unless the caller opts into tolerance", async () => {
		// Runtime endpoint edits take this path: an endpoint that never answered has
		// never been checked against the chain it would serve, and on a quiet range a
		// wrong-chain endpoint returns [] like an honest one — so it could join a
		// quorum on "no events".
		stubFetch({ "https://a.example": 56, "https://b.example": { status: 429 } })

		await expect(
			resolveChainConfigs([{ rpcUrls: ["https://a.example", "https://b.example"], bundlerUrl: "" }]),
		).rejects.toThrow(/must report their chainId before use/)
	})

	it("boots on the endpoints that answered, tolerating a throttled one", async () => {
		stubFetch({
			"https://a.example": { status: 429 },
			"https://b.example": 56,
			"https://c.example": 56,
		})

		const [resolved] = await resolveChainConfigs(
			[{ rpcUrls: ["https://a.example", "https://b.example", "https://c.example"], bundlerUrl: "" }],
			{ tolerateUnreachable: true },
		)

		expect(resolved.chainId).toBe(56)
		// The throttled endpoint is kept: it was probed, not adopted, and the quorum
		// client decides per call whether it can vote.
		expect(resolved.rpcUrls).toEqual(["https://a.example", "https://b.example", "https://c.example"])
	})

	it("still refuses a chain where nothing answered", async () => {
		stubFetch({ "https://a.example": { status: 429 }, "https://b.example": { status: 503 } })

		await expect(
			resolveChainConfigs([{ rpcUrls: ["https://a.example", "https://b.example"], bundlerUrl: "" }], {
				tolerateUnreachable: true,
			}),
		).rejects.toThrow(/No configured RPC endpoint could report its chainId/)
	})

	it("still refuses endpoints that disagree about the chain", async () => {
		stubFetch({ "https://a.example": 56, "https://b.example": 1 })

		await expect(
			resolveChainConfigs([{ rpcUrls: ["https://a.example", "https://b.example"], bundlerUrl: "" }]),
		).rejects.toThrow(/disagree on chainId/)
	})

	it("starts on the one endpoint that answered when the others failed", async () => {
		stubFetch({
			"https://a.example": 56,
			"https://b.example": { status: 429 },
			"https://c.example": { status: 503 },
		})

		const [resolved] = await resolveChainConfigs(
			[{ rpcUrls: ["https://a.example", "https://b.example", "https://c.example"], bundlerUrl: "" }],
			{ tolerateUnreachable: true },
		)

		expect(resolved.chainId).toBe(56)
	})

	it("waits for a slow endpoint at startup, and refuses the chain when it serves another one", async () => {
		// Two fast endpoints agree, and the third answers last for another chain. Reads go to
		// every endpoint, so starting without its answer would let it serve this chain.
		vi.useFakeTimers()
		stubSlowFetch({
			"https://slow.example": { chainId: 84532, afterMs: 5_000 },
			"https://b.example": { chainId: 8453, afterMs: 100 },
			"https://c.example": { chainId: 8453, afterMs: 150 },
		})
		let settled = false
		const resolving = resolveChainConfigs(
			[{ rpcUrls: ["https://slow.example", "https://b.example", "https://c.example"], bundlerUrl: "" }],
			{ tolerateUnreachable: true },
		)
		const outcome = expect(resolving).rejects.toThrow(/disagree on chainId/)
		void resolving.catch(() => {}).finally(() => {
			settled = true
		})

		await vi.advanceTimersByTimeAsync(4_999)
		expect(settled).toBe(false)
		await vi.advanceTimersByTimeAsync(1)
		await outcome
	})

	it("waits for every endpoint when silence is not tolerated", async () => {
		vi.useFakeTimers()
		stubSlowFetch({
			"https://slow.example": { chainId: 1, afterMs: 5_000 },
			"https://b.example": { chainId: 56, afterMs: 100 },
		})
		const resolving = resolveChainConfigs([
			{ rpcUrls: ["https://slow.example", "https://b.example"], bundlerUrl: "" },
		])
		const outcome = expect(resolving).rejects.toThrow(/disagree on chainId/)
		await vi.advanceTimersByTimeAsync(5_000)
		await outcome
	})

	it("ignores a silent endpoint when judging agreement", async () => {
		// The one that could not answer must not be read as a disagreement.
		stubFetch({
			"https://a.example": 8453,
			"https://b.example": { status: 429 },
			"https://c.example": 8453,
		})

		const [resolved] = await resolveChainConfigs(
			[{ rpcUrls: ["https://a.example", "https://b.example", "https://c.example"], bundlerUrl: "" }],
			{ tolerateUnreachable: true },
		)
		expect(resolved.chainId).toBe(8453)
	})
})
