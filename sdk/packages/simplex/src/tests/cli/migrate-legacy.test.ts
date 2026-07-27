import { describe, it, expect } from "vitest"
import { migrateLegacyConfig } from "@/cli/init/migrate-legacy"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"

const CNGN_ON_BASE = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F"

function legacyConfig(strategies: unknown[]): FillerTomlConfig {
	return {
		simplex: {
			signer: { type: SignerType.PrivateKey, key: "0xab" },
			maxConcurrentOrders: 5,
			queue: { maxRechecks: 10, recheckDelayMs: 30000 },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://example",
		},
		...({ strategies } as object),
		chains: [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }],
	} as FillerTomlConfig
}

describe("migrateLegacyConfig", () => {
	it("maps a stable strategy to below-par same-token pairs", () => {
		const config = legacyConfig([
			{
				type: "stable",
				bpsCurve: [
					{ amount: "100", value: 100 },
					{ amount: "100000", value: 10 },
				],
			},
		])
		const notes = migrateLegacyConfig(config)
		expect(notes.some((n) => n.includes("stable strategy"))).toBe(true)
		expect("strategies" in config).toBe(false)
		expect(config.pairs?.map((p) => `${p.token0}/${p.token1}`)).toEqual(["USDC/USDC", "USDT/USDT"])
		// 100 bps at $100, 10 bps at $100k
		expect(config.pairs?.[0].askPriceCurve).toEqual([
			{ amount: "100", price: "0.99" },
			{ amount: "100000", price: "0.999" },
		])
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("maps a hyperfx strategy to a registry-symbol pair and moves engine settings top-level", () => {
		const config = legacyConfig([
			{
				type: "hyperfx",
				maxOrderUsd: 5000,
				token1: { "EVM-8453": CNGN_ON_BASE },
				bidPriceCurve: [{ amount: "100", price: "1580" }],
				askPriceCurve: [{ amount: "100", price: "1550" }],
				spreadBps: 40,
				confirmationPolicies: {
					"8453": {
						points: [
							{ amount: "100", value: 1 },
							{ amount: "10000", value: 3 },
						],
					},
				},
				vault: { uniswapV4: { positions: [{ chain: "EVM-8453", tokenId: "42" }], side: "ask" } },
			},
		])
		const notes = migrateLegacyConfig(config)
		const pair = config.pairs?.find((p) => p.token1 === "CNGN")
		expect(pair).toBeDefined()
		expect(pair?.token0).toBe("USDC")
		expect(pair?.maxOrderSize).toBe("5000")
		expect(config.confirmationPolicies?.["8453"]).toBeDefined()
		expect(config.vault?.uniswapV4?.positions?.[0]?.tokenId).toBe("42")
		expect(config.vault?.uniswapV4?.spreadBps).toBe(40)
		// The pair has static curves, so the migrated `side` must be dropped —
		// the engine rejects the combination.
		expect(config.vault?.uniswapV4?.side).toBeUndefined()
		expect(notes.some((n) => n.includes("dropped [vault.uniswapV4].side"))).toBe(true)
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("clamps out-of-range bps margins into the valid ask-price range", () => {
		const config = legacyConfig([
			{
				type: "stable",
				bpsCurve: [
					{ amount: "100", value: 0 },
					{ amount: "100000", value: 10_000 },
				],
			},
		])
		const notes = migrateLegacyConfig(config)
		// 0 bps maps to par (rejected: must be < 1); 10000 bps maps to 0.
		expect(config.pairs?.[0].askPriceCurve).toEqual([
			{ amount: "100", price: "0.9999" },
			{ amount: "100000", price: "0.0001" },
		])
		expect(notes.some((n) => n.includes("clamped"))).toBe(true)
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("respects a legacy maxOrderUsd on the stable strategy", () => {
		const config = legacyConfig([
			{ type: "stable", maxOrderUsd: 25_000, bpsCurve: [{ amount: "100", value: 50 }] },
		])
		const notes = migrateLegacyConfig(config)
		expect(config.pairs?.map((p) => p.maxOrderSize)).toEqual(["25000", "25000"])
		expect(notes.some((n) => n.includes("order cap 25000 from the legacy maxOrderUsd"))).toBe(true)
	})

	it("keeps an unmatched exotic address map as an [assets] escape hatch", () => {
		const config = legacyConfig([
			{
				type: "hyperfx",
				maxOrderUsd: 1000,
				token1: { "EVM-8453": "0x9999999999999999999999999999999999999999" },
				bidPriceCurve: [{ amount: "0", price: "6" }],
				askPriceCurve: [{ amount: "0", price: "5.8" }],
			},
		])
		const notes = migrateLegacyConfig(config)
		expect(notes.some((n) => n.includes("[assets.TOKEN1]"))).toBe(true)
		expect(config.assets?.TOKEN1).toEqual({ "EVM-8453": "0x9999999999999999999999999999999999999999" })
		expect(config.pairs?.some((p) => p.token1 === "TOKEN1")).toBe(true)
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("numbers a second unresolvable exotic TOKEN2 instead of colliding", () => {
		const config = legacyConfig([
			{
				type: "hyperfx",
				maxOrderUsd: 1000,
				token1: { "EVM-8453": "0x9999999999999999999999999999999999999999" },
				askPriceCurve: [{ amount: "0", price: "5.8" }],
			},
			{
				type: "hyperfx",
				maxOrderUsd: 2000,
				token1: { "EVM-8453": "0x8888888888888888888888888888888888888888" },
				askPriceCurve: [{ amount: "0", price: "3.1" }],
			},
		])
		migrateLegacyConfig(config)
		expect(config.assets?.TOKEN1).toEqual({ "EVM-8453": "0x9999999999999999999999999999999999999999" })
		expect(config.assets?.TOKEN2).toEqual({ "EVM-8453": "0x8888888888888888888888888888888888888888" })
		expect(config.pairs?.map((p) => `${p.token0}/${p.token1}`)).toEqual(["USDC/TOKEN1", "USDC/TOKEN2"])
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("skips a hyperfx strategy whose USDC pair is already declared", () => {
		const config = legacyConfig([
			{
				type: "hyperfx",
				maxOrderUsd: 1000,
				token1: { "EVM-8453": CNGN_ON_BASE },
				askPriceCurve: [{ amount: "0", price: "1550" }],
			},
			{
				type: "hyperfx",
				maxOrderUsd: 2000,
				token1: { "EVM-8453": CNGN_ON_BASE },
				askPriceCurve: [{ amount: "0", price: "1560" }],
			},
		])
		const notes = migrateLegacyConfig(config)
		expect(config.pairs?.filter((p) => p.token1 === "CNGN")).toHaveLength(1)
		expect(notes.some((n) => n.includes("skipped"))).toBe(true)
		expect(() => validateConfig(config)).not.toThrow()
	})
})
