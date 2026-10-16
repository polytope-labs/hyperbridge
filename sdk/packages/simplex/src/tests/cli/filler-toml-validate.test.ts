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
	orderbook: { url: "https://orderbook.example/graphql" },
	simplex: {
		maxConcurrentOrders: 5,
		queue: { maxRechecks: 10, recheckDelayMs: 30000 },
		substratePrivateKey: "seed phrase here",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
	},
	chains: [{ rpcUrls: ["https://eth-mainnet.g.alchemy.com/v2/key"] }],
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

	it("loads an older config that still carries [[pairs]]", () => {
		const config = { ...minimalConfig(), pairs: [{ token0: "USDC", token1: "USDC" }] }
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects chains without rpcUrls and needs no bundlerUrl", () => {
		const noRpc = minimalConfig()
		noRpc.chains = [{ rpcUrls: [""] }]
		expect(() => validateConfig(noRpc)).toThrow(/rpcUrls/)

		const noBundler = minimalConfig()
		noBundler.chains = [{ rpcUrls: ["https://rpc.example"] }]
		expect(() => validateConfig(noBundler)).not.toThrow()

		// An older config still naming one loads; the value is not used.
		const legacy = minimalConfig()
		legacy.chains = [{ rpcUrls: ["https://rpc.example"], bundlerUrl: "https://bundler.example" }]
		expect(() => validateConfig(legacy)).not.toThrow()
	})

	it("rejects malformed confirmation policy keys and short point lists", () => {
		const badKey = minimalConfig()
		badKey.confirmationPolicies = { ethereum: { points: [{ amount: "1", value: 1 }, { amount: "2", value: 2 }] } }
		expect(() => validateConfig(badKey)).toThrow(/not a chain id/)

		const shortPoints = minimalConfig()
		shortPoints.confirmationPolicies = { "1": { points: [{ amount: "1", value: 1 }] } }
		expect(() => validateConfig(shortPoints)).toThrow(/at least 2 points/)
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

})

describe("validateConfig [rebalancing]", () => {
	const withRebalancing = (triggerPercentage: number, base = "12000"): FillerTomlConfig => ({
		...minimalConfig(),
		rebalancing: { triggerPercentage, baseBalances: { USDC: { "8453": base } } },
	})

	it.each([0, 1, -0.1, Number.NaN])("rejects a triggerPercentage of %s", (trigger) => {
		expect(() => validateConfig(withRebalancing(trigger))).toThrow(/triggerPercentage must be a number between 0 and 1/)
	})

	it("says so when triggerPercentage is not a number at all", () => {
		// A TOML `triggerPercentage = "0.5"` parses as a string.
		expect(() => validateConfig(withRebalancing("0.5" as unknown as number))).toThrow(
			'rebalancing.triggerPercentage must be a number between 0 and 1 (exclusive); got the string "0.5"',
		)
	})

	it("accepts a triggerPercentage strictly between 0 and 1", () => {
		expect(() => validateConfig(withRebalancing(0.5))).not.toThrow()
	})

	it.each(["0", "-5", "abc", "Infinity"])("rejects a base balance of %s", (base) => {
		expect(() => validateConfig(withRebalancing(0.5, base))).toThrow(/baseBalances\.USDC for chain 8453/)
	})
})

describe("validateConfig [orderbook]", () => {
	const withOrderbook = (orderbook: FillerTomlConfig["orderbook"]): FillerTomlConfig => ({
		...minimalConfig(),
		orderbook,
	})

	it("refuses a config with no orderbook at all", () => {
		const { orderbook: _dropped, ...noOrderbook } = minimalConfig()
		expect(() => validateConfig(noOrderbook as FillerTomlConfig)).toThrow(/an \[orderbook\] section is required/)
	})

	it("accepts a well-formed block", () => {
		expect(() =>
			validateConfig(
				withOrderbook({
					url: "https://orderbook.example/graphql",
					defaultTtlSecs: 1800,
					reconcileIntervalSecs: 300,
					requestTimeoutMs: 10000,
				}),
			),
		).not.toThrow()
	})

	it("requires a url, rather than failing on the first posting", () => {
		expect(() => validateConfig(withOrderbook({ url: "" }))).toThrow(/orderbook.url is required/)
	})

	it("refuses a ttl below the orderbook's own floor", () => {
		expect(() => validateConfig(withOrderbook({ url: "https://example", defaultTtlSecs: 60 }))).toThrow(
			/defaultTtlSecs must be an integer >= 900/,
		)
	})
})
