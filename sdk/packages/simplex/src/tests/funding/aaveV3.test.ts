import { describe, it, expect } from "vitest"
import { decodeFunctionData } from "viem"
import { AaveV3FundingPlanner } from "@/funding/aaveV3/AaveV3FundingPlanner"
import { AAVE_V3_POOL_ABI } from "@/config/abis/AaveV3Pool"
import type { AaveV3OutputFundingConfig } from "@/funding/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { HexString } from "@hyperbridge/sdk"

const CHAIN = "EVM-8453"
const POOL = `0x${"c".repeat(40)}` as HexString
const SOLVER = `0x${"d".repeat(40)}` as HexString
const USDC = `0x${"a".repeat(40)}` as HexString
const AUSDC = `0x${"b".repeat(40)}` as HexString
const USDT = `0x${"e".repeat(40)}` as HexString

/** Live balances the mocked client returns. Mutable so tests can vary scenarios. */
interface Balances {
	aTokenBalance: bigint
	availableReserve: bigint
}

/**
 * Builds a planner backed by a mocked public client. `readContract` dispatches
 * on functionName/address to return controlled reserve data and balances.
 */
function makePlanner(balances: Balances) {
	const fakeClient = {
		async readContract({
			address,
			functionName,
			args,
		}: {
			address: HexString
			functionName: string
			args?: readonly unknown[]
		}) {
			if (functionName === "getReserveData") return { aTokenAddress: AUSDC }
			if (functionName === "decimals") return 6
			if (functionName === "balanceOf") {
				const who = (args?.[0] as string)?.toLowerCase()
				// aToken.balanceOf(solver) → solver's supply position
				if (address.toLowerCase() === AUSDC.toLowerCase() && who === SOLVER.toLowerCase()) {
					return balances.aTokenBalance
				}
				// underlying.balanceOf(aToken) → pool's withdrawable reserve
				if (address.toLowerCase() === USDC.toLowerCase() && who === AUSDC.toLowerCase()) {
					return balances.availableReserve
				}
				return 0n
			}
			throw new Error(`unexpected readContract: ${functionName}`)
		},
	}

	const clientManager = { getPublicClient: () => fakeClient } as unknown as ChainClientManager
	const configService = { getAaveV3PoolAddress: () => POOL } as unknown as FillerConfigService
	const config: AaveV3OutputFundingConfig = { reservesByChain: { [CHAIN]: [{ asset: USDC }] } }
	return new AaveV3FundingPlanner(clientManager, config, configService)
}

describe("AaveV3FundingPlanner", () => {
	it("plans a withdraw call for the full deficit when liquidity is ample", async () => {
		const planner = makePlanner({ aTokenBalance: 1_000_000n, availableReserve: 1_000_000n })
		await planner.initialise(SOLVER)

		const need = 250_000n
		const { calls, credited } = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), need)

		expect(credited).toBe(need)
		expect(calls).toHaveLength(1)
		expect(calls[0].target.toLowerCase()).toBe(POOL.toLowerCase())
		expect(calls[0].value).toBe(0n)

		const decoded = decodeFunctionData({ abi: AAVE_V3_POOL_ABI, data: calls[0].data })
		expect(decoded.functionName).toBe("withdraw")
		expect((decoded.args[0] as string).toLowerCase()).toBe(USDC.toLowerCase())
		expect(decoded.args[1]).toBe(need)
		expect((decoded.args[2] as string).toLowerCase()).toBe(SOLVER.toLowerCase())
	})

	it("caps the withdraw at the pool's withdrawable reserve", async () => {
		// Solver supplied 1M but only 300k is unborrowed and withdrawable.
		const planner = makePlanner({ aTokenBalance: 1_000_000n, availableReserve: 300_000n })
		await planner.initialise(SOLVER)

		const { calls, credited } = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), 500_000n)

		expect(credited).toBe(300_000n)
		const decoded = decodeFunctionData({ abi: AAVE_V3_POOL_ABI, data: calls[0].data })
		expect(decoded.args[1]).toBe(300_000n)
	})

	it("returns a no-op for tokens that are not configured Aave reserves", async () => {
		const planner = makePlanner({ aTokenBalance: 1_000_000n, availableReserve: 1_000_000n })
		await planner.initialise(SOLVER)

		const res = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDT.toLowerCase(), 100_000n)
		expect(res.calls).toHaveLength(0)
		expect(res.credited).toBe(0n)
	})

	it("does not over-source across concurrent plans (consume accounting)", async () => {
		const planner = makePlanner({ aTokenBalance: 400_000n, availableReserve: 400_000n })
		await planner.initialise(SOLVER)

		const first = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), 300_000n)
		const second = await planner.planWithdrawalForToken(CHAIN, SOLVER, USDC.toLowerCase(), 300_000n)

		expect(first.credited).toBe(300_000n)
		// Only 100k remains unreserved after the first plan; refresh sees the same
		// on-chain balance (withdrawal not yet executed) so consume must hold it back.
		expect(second.credited).toBe(100_000n)
	})

	it("returns null for exotic-token pricing (stable-only venue)", async () => {
		const planner = makePlanner({ aTokenBalance: 1n, availableReserve: 1n })
		await planner.initialise(SOLVER)
		expect(await planner.getExoticTokenPrice(CHAIN, USDC)).toBeNull()
	})

	it("rejects invalid reserve config", () => {
		expect(() => AaveV3FundingPlanner.validateConfig([{ chain: "", asset: USDC }])).toThrow()
		expect(() => AaveV3FundingPlanner.validateConfig([{ chain: CHAIN, asset: "" }])).toThrow()
		expect(() => AaveV3FundingPlanner.validateConfig([{ chain: CHAIN, asset: USDC }])).not.toThrow()
	})
})
