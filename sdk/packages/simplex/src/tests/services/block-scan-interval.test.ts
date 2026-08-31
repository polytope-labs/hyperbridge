import { describe, it, expect } from "vitest"
import { DEFAULT_BLOCK_SCAN_INTERVAL_SECONDS, MIN_BLOCK_SCAN_INTERVAL_SECONDS } from "@/services/FillerConfigService"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { ChainScanner } from "@/scanner/chain-scanner"
import { OrderScanner } from "@/scanner/order-scanner"

const TARGET = { chain: "EVM-8453", chainId: 8453, gateway: "0xAA" as const, rpcUrls: ["https://base.example"] }

/** The interval the scan loop was armed with — the value that actually paces RPC spend. */
const intervalOf = (scanner: ChainScanner) => (scanner as unknown as { scanIntervalMs: number }).scanIntervalMs

function baseToml(blockScanIntervalSeconds?: number): FillerTomlConfig {
	return {
		pairs: [{ token0: "USDC", token1: "USDC", maxOrderSize: "100", askPriceCurve: [{ amount: "0", price: "0.99" }] }],
		simplex: {
			signer: { type: "privateKey", key: `0x${"11".repeat(32)}` },
			maxConcurrentOrders: 1,
			substratePrivateKey: "0xabc",
			hyperbridgeWsUrl: "wss://example",
			blockScanIntervalSeconds,
		},
		chains: [{ rpcUrls: ["https://mainnet.base.org"], bundlerUrl: "https://b" }],
	} as FillerTomlConfig
}

describe("block scan interval", () => {
	it("defaults the scan loop to 3 seconds when unset", () => {
		expect(DEFAULT_BLOCK_SCAN_INTERVAL_SECONDS).toBe(3)
		expect(intervalOf(new ChainScanner(TARGET))).toBe(3000)
	})

	it("carries scanIntervalSecs from OrderScanner.create to the scan loop, in ms", async () => {
		const scanner = await OrderScanner.create({ chains: [{ ...TARGET }], scanIntervalSecs: 0.5 })
		const loop = (scanner as unknown as { scanners: Map<number, ChainScanner> }).scanners.get(8453)!
		expect(intervalOf(loop)).toBe(500)
		await scanner.close()
	})

	it("rejects a scanIntervalSecs below the shared floor", async () => {
		await expect(OrderScanner.create({ chains: [{ ...TARGET }], scanIntervalSecs: 0.05 })).rejects.toThrow(
			/scanIntervalSecs/,
		)
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
