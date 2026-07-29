import { describe, it, expect } from "vitest"
import { pickAnchorStable } from "@/config/wizard-guards"
import { assertPairSymbolsResolve, type PairAddressResolver } from "@/config/pairs"

describe("pickAnchorStable", () => {
	it("prefers USDC when no stable has a market against the symbol", () => {
		expect(pickAnchorStable([{ token0: "CNGN", token1: "CNGN" }], "CNGN")).toBe("USDC")
	})

	it("skips a stable with an existing orientation, in either direction", () => {
		expect(pickAnchorStable([{ token0: "USDC", token1: "CNGN" }], "CNGN")).toBe("USDT")
		expect(pickAnchorStable([{ token0: "CNGN", token1: "USDC" }], "CNGN")).toBe("USDT")
	})

	it("returns null when every stable is taken", () => {
		const pairs = [
			{ token0: "USDC", token1: "CNGN" },
			{ token0: "USDT", token1: "CNGN" },
			{ token0: "CNGN", token1: "DAI" },
		]
		expect(pickAnchorStable(pairs, "CNGN")).toBeNull()
	})
})

describe("assertPairSymbolsResolve", () => {
	const resolver = (addresses: Record<string, Record<string, string>>): PairAddressResolver => ({
		getAddress: (symbol, chain) => addresses[symbol.trim().toUpperCase()]?.[chain] ?? null,
	})

	it("accepts symbols that resolve somewhere and to distinct contracts", () => {
		const registry = resolver({
			USDC: { "EVM-8453": "0x1111111111111111111111111111111111111111" },
			CNGN: { "EVM-8453": "0x2222222222222222222222222222222222222222" },
		})
		expect(() =>
			assertPairSymbolsResolve(
				[{ token0: "USDC", token1: "CNGN", maxOrderSize: "1000" }],
				registry,
				["EVM-8453", "EVM-1"],
			),
		).not.toThrow()
	})

	it("rejects a symbol deployed on none of the configured chains", () => {
		const registry = resolver({
			USDC: { "EVM-8453": "0x1111111111111111111111111111111111111111" },
			CNGN: { "EVM-56": "0x2222222222222222222222222222222222222222" },
		})
		expect(() =>
			assertPairSymbolsResolve([{ token0: "USDC", token1: "CNGN", maxOrderSize: "1000" }], registry, ["EVM-8453"]),
		).toThrow(/'CNGN' does not resolve/)
	})

	it("rejects two symbols aliasing the same contract on a chain", () => {
		const shared = "0x1111111111111111111111111111111111111111"
		const registry = resolver({
			USDC: { "EVM-8453": shared },
			FAKE: { "EVM-8453": shared },
		})
		expect(() =>
			assertPairSymbolsResolve([{ token0: "USDC", token1: "FAKE", maxOrderSize: "1000" }], registry, ["EVM-8453"]),
		).toThrow(/both resolve to/)
	})
})
