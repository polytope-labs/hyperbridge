import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { readFileSync } from "fs"
import { resolve } from "path"
import {
	assertConfirmationCoverage,
	validateConfig,
	validateRebalancingConfig,
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

	// A solver without a Hyperbridge key bids on the orderbook alone.
	it("accepts a config without a substratePrivateKey or hyperbridgeWsUrl", () => {
		const config = minimalConfig()
		config.simplex.substratePrivateKey = undefined
		config.simplex.hyperbridgeWsUrl = undefined
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("rejects a hyperbridgeWsUrl missing while a substratePrivateKey is set", () => {
		const config = minimalConfig()
		config.simplex.hyperbridgeWsUrl = ""
		expect(() => validateConfig(config)).toThrow(/hyperbridgeWsUrl is required when simplex.substratePrivateKey is set/)
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

describe("validateConfig [simplex.overfillProtection]", () => {
	// Removed, but ignored rather than refused: `simplex init` wrote the section
	// uncommented, and rejecting it would crash-loop those deployments. Boot warns.
	it("accepts the removed section in any form, including an empty table", () => {
		for (const overfillProtection of [{}, { maxOverfillBps: 500 }, { maxConsecutiveClamps: 3 }]) {
			const config = minimalConfig()
			config.simplex.overfillProtection = overfillProtection
			expect(() => validateConfig(config)).not.toThrow()
		}
	})

	it("accepts a config without it, or with it undefined", () => {
		expect(() => validateConfig(minimalConfig())).not.toThrow()
		const config = minimalConfig()
		config.simplex.overfillProtection = undefined
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("accepts a TOML file that still carries the section", () => {
		// The same parse every entry point (`simplex run`, `simplex init`, the desktop) reads the file with.
		const example = readFileSync(resolve(__dirname, "../../../filler-config-example.toml"), "utf-8")
		const config = parse(`${example}\n[simplex.overfillProtection]\nmaxOverfillBps = 500\n`) as FillerConfigFile
		config.simplex.substratePrivateKey = "seed"
		config.simplex.hyperbridgeWsUrl = "wss://example"
		for (const chain of config.chains) chain.rpcUrls = ["https://rpc.example"]
		expect(() => validateConfig(config)).not.toThrow()
	})
})

describe("validateConfig [rebalancing]", () => {
	const hub = { asset: "USDC", chain: "EVM-8453", threshold: "6000", minBalance: "5000" }
	const withHubs = (...hubs: unknown[]): FillerTomlConfig => ({
		...minimalConfig(),
		rebalancing: { hubs } as FillerTomlConfig["rebalancing"],
	})

	it("accepts a hub per asset, and a section with no hubs", () => {
		expect(() => validateConfig(withHubs(hub, { ...hub, asset: "USDT", chain: "EVM-42161" }))).not.toThrow()
		expect(() => validateConfig({ ...minimalConfig(), rebalancing: { intervalMs: 60_000 } })).not.toThrow()
	})

	it("rejects the settings from before hubs with the new shape", () => {
		const config = minimalConfig()
		config.rebalancing = {
			triggerPercentage: 0.5,
			baseBalances: { USDC: { "8453": "10000" } },
		} as unknown as FillerTomlConfig["rebalancing"]
		expect(() => validateConfig(config)).toThrow(
			/rebalancing\.triggerPercentage and rebalancing\.baseBalances were removed[\s\S]*\[\[rebalancing\.hubs\]\]\nasset      = "USDC"/,
		)
	})

	it("rejects a rebalancing value that is not a section", () => {
		const config = { ...minimalConfig(), rebalancing: true as unknown as FillerTomlConfig["rebalancing"] }
		expect(() => validateConfig(config)).toThrow(/rebalancing must be a \[rebalancing\] section; got boolean/)
	})

	it("rejects two hubs for one asset, however the symbol is written", () => {
		expect(() => validateConfig(withHubs(hub, { ...hub, asset: "usdc", chain: "EVM-137" }))).toThrow(
			/USDC has more than one hub/,
		)
	})

	it.each(["8453", "base", "EVM-", "EVM-08453", undefined])("rejects a hub chain of %j", (chain) => {
		expect(() => validateConfig(withHubs({ ...hub, chain }))).toThrow(/USDC 'chain' must be "EVM-<chainId>"/)
	})

	it("rejects a threshold that is not above the minimum balance", () => {
		expect(() => validateConfig(withHubs({ ...hub, threshold: "5000" }))).toThrow(
			/USDC 'threshold' must be greater than 'minBalance'/,
		)
	})

	it.each(["0", "0.00"])("rejects a minimum balance of %j, as a vault's is", (minBalance) => {
		expect(() => validateConfig(withHubs({ ...hub, minBalance }))).toThrow(
			/USDC 'minBalance' must be a quoted positive amount/,
		)
	})

	it.each([6000, "-1", "abc", "", undefined])("rejects a threshold of %j", (threshold) => {
		// A bare TOML number is refused too: the amount is scaled as a decimal string.
		expect(() => validateConfig(withHubs({ ...hub, threshold }))).toThrow(
			/USDC 'threshold' must be a quoted positive amount with at most two decimals, such as "5000"/,
		)
	})

	it("rejects amounts finer than a cent, which could differ on paper and scale to one balance", () => {
		expect(() => validateConfig(withHubs({ ...hub, threshold: "5000.0000004", minBalance: "5000.0000001" }))).toThrow(
			/at most two decimals/,
		)
		expect(() => validateConfig(withHubs({ ...hub, threshold: "5000.01", minBalance: "5000" }))).not.toThrow()
	})

	// 2^31 ms and above: Node arms such a timer for one millisecond.
	it.each([0, 9_999, 60_000.5, "300000", 2_147_483_648, 9_999_999_999])("rejects an intervalMs of %j", (intervalMs) => {
		const config = withHubs(hub)
		config.rebalancing = { ...config.rebalancing, intervalMs: intervalMs as number }
		expect(() => validateConfig(config)).toThrow(/rebalancing\.intervalMs must be an integer from 10000 to 2147483647/)
	})

	it("requires every hub to be a chain the solver runs, once the chains are known", () => {
		expect(() => validateRebalancingConfig({ hubs: [hub] }, [8453, 137])).not.toThrow()
		expect(() => validateRebalancingConfig({ hubs: [hub] }, [1, 137])).toThrow(
			/USDC's hub EVM-8453 is not a chain this solver runs/,
		)
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

	it("refuses a quoted delegateHeartbeats, which would leave delegation on", () => {
		const orderbook = { url: "https://example", delegateHeartbeats: "false" as unknown as boolean }
		expect(() => validateConfig(withOrderbook(orderbook))).toThrow(
			/delegateHeartbeats must be true or false; got the string "false"/,
		)
	})
})
