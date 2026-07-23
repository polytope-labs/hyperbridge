import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { readFileSync } from "fs"
import { resolve } from "path"
import { validateConfig, DEFAULT_CONFIRMATION_POLICIES, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"

const minimalStable = (): FillerTomlConfig => ({
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
	strategies: [
		{
			type: "stable",
			bpsCurve: [
				{ amount: "100", value: 100 },
				{ amount: "100000", value: 10 },
			],
		},
	],
	chains: [{ rpcUrls: ["https://eth-mainnet.g.alchemy.com/v2/key"], bundlerUrl: "https://bundler.example" }],
})

describe("validateConfig", () => {
	it("accepts a minimal stable config", () => {
		expect(() => validateConfig(minimalStable())).not.toThrow()
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

	it("rejects a missing signer", () => {
		const config = minimalStable()
		config.simplex.signer = undefined
		expect(() => validateConfig(config)).toThrow(/Signer configuration is required/)
	})

	it("allows a missing signer in global watch-only mode", () => {
		const config = minimalStable()
		config.simplex.signer = undefined
		config.simplex.watchOnly = true
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects a missing substratePrivateKey", () => {
		const config = minimalStable()
		config.simplex.substratePrivateKey = ""
		expect(() => validateConfig(config)).toThrow(/substratePrivateKey is required/)
	})

	it("rejects a missing hyperbridgeWsUrl", () => {
		const config = minimalStable()
		config.simplex.hyperbridgeWsUrl = ""
		expect(() => validateConfig(config)).toThrow(/hyperbridgeWsUrl is required/)
	})

	it("rejects an empty strategies list", () => {
		const config = minimalStable()
		config.strategies = []
		expect(() => validateConfig(config)).toThrow(/At least one strategy/)
	})

	it("rejects chains without rpcUrls or bundlerUrl", () => {
		const noRpc = minimalStable()
		noRpc.chains = [{ rpcUrls: [""], bundlerUrl: "https://bundler.example" }]
		expect(() => validateConfig(noRpc)).toThrow(/rpcUrls/)

		const noBundler = minimalStable()
		noBundler.chains = [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "" }]
		expect(() => validateConfig(noBundler)).toThrow(/bundlerUrl/)
	})

	it("rejects a stable strategy with fewer than 2 bpsCurve points", () => {
		const config = minimalStable()
		config.strategies = [{ type: "stable", bpsCurve: [{ amount: "100", value: 100 }] }]
		expect(() => validateConfig(config)).toThrow(/at least 2 points/)
	})

	it("rejects curve values the runtime policies reject at boot", () => {
		// fractional bps — InterpolatedCurve requires integers
		const fractionalBps = minimalStable()
		fractionalBps.strategies = [
			{
				type: "stable",
				bpsCurve: [
					{ amount: "100", value: 1.5 },
					{ amount: "1000", value: 50 },
				],
			},
		]
		expect(() => validateConfig(fractionalBps)).toThrow(/bpsCurve.*invalid/i)

		// negative FX prices — FillerPricePolicy rejects them (decimal.js counts 0 as
		// positive-signed, so 0 passes boot and therefore passes here: parity)
		for (const price of ["-3", "-0.5"]) {
			const badPrice = minimalStable()
			badPrice.strategies = [
				{
					type: "hyperfx",
					maxOrderUsd: 5000,
					token1: { "EVM-56": "0x1111111111111111111111111111111111111111" },
					bidPriceCurve: [{ amount: "100", price }],
				},
			]
			expect(() => validateConfig(badPrice)).toThrow(/bidPriceCurve.*invalid/i)
		}

		// negative confirmation counts
		const badConfirmations = minimalStable()
		badConfirmations.strategies = [
			{
				type: "stable",
				bpsCurve: [
					{ amount: "100", value: 100 },
					{ amount: "1000", value: 50 },
				],
				confirmationPolicies: {
					"1": {
						points: [
							{ amount: "100", value: -1 },
							{ amount: "1000", value: 5 },
						],
					},
				},
			},
		]
		expect(() => validateConfig(badConfirmations)).toThrow(/chain 1 is invalid/i)
	})

	it("rejects hyperfx without curves or uniswap positions", () => {
		const config = minimalStable()
		config.strategies = [
			{
				type: "hyperfx",
				maxOrderUsd: 5000,
				token1: { "EVM-56": "0x1111111111111111111111111111111111111111" },
			},
		]
		expect(() => validateConfig(config)).toThrow(/bid and\/or ask price curve/)
	})

	it("rejects hyperfx without token1 entries", () => {
		const config = minimalStable()
		config.strategies = [
			{
				type: "hyperfx",
				maxOrderUsd: 5000,
				token1: {},
				bidPriceCurve: [{ amount: "100", price: "1580" }],
				askPriceCurve: [{ amount: "100", price: "1560" }],
			},
		]
		expect(() => validateConfig(config)).toThrow(/token1/)
	})

	it("rejects invalid allowlist addresses", () => {
		const config = minimalStable()
		config.allowlist = { users: ["not-an-address"] }
		expect(() => validateConfig(config)).toThrow(/invalid address/)
	})

	it("keeps built-in confirmation policy defaults for the core five mainnets", () => {
		expect(Object.keys(DEFAULT_CONFIRMATION_POLICIES).sort()).toEqual(["1", "137", "42161", "56", "8453"].sort())
	})
})
