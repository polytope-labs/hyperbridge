import { describe, it, expect } from "vitest"
import {
	FillerConfigService,
	DEFAULT_BLOCK_SCAN_INTERVAL_SECONDS,
	MIN_BLOCK_SCAN_INTERVAL_SECONDS,
	type ResolvedChainConfig,
} from "@/services/FillerConfigService"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"

const CHAINS: ResolvedChainConfig[] = [{ chainId: 8453, rpcUrls: ["https://mainnet.base.org"], bundlerUrl: "https://b" }]

function baseToml(blockScanIntervalSeconds?: number): FillerTomlConfig {
	return {
		pairs: [{ token0: "USDC", token1: "USDC", maxOrderSize: "100", askPriceCurve: [{ amount: "0", price: "0.99" }] }],
		simplex: {
			signer: { type: "privateKey", key: `0x${"11".repeat(32)}` },
			maxConcurrentOrders: 1,
			queue: { maxRechecks: 1, recheckDelayMs: 1000 },
			substratePrivateKey: "0xabc",
			hyperbridgeWsUrl: "wss://example",
			blockScanIntervalSeconds,
		},
		chains: [{ rpcUrls: ["https://mainnet.base.org"], bundlerUrl: "https://b" }],
	} as FillerTomlConfig
}

describe("block scan interval", () => {
	it("defaults to 3 seconds when unset", () => {
		const service = new FillerConfigService(CHAINS, { maxConcurrentOrders: 1 })
		expect(DEFAULT_BLOCK_SCAN_INTERVAL_SECONDS).toBe(3)
		expect(service.getBlockScanIntervalMs()).toBe(3000)
	})

	it("converts the configured seconds to the milliseconds setInterval wants", () => {
		const service = new FillerConfigService(CHAINS, { maxConcurrentOrders: 1, blockScanIntervalSeconds: 5 })
		expect(service.getBlockScanIntervalMs()).toBe(5000)
	})

	it("supports sub-second polling", () => {
		const service = new FillerConfigService(CHAINS, { maxConcurrentOrders: 1, blockScanIntervalSeconds: 0.5 })
		expect(service.getBlockScanIntervalMs()).toBe(500)
	})

	it("accepts valid intervals through validateConfig", () => {
		expect(() => validateConfig(baseToml(5))).not.toThrow()
		expect(() => validateConfig(baseToml(0.5))).not.toThrow()
		expect(() => validateConfig(baseToml(MIN_BLOCK_SCAN_INTERVAL_SECONDS))).not.toThrow()
		expect(() => validateConfig(baseToml(undefined))).not.toThrow()
	})

	it("rejects intervals that would spin the scanner", () => {
		// 0 and negatives make setInterval fire every tick, draining an RPC budget
		// in minutes.
		for (const bad of [0, -1, 0.05]) {
			expect(() => validateConfig(baseToml(bad))).toThrow(/blockScanIntervalSeconds/)
		}
	})

	it("rejects a non-numeric interval", () => {
		expect(() => validateConfig(baseToml(Number.NaN))).toThrow(/blockScanIntervalSeconds/)
	})
})
