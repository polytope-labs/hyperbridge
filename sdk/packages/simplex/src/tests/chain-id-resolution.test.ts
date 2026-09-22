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

afterEach(() => {
	vi.unstubAllGlobals()
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
