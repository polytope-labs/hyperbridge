import { fetchStateCommitmentsEVM } from "@/utils/state-machine.helper"
import { stringify } from "safe-stable-stringify"

describe("fetchEvmStateCommitmentsFromHeight Integration Test", () => {
	test("fetches real state commitment on EVM chain", async () => {
		// @ts-ignore
		global.chainId = 97
		const result = await fetchStateCommitmentsEVM({
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
		// @ts-ignore
		global.chainId = 56
		const result = await fetchStateCommitmentsEVM({
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
