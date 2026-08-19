import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { readFileSync } from "fs"
import { resolve } from "path"
import {
	assertConfirmationCoverage,
	validateConfig,
	type FillerConfigFile,
	type FillerTomlConfig,
} from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"

const minimalConfig = (): FillerTomlConfig => ({
	simplex: {
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
		const config = parse(example) as FillerConfigFile
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

	// `SimplexConfig` has no signer field: the signer is an argument to
	// `Simplex.start`, and the `[simplex.signer]` block belongs to the binary's
	// file format (`FillerConfigFile`). Neither its presence nor its absence is
	// this validator's business — boot rejects a missing signer instead.
	it("ignores a [simplex.signer] block parsed from a config file", () => {
		const config = minimalConfig() as FillerConfigFile
		config.simplex.signer = { type: SignerType.PrivateKey, key: "0xab" }
		expect(() => validateConfig(config)).not.toThrow()
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

	// Pair rules (same-token invariants, crossed books, anchoring, curve
	// requirements) are unit-tested in pairs.test.ts; this case only proves
	// validateConfig forwards `pairs` and `assets` to validatePairConfigs.
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

	// The gate constructs the same policies boot does, so anything boot rejects
	// fails at config time — not first on `simplex run`.
	it("rejects fractional confirmation values at the gate, like boot does", () => {
		const config = minimalConfig()
		config.confirmationPolicies = {
			"1": {
				points: [
					{ amount: "1", value: 1.5 },
					{ amount: "100", value: 3 },
				],
			},
		}
		expect(() => validateConfig(config)).toThrow(/non-negative integer/)
	})

	it("gates confirmation coverage like boot: defaults cover mainnets, testnets need entries", () => {
		expect(() => assertConfirmationCoverage(undefined, [1, 137, 42161, 56, 8453, 130])).not.toThrow()
		expect(() => assertConfirmationCoverage(undefined, [11155111])).toThrow(/No confirmation policy/)
		expect(() =>
			assertConfirmationCoverage(
				{
					"11155111": {
						points: [
							{ amount: "100", value: 1 },
							{ amount: "10000", value: 2 },
						],
					},
				},
				[11155111],
			),
		).not.toThrow()
	})

	it("rejects invalid curve amounts at the gate, like the price policy does", () => {
		const config = minimalConfig()
		config.pairs = [
			{
				token0: "USDC",
				token1: "CNGN",
				maxOrderSize: "1000",
				askPriceCurve: [{ amount: "-5", price: "1550" }],
			},
		]
		expect(() => validateConfig(config)).toThrow(/askPriceCurve — .*invalid amount/)
	})
})
