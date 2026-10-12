import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest"
import { bootFiller } from "@/core/boot"
import type { FillerTomlConfig } from "@/config/filler-toml"
import { LoggerContext } from "@/services/Logger"
import { MemoryDataStore } from "@/data/memory"
import { privateKeySigner } from "@/services/wallet"
import { startMockRpc, type MockRpc } from "../helpers/mock-rpc"
import { stubOrderScanner } from "../helpers/stub-scanner"

const CHAIN_ID = 8453
const SUBSTRATE_SEED = `0x${"11".repeat(32)}`

// The mock answers eth_supportedEntryPoints with EntryPoint v0.7 alone, which no
// SolverAccount validates against. It stands in for Hyperbridge's bundler on Base.
let rpc: MockRpc
const bundler = vi.hoisted(() => ({ url: "" }))

vi.mock("@/config/bundlers", () => ({
	HYPERBRIDGE_BUNDLER_URLS: {},
	hyperbridgeBundlerUrl: (chainId: number) => (chainId === 8453 && bundler.url ? bundler.url : undefined),
}))

beforeAll(async () => {
	rpc = await startMockRpc({ chainId: CHAIN_ID })
})

afterEach(() => {
	bundler.url = rpc.url
})

afterAll(() => {
	rpc.close()
})

function config(): FillerTomlConfig {
	return {
		simplex: { substratePrivateKey: SUBSTRATE_SEED, hyperbridgeWsUrl: "ws://127.0.0.1:1" },
		pairs: [{ token0: "USDC", token1: "USDC" }],
		chains: [{ rpcUrls: [rpc.url] }],
		orderbook: { url: "https://orderbook.example/graphql" },
	}
}

function boot() {
	return bootFiller(config(), {
		loggers: new LoggerContext({}),
		scanners: { orders: stubOrderScanner([CHAIN_ID]) },
		data: new MemoryDataStore(),
		ownsData: true,
		signer: privateKeySigner(`0x${"22".repeat(32)}`),
	})
}

describe("boot bundler EntryPoint check", () => {
	it("refuses to start when a filling chain's bundler does not list its EntryPoint", async () => {
		bundler.url = rpc.url
		await expect(boot()).rejects.toThrow(
			/Bundler 127\.0\.0\.1:\d+ for Base \(EVM-8453\) does not support EntryPoint 0x[0-9a-fA-F]{40}; it lists 0x0000000071727De22E5E9d8BAf0edAc6f37da032/,
		)
	})

	it("refuses to start when Hyperbridge runs no bundler for a filling chain", async () => {
		bundler.url = ""
		await expect(boot()).rejects.toThrow(/Hyperbridge runs no bundler for Base \(EVM-8453\).*set it watch-only/)
	})
})
