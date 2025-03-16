import { ApiPromise, WsProvider } from "@polkadot/api"
import { fetchStateCommitmentsEVM, fetchStateCommitmentsSubstrate } from "@/utils/state-machine.helper"
import { JsonRpcProvider } from "@ethersproject/providers"
import { stringify } from "safe-stable-stringify"

describe("fetchStateCommitmentsSubstrate Integration Test", () => {
	let api: ApiPromise

	test("fetches real state commitment on Hyperbridge", async () => {
		const provider = new WsProvider(process.env.HYPERBRIDGE_RPC_URL)
		const api = await ApiPromise.create({ provider })
		const result = await fetchStateCommitmentsSubstrate({
			api,
			stateMachineId: "EVM-10200",
			consensusStateId: "GNO0",
			height: 14803804n,
		})

		console.log("Gnosis commitment", stringify(result?.timestamp, null, 4))

		expect(result).toBeDefined()
		expect(result?.timestamp).toBeDefined()
		expect(result?.state_root).toBeInstanceOf(Uint8Array)

		await api.disconnect()
	}, 30000) // Increase timeout to 30 seconds
})

describe("fetchEvmStateCommitmentsFromHeight Integration Test", () => {
	test("fetches real state commitment on EVM chain", async () => {
		let client = new JsonRpcProvider(process.env.BSC_RPC_URL)
		// @ts-ignore
		global.chainId = 97
		const result = await fetchStateCommitmentsEVM({
			client,
			stateMachineId: "KUSAMA-4009",
			consensusStateId: "ETH0",
			height: 4120654n,
		})

		console.log("Bsc testnet", stringify(result?.timestamp, null, 4))

		expect(result).toBeDefined()
		expect(result?.timestamp).toBeDefined()
		expect(result?.state_root).toBeInstanceOf(Uint8Array)
	}, 30000) // Increase timeout to 30 seconds

	test("fetches real state commitment on EVM chain", async () => {
		let client = new JsonRpcProvider(process.env.BSC_RPC_URL)
		// @ts-ignore
		global.chainId = 56
		const result = await fetchStateCommitmentsEVM({
			client,
			stateMachineId: "POLKADOT-3367",
			consensusStateId: "ETH0",
			height: 4432117n,
		})

		console.log("Bsc mainnet", stringify(result?.timestamp, null, 4))

		expect(result).toBeDefined()
		expect(result?.timestamp).toBeDefined()
		expect(result?.state_root).toBeInstanceOf(Uint8Array)
	}, 30000) // Increase timeout to 30 seconds
})
