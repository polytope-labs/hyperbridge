import { ENV_CONFIG } from "@/constants"
import {
	getBlockTimestamp,
	getEvmBlockTimestamp,
	getSubstrateBlockTimestamp,
	replaceWebsocketWithHttp,
} from "@/utils/rpc.helpers"

describe("Get Substrate Block Timestamp", () => {
	const chain = "KUSAMA-4009"
	const blockHash = "0xfc53c051dd3adc9b564fcf0e6bcfa00ecdb8faddcd5dfbd9e84f8e9c1c6f2f28"

	beforeAll(async () => {
		const { ApiPromise, WsProvider } = await import("@polkadot/api")
		;(globalThis as any).api = await ApiPromise.create({ provider: new WsProvider(ENV_CONFIG[chain]) })
	})

	test("should get a valid milliseconds timestamp from a substrate block", async () => {
		const timestamp = await getSubstrateBlockTimestamp(blockHash)

		expect(new Date(Number(timestamp)).toISOString()).toBe("2025-04-30T17:38:18.000Z")
		expect(timestamp).toBe(1746034698000n)
	})

	test("should handle API errors gracefully", async () => {
		await expect(getSubstrateBlockTimestamp("0x00")).rejects.toThrow(
			"RPC error: createType(BlockHash):: Expected input with 32 bytes (256 bits), found 1 bytes",
		)
	})
})

describe("Get Evm Block Timestamp", () => {
	const chain = "EVM-97"
	const blockHash = "0xcf2a760fab352596b2c3774658bdf57ffa34b7e8d8fe691732fffca29f98f5ef"

	test("should fetch the block timestamp by querying the block and extracing the timestamp", async () => {
		const timestamp = await getEvmBlockTimestamp(blockHash, chain)

		expect(timestamp).toBe(1746112524n)
		expect(new Date(Number(timestamp * 1000n)).toISOString()).toBe("2025-05-01T15:15:24.000Z")
	})

	test("should handle API errors gracefully", async () => {
		await expect(
			getEvmBlockTimestamp("0x0000000000000000000000000000000000000000000000000000000000000000", chain),
		).rejects.toThrow('Unexpected response: No timestamp found in response {"jsonrpc":"2.0","id":1,"result":null}')
	})

	test("should handle invalid chain parameter", async () => {
		await expect(getEvmBlockTimestamp(blockHash, "UNKNOWN")).rejects.toThrow("No RPC URL found for chain: UNKNOWN")
	})
})

describe("Get Block Timestamp", () => {
	const chain = "KUSAMA-4009"
	const blockHash = "0xfc53c051dd3adc9b564fcf0e6bcfa00ecdb8faddcd5dfbd9e84f8e9c1c6f2f28"

	test("should use pick the appropriate function based on the chain and fetch the timestamp", async () => {
		expect(await getBlockTimestamp(blockHash, chain)).toBe(1746034698000n)
	})
})

describe("replaceWebsocketWithHttp", () => {
	test("should replace the websocket URL with an HTTP URL or throw an error", async () => {
		expect(replaceWebsocketWithHttp(ENV_CONFIG["KUSAMA-4009"])).toBe(
			"https://hyperbridge-paseo-rpc.blockops.network",
		)
		expect(() => replaceWebsocketWithHttp(ENV_CONFIG["UNKNOWN"])).toThrow()
		expect(replaceWebsocketWithHttp(ENV_CONFIG["EVM-97"])).toBe(
			"https://wandering-delicate-silence.bsc-testnet.quiknode.pro/74d3977082e2021a0e005e12dbdcbb6732ed74ee",
		)
		expect(replaceWebsocketWithHttp("https://")).toBe("https://")
		expect(replaceWebsocketWithHttp("ws://")).toBe("http://")
	})
})
