import { ApiPromise, WsProvider } from "@polkadot/api"
import { fetchStateCommitmentsEVM, fetchStateCommitmentsSubstrate } from "@/utils/state-machine.helper"
import { JsonRpcProvider } from "@ethersproject/providers"

describe("fetchStateCommitmentsSubstrate Integration Test", () => {
	let api: ApiPromise

	beforeAll(async () => {
		const provider = new WsProvider("wss://hyperbridge-paseo-rpc.blockops.network")
		api = await ApiPromise.create({ provider })
	})

	afterAll(async () => {
		if (api) {
			await api.disconnect()
			await new Promise((resolve) => setTimeout(resolve, 1000)) // Give time for cleanup
		}
	})

	test("fetches real state commitment on Hyperbridge", async () => {
		const result = await fetchStateCommitmentsSubstrate({
			api,
			stateMachineId: "KUSAMA-2030",
			consensusStateId: "PARA",
			height: 1381783n,
		})

		console.log(result)

		expect(result).toBeDefined()
		expect(result?.timestamp).toBeDefined()
		expect(result?.state_root).toBeInstanceOf(Uint8Array)
	}, 30000) // Increase timeout to 30 seconds
})

describe("fetchEvmStateCommitmentsFromHeight Integration Test", () => {
	let client: JsonRpcProvider

	beforeAll(() => {
		client = new JsonRpcProvider(
			"https://wandering-delicate-silence.bsc-testnet.quiknode.pro/74d3977082e2021a0e005e12dbdcbb6732ed74ee",
		)
	})

	test("fetches real state commitment on EVM chain", async () => {
		const result = await fetchStateCommitmentsEVM({
			client,
			stateMachineId: "KUSAMA-4009",
			consensusStateId: "ETH0",
			height: 3663176n,
		})

		console.log(Number(result?.timestamp) / 2)

		expect(result).toBeDefined()
		expect(result?.timestamp).toBeDefined()
		expect(result?.state_root).toBeInstanceOf(Uint8Array)
	}, 30000) // Increase timeout to 30 seconds
})
