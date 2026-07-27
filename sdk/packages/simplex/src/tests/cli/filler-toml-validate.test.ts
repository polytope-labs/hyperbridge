import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { readFileSync } from "fs"
import { resolve } from "path"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { DEFAULT_CONFIRMATION_POLICIES } from "@/config/interpolated-curve"
import { SignerType } from "@/services/wallet"

const minimalConfig = (): FillerTomlConfig => ({
	simplex: {
		signer: {
			type: SignerType.PrivateKey,
			key: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
		},
		maxConcurrentOrders: 5,
		queue: { maxRechecks: 10, recheckDelayMs: 30000 },
		substratePrivateKey: "seed phrase here",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
	},
	pairs: [
		{
			token0: "USDC",
			token1: "USDC",
			maxOrderSize: "100000",
			askPriceCurve: [
				{ amount: "100", price: "0.99" },
				{ amount: "100000", price: "0.999" },
			],
		},
	],
	chains: [{ rpcUrls: ["https://eth-mainnet.g.alchemy.com/v2/key"], bundlerUrl: "https://bundler.example" }],
})

describe("validateConfig", () => {
	it("accepts a minimal same-token pair config", () => {
		expect(() => validateConfig(minimalConfig())).not.toThrow()
	})

	it("accepts the example config shape parsed from TOML", () => {
		const example = readFileSync(resolve(__dirname, "../../../filler-config-example.toml"), "utf-8")
		const config = parse(example) as FillerTomlConfig
		config.simplex.signer = { type: SignerType.PrivateKey, key: "0xab" }
		config.simplex.substratePrivateKey = "seed"
		config.simplex.hyperbridgeWsUrl = "wss://example"
		for (const chain of config.chains) {
			chain.rpcUrls = ["https://rpc.example"]
			chain.bundlerUrl = "https://bundler.example"
		}
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects the removed [[strategies]] key with a migration message", () => {
		const config = minimalConfig() as FillerTomlConfig & { strategies?: unknown }
		config.strategies = [{ type: "stable" }]
		expect(() => validateConfig(config)).toThrow(/\[\[strategies\]\] was removed/)
	})

	it("rejects a missing signer", () => {
		const config = minimalConfig()
		config.simplex.signer = undefined
		expect(() => validateConfig(config)).toThrow(/Signer configuration is required/)
	})

	it("allows a missing signer in global watch-only mode", () => {
		const config = minimalConfig()
		config.simplex.signer = undefined
		config.simplex.watchOnly = true
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("allows a missing signer when the --watch-only flag forces watch-only", () => {
		const config = minimalConfig()
		config.simplex.signer = undefined
		expect(() => validateConfig(config, true)).not.toThrow()
	})

	it("rejects a missing substratePrivateKey", () => {
		const config = minimalConfig()
		config.simplex.substratePrivateKey = ""
		expect(() => validateConfig(config)).toThrow(/substratePrivateKey is required/)
	})

	it("rejects a missing hyperbridgeWsUrl", () => {
		const config = minimalConfig()
		config.simplex.hyperbridgeWsUrl = ""
		expect(() => validateConfig(config)).toThrow(/hyperbridgeWsUrl is required/)
	})

	it("rejects an empty pairs list", () => {
		const config = minimalConfig()
		config.pairs = []
		expect(() => validateConfig(config)).toThrow(/At least one \[\[pairs\]\] entry/)
	})

	it("rejects chains without rpcUrls or bundlerUrl", () => {
		const noRpc = minimalConfig()
		noRpc.chains = [{ rpcUrls: [""], bundlerUrl: "https://bundler.example" }]
		expect(() => validateConfig(noRpc)).toThrow(/rpcUrls/)

		const noBundler = minimalConfig()
		noBundler.chains = [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "" }]
		expect(() => validateConfig(noBundler)).toThrow(/bundlerUrl/)
	})

	it("rejects same-token pairs with a bid curve or prices at or above par", () => {
		const withBid = minimalConfig()
		withBid.pairs![0].bidPriceCurve = [{ amount: "100", price: "0.98" }]
		expect(() => validateConfig(withBid)).toThrow(/ask-only/)

		const atPar = minimalConfig()
		atPar.pairs![0].askPriceCurve = [{ amount: "100", price: "1" }]
		expect(() => validateConfig(atPar)).toThrow(/strictly below 1/)
	})

	it("rejects unknown symbols without an [assets] entry, accepts them with one", () => {
		const unknown = minimalConfig()
		unknown.pairs!.push({
			token0: "USDC",
			token1: "BRZ",
			maxOrderSize: "5000",
			bidPriceCurve: [{ amount: "100", price: "6" }],
			askPriceCurve: [{ amount: "100", price: "5.8" }],
		})
		expect(() => validateConfig(unknown)).toThrow(/unknown symbol 'BRZ'/)

		unknown.assets = { BRZ: { "EVM-137": "0x1111111111111111111111111111111111111111" } }
		expect(() => validateConfig(unknown)).not.toThrow()
	})

	it("accepts a stable-to-stable market with near-par uncrossed curves", () => {
		const config = minimalConfig()
		config.pairs!.push({
			token0: "USDC",
			token1: "USDT",
			maxOrderSize: "100000",
			bidPriceCurve: [{ amount: "0", price: "1.001" }],
			askPriceCurve: [{ amount: "0", price: "0.999" }],
		})
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects a crossed cross-asset book", () => {
		const crossed = minimalConfig()
		crossed.pairs!.push({
			token0: "USDC",
			token1: "CNGN",
			maxOrderSize: "5000",
			bidPriceCurve: [{ amount: "100", price: "1550" }],
			askPriceCurve: [{ amount: "100", price: "1560" }],
		})
		expect(() => validateConfig(crossed)).toThrow(/crossed|bid/i)
	})

	it("rejects an unanchored token0", () => {
		// ZARP/CNGN alone: neither side reaches a USD stable through any curve.
		const config = minimalConfig()
		config.pairs = [
			{
				token0: "ZARP",
				token1: "CNGN",
				maxOrderSize: "90000",
				bidPriceCurve: [{ amount: "0", price: "86" }],
				askPriceCurve: [{ amount: "0", price: "84" }],
			},
		]
		expect(() => validateConfig(config)).toThrow(/no USD anchor/)
	})

	it("accepts a transitively anchored token0 via a referenceOnly pair", () => {
		const config = minimalConfig()
		config.pairs = [
			{
				token0: "USDC",
				token1: "CNGN",
				referenceOnly: true,
				askPriceCurve: [{ amount: "0", price: "1565" }],
			},
			{
				token0: "ZARP",
				token1: "CNGN",
				maxOrderSize: "90000",
				bidPriceCurve: [{ amount: "0", price: "86" }],
				askPriceCurve: [{ amount: "0", price: "84" }],
			},
		]
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects a pair without curves unless venue pricing is configured", () => {
		const config = minimalConfig()
		config.pairs!.push({ token0: "USDC", token1: "CNGN", maxOrderSize: "5000" })
		expect(() => validateConfig(config)).toThrow(/bid and\/or ask price curve/)

		config.vault = {
			uniswapV4: { positions: [{ chain: "EVM-8453", tokenId: "123" }] },
		}
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects a curve-less venue-priced pair whose quote asset is not a USD stable", () => {
		const config = minimalConfig()
		config.pairs!.push(
			{
				token0: "USDC",
				token1: "CNGN",
				referenceOnly: true,
				askPriceCurve: [{ amount: "0", price: "1565" }],
			},
			{ token0: "CNGN", token1: "ZARP", maxOrderSize: "5000" },
		)
		config.vault = { uniswapV4: { positions: [{ chain: "EVM-8453", tokenId: "123" }] } }
		expect(() => validateConfig(config)).toThrow(/venue pricing needs a USD-stable token0/)
	})

	it("rejects malformed confirmation policy keys and short point lists", () => {
		const badKey = minimalConfig()
		badKey.confirmationPolicies = { ethereum: { points: [{ amount: "1", value: 1 }, { amount: "2", value: 2 }] } }
		expect(() => validateConfig(badKey)).toThrow(/not a chain id/)

		const shortPoints = minimalConfig()
		shortPoints.confirmationPolicies = { "1": { points: [{ amount: "1", value: 1 }] } }
		expect(() => validateConfig(shortPoints)).toThrow(/at least 2 points/)
	})

	it("rejects vault.uniswapV4 side without venue positions or with curves present", () => {
		const noPositions = minimalConfig()
		noPositions.vault = { uniswapV4: { side: "ask" } }
		expect(() => validateConfig(noPositions)).toThrow(/requires \[vault.uniswapV4\].positions/)

		const withCurves = minimalConfig()
		withCurves.vault = { uniswapV4: { side: "ask", positions: [{ chain: "EVM-8453", tokenId: "1" }] } }
		expect(() => validateConfig(withCurves)).toThrow(/only applies to pool pricing/)
	})

	it("rejects invalid allowlist addresses", () => {
		const config = minimalConfig()
		config.allowlist = { users: ["not-an-address"] }
		expect(() => validateConfig(config)).toThrow(/invalid address/)
	})

	it("keeps built-in confirmation policy defaults for the core mainnets", () => {
		expect(Object.keys(DEFAULT_CONFIRMATION_POLICIES).sort()).toEqual(
			["1", "137", "42161", "56", "8453", "130"].sort(),
		)
	})
})
