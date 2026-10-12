import "log-timestamp"

import { afterAll, beforeAll, describe, expect, it } from "vitest"
import type { HexString, StateMachineIdParams } from "@/types"
import { EvmChain, SubstrateChain } from "@/chain"
import { chainConfigs } from "@/configs/chain"

const BSC_CHAPEL_HOST = chainConfigs[97].addresses.Host as HexString

const HYPERBRIDGE_ON_BSC: StateMachineIdParams = { stateId: { Kusama: 4009 }, consensusStateId: "PAS0" }
const BSC_ON_HYPERBRIDGE: StateMachineIdParams = { stateId: { Evm: 97 }, consensusStateId: "BSC0" }

describe.sequential("State Queries", () => {
	let bsc: EvmChain
	let hyperbridge: SubstrateChain

	beforeAll(async () => {
		bsc = EvmChain.fromParams({
			chainId: 97,
			rpcUrl: process.env.BSC_CHAPEL!,
			host: BSC_CHAPEL_HOST,
			consensusStateId: "BSC0",
		})
		hyperbridge = await SubstrateChain.connect({
			consensusStateId: "PAS0",
			stateMachineId: "KUSAMA-4009",
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			hasher: "Keccak",
		})
	}, 60_000)

	afterAll(async () => {
		await hyperbridge?.disconnect()
	})

	it("should read latest state machine height on EVM", async () => {
		const latestHeight = await bsc.latestStateMachineHeight(HYPERBRIDGE_ON_BSC)
		expect(latestHeight).toBeGreaterThan(0)
		console.log(latestHeight)
	}, 300_000)

	it("should read latest state machine height on Substrate", async () => {
		const latestHeight = await hyperbridge.latestStateMachineHeight(BSC_ON_HYPERBRIDGE)
		expect(latestHeight).toBeGreaterThan(0)
		console.log(latestHeight)
	}, 300_000)

	it("should read challenge period on Substrate", async () => {
		const challengePeriod = await hyperbridge.challengePeriod(BSC_ON_HYPERBRIDGE)
		expect(challengePeriod).toBe(BigInt(0))
	}, 300_000)

	it("should read challenge period on EVM", async () => {
		const challengePeriod = await bsc.challengePeriod(HYPERBRIDGE_ON_BSC)
		expect(challengePeriod).toBe(BigInt(0))
	}, 300_000)

	it("should read state machine update time on EVM", async () => {
		const latestHeight = await bsc.latestStateMachineHeight(HYPERBRIDGE_ON_BSC)
		const updateTime = await bsc.stateMachineUpdateTime({ id: HYPERBRIDGE_ON_BSC, height: latestHeight })
		expect(updateTime).toBeGreaterThan(0)
	}, 300_000)

	it("should read state machine update time on substrate", async () => {
		const latestHeight = await hyperbridge.latestStateMachineHeight(BSC_ON_HYPERBRIDGE)
		const updateTime = await hyperbridge.stateMachineUpdateTime({ id: BSC_ON_HYPERBRIDGE, height: latestHeight })
		expect(updateTime).toBeGreaterThan(0)
		console.log(updateTime)
	}, 300_000)
})
