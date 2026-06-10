import { describe, it, expect, vi } from "vitest"
import { parseUnits } from "viem"
import { AaveV3FundingPlanner } from "@/funding/aaveV3/AaveV3FundingPlanner"
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
const AUSDT = `0x${"f".repeat(40)}` as HexString

const aTokenOf: Record<string, HexString> = { [USDC.toLowerCase()]: AUSDC, [USDT.toLowerCase()]: AUSDT }

/**
 * Planner backed by mocked clients. `wallet` maps an underlying asset to the
 * solver's wallet balance (drives the sweep); aToken/reserve reads return large
 * constants so hydrate succeeds.
 */
function makePlanner(wallet: Record<string, bigint>, reserves: AaveV3OutputFundingConfig["reservesByChain"][string]) {
	const sendTransaction = vi.fn(async (_tx: { to: HexString; data: HexString; value: bigint }) => "0xtx" as HexString)

	const publicClient = {
		async readContract({
			address,
			functionName,
			args,
		}: {
			address: HexString
			functionName: string
			args?: readonly unknown[]
		}) {
			if (functionName === "getReserveData")
				return { aTokenAddress: aTokenOf[(args?.[0] as string).toLowerCase()] }
			if (functionName === "decimals") return 6
			if (functionName === "balanceOf") {
				const who = (args?.[0] as string)?.toLowerCase()
				// asset.balanceOf(solver) → wallet balance (what the sweep reads)
				if (who === SOLVER.toLowerCase() && wallet[address.toLowerCase()] !== undefined) {
					return wallet[address.toLowerCase()]
				}
				return 10_000_000_000n // large constant for aToken/reserve reads during hydrate
			}
			throw new Error(`unexpected readContract: ${functionName}`)
		},
		waitForTransactionReceipt: async () => ({ status: "success" }),
	}

	const walletClient = { chain: { id: 8453 }, sendTransaction }
	const clientManager = {
		getPublicClient: () => publicClient,
		getWalletClient: () => walletClient,
	} as unknown as ChainClientManager
	const configService = { getAaveV3PoolAddress: () => POOL } as unknown as FillerConfigService

	const planner = new AaveV3FundingPlanner(clientManager, { reservesByChain: { [CHAIN]: reserves } }, configService)
	return { planner, sendTransaction }
}

const u = (n: string) => parseUnits(n, 6)

describe("AaveV3FundingPlanner.sweepExcessToPool", () => {
	it("supplies the excess above threshold", async () => {
		const { planner, sendTransaction } = makePlanner({ [USDC.toLowerCase()]: u("5000") }, [
			{ asset: USDC, threshold: "3000" },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToPool(CHAIN)

		expect(sendTransaction).toHaveBeenCalledOnce()
		expect(sendTransaction.mock.calls[0][0].to.toLowerCase()).toBe(SOLVER.toLowerCase())
	})

	it("does nothing when balance is at or below threshold", async () => {
		const { planner, sendTransaction } = makePlanner({ [USDC.toLowerCase()]: u("3000") }, [
			{ asset: USDC, threshold: "3000" },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToPool(CHAIN)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("skips dust below minSweep", async () => {
		// excess = 5 USDC, below the default 10 minSweep.
		const { planner, sendTransaction } = makePlanner({ [USDC.toLowerCase()]: u("3005") }, [
			{ asset: USDC, threshold: "3000" },
		])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToPool(CHAIN)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("does not sweep a reserve with no threshold (withdraw-only)", async () => {
		const { planner, sendTransaction } = makePlanner({ [USDC.toLowerCase()]: u("9999") }, [{ asset: USDC }])
		await planner.initialise(SOLVER)
		await planner.sweepExcessToPool(CHAIN)

		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("batches both reserves' excess into a single transaction", async () => {
		const { planner, sendTransaction } = makePlanner(
			{ [USDC.toLowerCase()]: u("5000"), [USDT.toLowerCase()]: u("4000") },
			[
				{ asset: USDC, threshold: "3000" },
				{ asset: USDT, threshold: "3000" },
			],
		)
		await planner.initialise(SOLVER)
		await planner.sweepExcessToPool(CHAIN)

		expect(sendTransaction).toHaveBeenCalledOnce()
	})
})
