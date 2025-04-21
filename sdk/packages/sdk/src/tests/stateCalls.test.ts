import "log-timestamp"

import type { HexString, IEvmConfig, ISubstrateConfig } from "@/types"
import { http, createPublicClient, getContract, toHex } from "viem"

import { bscTestnet } from "viem/chains"
import PING_MODULE from "@/abis/pingModule"

import { getChain } from "@/chain"

describe.sequential("State Queries", () => {
	let bscConfig: IEvmConfig
	let hyperbridgeConfig: ISubstrateConfig

	beforeAll(async () => {
		const { bscIsmpHostAddress } = await bscSetup()
		bscConfig = {
			consensusStateId: "BSC0",
			rpcUrl: process.env.BSC_CHAPEL!,
			stateMachineId: "EVM-97",
			host: bscIsmpHostAddress,
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
			const chain = await getChain(bscConfig)
			const stateMachineId = { stateId: { Kusama: 4009 }, consensusStateId: toHex("PASO") }
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
			const chain = await getChain(hyperbridgeConfig)
			const stateMachineId = { stateId: { Evm: 97 }, consensusStateId: toHex("BSC0") }
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
			const chain = await getChain(hyperbridgeConfig)
			const stateMachineId = { stateId: { Evm: 97 }, consensusStateId: toHex("BSC0") }
			const challengePeriod = await chain.challengePeriod(stateMachineId)
			expect(challengePeriod).toBe(BigInt(0))
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read challenge period on EVM", async () => {
		try {
			const chain = await getChain(bscConfig)
			const stateMachineId = { stateId: { Kusama: 4009 }, consensusStateId: toHex("PASO") }
			const challengePeriod = await chain.challengePeriod(stateMachineId)
			expect(challengePeriod).toBe(BigInt(0))
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read state machine update time on EVM", async () => {
		try {
			const chain = await getChain(bscConfig)
			const stateMachineId = { stateId: { Kusama: 4009 }, consensusStateId: toHex("PASO") }
			const latestHeight = await chain.latestStateMachineHeight(stateMachineId)
			const stateMachineheight = { id: stateMachineId, height: latestHeight }
			const updateTime = await chain.stateMachineUpdateTime(stateMachineheight)
			expect(updateTime).toBeGreaterThan(0)
		} catch (err) {
			console.error(err)
			expect(err).toBeUndefined()
		}
	}, 300_000)

	it("should read state machine update time on substrate", async () => {
		try {
			const chain = await getChain(hyperbridgeConfig)
			const stateMachineId = { stateId: { Evm: 97 }, consensusStateId: toHex("BSC0") }
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

async function bscSetup() {
	const bscTestnetClient = createPublicClient({
		chain: bscTestnet,
		transport: http(process.env.BSC_CHAPEL),
	})

	const bscPing = getContract({
		address: process.env.PING_MODULE_ADDRESS! as HexString,
		abi: PING_MODULE.ABI,
		client: { public: bscTestnetClient },
	})

	const bscIsmpHostAddress = await bscPing.read.host()

	return {
		bscIsmpHostAddress,
	}
}
