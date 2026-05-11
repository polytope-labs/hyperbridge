import { ENV_CONFIG } from "@/constants"
import {
	getBlockTimestamp,
	getContractCallInput,
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

describe("Get Contract Call Input", () => {
	const chain = "EVM-56"
	const directCallTxHash = "0x2f6b062f23e3dd611211afd60b4d4ab91bb2995f7ffa34a30a79541447d3ecd5" // Fill Order hash
	const directCallTargetContractAddress = "0x1A4ee689A004B10210A1dF9f24A387Ea13359aCF" // IGV1 address

	const nestedCallTxHash = "0x6990b2ed686c3f3a3e4a7fd63880aa68d022e449afedd3f33f9a6e3025215394"
	const nestedCallTargetContractAddress = "0xef7FfbC192b26561d334c874335542B01cB09b57"

	test("should fetch the input (calldata) for a target contract call within nested calls", async () => {
		const input = await getContractCallInput(nestedCallTxHash, nestedCallTargetContractAddress, chain)

		expect(input).toBeDefined()
		if (input) {
			expect(typeof input).toBe("string")
			expect(input.startsWith("0x")).toBe(true)
			expect(input.length).toBeGreaterThan(2)
		}
	}, 60000)

	test("should return null when transaction directly calls target contract", async () => {
		const result = await getContractCallInput(directCallTxHash, directCallTargetContractAddress, chain)

		expect(result).toBeNull()
	}, 60000)

	test("should handle invalid transaction hash", async () => {
		await expect(
			getContractCallInput(
				"0x0000000000000000000000000000000000000000000000000000000000000000",
				directCallTargetContractAddress,
				chain,
			),
		).rejects.toThrow()
	}, 60000)

	test("should return null when target contract not found in nested calls", async () => {
		const result = await getContractCallInput(nestedCallTxHash, "0x0000000000000000000000000000000000000000", chain)
		expect(result).toBeNull()
	}, 60000)

	test("should handle invalid chain parameter", async () => {
		await expect(
			getContractCallInput(nestedCallTxHash, nestedCallTargetContractAddress, "UNKNOWN"),
		).rejects.toThrow("No RPC URL found for chain: UNKNOWN")
	})
})
