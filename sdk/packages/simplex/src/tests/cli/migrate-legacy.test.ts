import { describe, it, expect } from "vitest"
import { migrateLegacyConfig } from "@/cli/init/migrate-legacy"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { DEFAULT_ORDERBOOK_URLS } from "@/config/defaults"
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

const policy = (value: number) => ({
	points: [
		{ amount: "100", value },
		{ amount: "10000", value: value + 2 },
	],
})

describe("migrateLegacyConfig", () => {
	it("drops [[strategies]] and adds an orderbook, leaving a config that loads", () => {
		const config = legacyConfig([
			{ type: "stable", bpsCurve: [{ amount: "100", value: 100 }] },
			{
				type: "hyperfx",
				token1: { "EVM-8453": CNGN_ON_BASE },
				askPriceCurve: [{ amount: "100", price: "1550" }],
				vault: { uniswapV4: { positions: [{ chain: "EVM-8453", tokenId: "42" }], side: "ask" } },
			},
		])
		const notes = migrateLegacyConfig(config)
		expect("strategies" in config).toBe(false)
		expect(config.orderbook).toEqual({ url: DEFAULT_ORDERBOOK_URLS.mainnet })
		expect(notes.some((n) => n.includes("dropped [[strategies]]"))).toBe(true)
		expect(() => validateConfig(config)).not.toThrow()
	})

	it("moves per-strategy confirmation policies top-level without overriding existing ones", () => {
		const config = legacyConfig([
			{ type: "hyperfx", confirmationPolicies: { "8453": policy(1), "EVM-56": policy(5) } },
			{ type: "stable", confirmationPolicies: { "EVM-8453": policy(9) } },
		])
		config.confirmationPolicies = { "EVM-56": policy(3) }
		migrateLegacyConfig(config)
		expect(config.confirmationPolicies).toEqual({ "56": policy(3), "8453": policy(1) })
	})

	it("leaves a config without [[strategies]] as it was", () => {
		const config = legacyConfig([])
		expect(migrateLegacyConfig(config)).toEqual([])
		expect("strategies" in config).toBe(false)
		expect(config.orderbook).toBeUndefined()
	})
})
