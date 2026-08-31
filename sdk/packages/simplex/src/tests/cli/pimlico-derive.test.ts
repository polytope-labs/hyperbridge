import { describe, it, expect } from "vitest"
import { parsePimlicoUrl, isPimlicoUrl, derivePimlicoBundler } from "@/cli/init/derive/pimlico"

describe("parsePimlicoUrl", () => {
	it("parses a valid bundler URL", () => {
		expect(parsePimlicoUrl("https://api.pimlico.io/v2/1/rpc?apikey=pim_abc")).toEqual({
			chainId: 1,
			apiKey: "pim_abc",
		})
	})

	it("tolerates extra query params", () => {
		expect(parsePimlicoUrl("https://api.pimlico.io/v2/8453/rpc?foo=bar&apikey=key")).toEqual({
			chainId: 8453,
			apiKey: "key",
		})
	})

	it("rejects missing apikey", () => {
		expect(parsePimlicoUrl("https://api.pimlico.io/v2/1/rpc")).toBeNull()
	})

	it("rejects non-numeric chain segments", () => {
		expect(parsePimlicoUrl("https://api.pimlico.io/v2/ethereum/rpc?apikey=key")).toBeNull()
	})

	it("rejects other hosts and schemes", () => {
		expect(parsePimlicoUrl("https://api.pimlico.io.evil.com/v2/1/rpc?apikey=key")).toBeNull()
		expect(parsePimlicoUrl("http://api.pimlico.io/v2/1/rpc?apikey=key")).toBeNull()
		expect(parsePimlicoUrl("not a url")).toBeNull()
	})
})

describe("isPimlicoUrl", () => {
	it("detects pimlico URLs", () => {
		expect(isPimlicoUrl("https://api.pimlico.io/v2/137/rpc?apikey=key")).toBe(true)
		expect(isPimlicoUrl("https://eth-mainnet.g.alchemy.com/v2/key")).toBe(false)
	})
})

describe("derivePimlicoBundler", () => {
	it("builds per-chain bundler URLs", () => {
		expect(derivePimlicoBundler("key", 42161)).toBe("https://api.pimlico.io/v2/42161/rpc?apikey=key")
	})

	it("round-trips with parsePimlicoUrl", () => {
		expect(parsePimlicoUrl(derivePimlicoBundler("k", 56))).toEqual({ chainId: 56, apiKey: "k" })
	})

	it("round-trips keys containing URL-special characters", () => {
		for (const apiKey of ["pim+key", "pim%2Bkey", "pim key", "pim=key&x"]) {
			const derived = derivePimlicoBundler(apiKey, 1)
			expect(parsePimlicoUrl(derived)).toEqual({ chainId: 1, apiKey })
			// deriving from the parsed key again must be stable
			expect(derivePimlicoBundler(parsePimlicoUrl(derived)!.apiKey, 1)).toBe(derived)
		}
	})
})
