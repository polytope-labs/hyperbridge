import "log-timestamp"

import type { HexString, IEvmConfig, ISubstrateConfig } from "@/types"
import { EvmChain, SubstrateChain } from "@/chain"
import { chainConfigs } from "@/configs/chain"

const BSC_CHAPEL_HOST = chainConfigs[97].addresses.Host as HexString

describe.sequential("State Queries", () => {
	let bscConfig: IEvmConfig
	let hyperbridgeConfig: ISubstrateConfig

	beforeAll(() => {
		bscConfig = {
			consensusStateId: "BSC0",
			rpcUrl: process.env.BSC_CHAPEL!,
			stateMachineId: "EVM-97",
			host: BSC_CHAPEL_HOST,
		}

		hyperbridgeConfig = {
			consensusStateId: "PAS0",
			stateMachineId: "KUSAMA-4009",
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			hasher: "Keccak",
		}
	})

	it("should read latest state machine height on EVM", async () => {
		try {
			const chain = EvmChain.fromParams({
				chainId: 97,
				rpcUrl: bscConfig.rpcUrl,
				host: bscConfig.host!,
				consensusStateId: bscConfig.consensusStateId,
			})
			const stateMachineId = { stateId: { Kusama: 4009 }, consensusStateId: "PASO" }
			const latestHeight = await chain.latestStateMachineHeight(stateMachineId)
			expect(latestHeight).toBeGreaterThan(0)

			console.log(latestHeight)
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read latest state machine height on Substrate", async () => {
		try {
			const chain = await SubstrateChain.connect(hyperbridgeConfig)
			const stateMachineId = { stateId: { Evm: 97 }, consensusStateId: "BSC0" }
			const latestHeight = await chain.latestStateMachineHeight(stateMachineId)
			expect(latestHeight).toBeGreaterThan(0)
			console.log(latestHeight)
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read challenge period on Substrate", async () => {
		try {
			const chain = await SubstrateChain.connect(hyperbridgeConfig)
			const stateMachineId = { stateId: { Evm: 97 }, consensusStateId: "BSC0" }
			const challengePeriod = await chain.challengePeriod(stateMachineId)
			expect(challengePeriod).toBe(BigInt(0))
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read challenge period on EVM", async () => {
		try {
			const chain = EvmChain.fromParams({
				chainId: 97,
				rpcUrl: bscConfig.rpcUrl,
				host: bscConfig.host!,
				consensusStateId: bscConfig.consensusStateId,
			})
			const stateMachineId = { stateId: { Kusama: 4009 }, consensusStateId: "PASO" }
			const challengePeriod = await chain.challengePeriod(stateMachineId)
			expect(challengePeriod).toBe(BigInt(0))
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read state machine update time on EVM", async () => {
		try {
			const chain = EvmChain.fromParams({
				chainId: 97,
				rpcUrl: bscConfig.rpcUrl,
				host: bscConfig.host!,
				consensusStateId: bscConfig.consensusStateId,
			})
			const stateMachineId = { stateId: { Kusama: 4009 }, consensusStateId: "PASO" }
			const latestHeight = await chain.latestStateMachineHeight(stateMachineId)
			const stateMachineheight = { id: stateMachineId, height: latestHeight }
			const updateTime = await chain.stateMachineUpdateTime(stateMachineheight)
			expect(updateTime).toBeGreaterThan(0)
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it.skip("should read state machine update time on substrate", async () => {
		try {
			const chain = await SubstrateChain.connect(hyperbridgeConfig)
			const stateMachineId = { stateId: { Evm: 97 }, consensusStateId: "BSC0" }
			const latestHeight = await chain.latestStateMachineHeight(stateMachineId)
			const stateMachineheight = { id: stateMachineId, height: latestHeight }
			const updateTime = await chain.stateMachineUpdateTime(stateMachineheight)
			expect(updateTime).toBeGreaterThan(0)

			console.log(updateTime)
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)
})
