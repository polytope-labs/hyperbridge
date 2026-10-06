import { afterAll, beforeAll, describe, expect, it } from "vitest"
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
// SolverAccount validates against.
let rpc: MockRpc

beforeAll(async () => {
	rpc = await startMockRpc({ chainId: CHAIN_ID })
})

afterAll(() => {
	rpc.close()
})

function config(): FillerTomlConfig {
	return {
		simplex: { substratePrivateKey: SUBSTRATE_SEED, hyperbridgeWsUrl: "ws://127.0.0.1:1" },
		pairs: [{ token0: "USDC", token1: "USDC" }],
		chains: [{ rpcUrls: [rpc.url], bundlerUrl: rpc.url }],
		orderbook: { url: "https://orderbook.example/graphql" },
	}
}

describe("boot bundler EntryPoint check", () => {
	it("refuses to start when a filling chain's bundler does not list its EntryPoint", async () => {
		const boot = bootFiller(config(), {
			loggers: new LoggerContext({}),
			scanners: { orders: stubOrderScanner([CHAIN_ID]) },
			data: new MemoryDataStore(),
			ownsData: true,
			signer: privateKeySigner(`0x${"22".repeat(32)}`),
		})

		await expect(boot).rejects.toThrow(
			/Bundler 127\.0\.0\.1:\d+ for Base \(EVM-8453\) does not support EntryPoint 0x[0-9a-fA-F]{40}; it lists 0x0000000071727De22E5E9d8BAf0edAc6f37da032/,
		)
	})
})
