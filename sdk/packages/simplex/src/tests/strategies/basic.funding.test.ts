import { describe, it, expect, vi } from "vitest"
import { BasicFiller } from "@/strategies/basic"
import type { FundingVenue, FundingPlanResult } from "@/funding/types"
import type { Order, HexString, ERC7821Call } from "@hyperbridge/sdk"

const USDC = `0x${"a".repeat(40)}` as HexString
const USDC_B32 = `0x${"0".repeat(24)}${"a".repeat(40)}` as HexString
const SOLVER = `0x${"d".repeat(40)}` as HexString
const DUMMY_CALL: ERC7821Call = { target: USDC, value: 0n, data: "0x" as HexString }

/** Funding venue stub that credits a fixed amount and emits one call. */
function venue(credited: bigint): FundingVenue {
	return {
		name: "AaveV3",
		initialise: vi.fn(async () => {}),
		refresh: vi.fn(async () => {}),
		getExoticTokenPrice: vi.fn(async () => null),
		planWithdrawalForToken: vi.fn(
			async (): Promise<FundingPlanResult> =>
				credited > 0n ? { calls: [DUMMY_CALL], credited } : { calls: [], credited: 0n },
		),
	}
}

/**
 * Builds a BasicFiller with mocked deps and a wallet balance, exposing
 * `planFunding` and the cache spies for assertions.
 */
function makeFiller(walletBalance: bigint, venues: FundingVenue[]) {
	const setFundingPrepends = vi.fn()
	const clearFundingPrepends = vi.fn()

	const clientManager = {
		getPublicClient: () => ({ readContract: async () => walletBalance }),
	}
	const contractService = { cacheService: { setFundingPrepends, clearFundingPrepends } }
	const configService = { getMaxOverfillBps: () => 100n }
	const signer = { account: { address: SOLVER } }

	const filler = new BasicFiller(
		signer as never,
		configService as never,
		clientManager as never,
		contractService as never,
		{} as never,
		{ getConfirmationBlocks: () => 0 } as never,
		venues,
	)
	// planFunding is private; reach it for the unit under test.
	const planFunding = (order: Order, outs: { token: HexString; amount: bigint }[]) =>
		(filler as unknown as { planFunding: (o: Order, f: unknown[]) => Promise<boolean> }).planFunding(order, outs)

	return { planFunding, setFundingPrepends, clearFundingPrepends }
}

/** Minimal order with one output leg whose user-minimum is `userMin`. */
function order(userMin: bigint): Order {
	return {
		id: "0xorder",
		destination: "EVM-8453",
		output: { assets: [{ token: USDC_B32, amount: userMin }] },
	} as unknown as Order
}

describe("BasicFiller.planFunding", () => {
	it("does nothing when the wallet already covers the output", async () => {
		const { planFunding, setFundingPrepends, clearFundingPrepends } = makeFiller(1_000_000n, [venue(0n)])
		const outs = [{ token: USDC_B32, amount: 500_000n }]

		const ok = await planFunding(order(490_000n), outs)

		expect(ok).toBe(true)
		expect(outs[0].amount).toBe(500_000n) // unchanged
		expect(setFundingPrepends).not.toHaveBeenCalled()
		expect(clearFundingPrepends).toHaveBeenCalled()
	})

	it("prepends a withdrawal when the venue covers the full shortfall", async () => {
		const { planFunding, setFundingPrepends } = makeFiller(200_000n, [venue(300_000n)])
		const outs = [{ token: USDC_B32, amount: 500_000n }]

		const ok = await planFunding(order(490_000n), outs)

		expect(ok).toBe(true)
		expect(outs[0].amount).toBe(500_000n) // fully funded → output preserved
		expect(setFundingPrepends).toHaveBeenCalledOnce()
	})

	it("reduces the output to the coverable amount on partial funding (≥ user min)", async () => {
		// wallet 200k + venue 250k = 450k sourceable; competitive output 500k; user min 400k.
		const { planFunding, setFundingPrepends } = makeFiller(200_000n, [venue(250_000n)])
		const outs = [{ token: USDC_B32, amount: 500_000n }]

		const ok = await planFunding(order(400_000n), outs)

		expect(ok).toBe(true)
		expect(outs[0].amount).toBe(450_000n) // reduced to sourceable
		expect(setFundingPrepends).toHaveBeenCalledOnce()
	})

	it("skips the order when even the user minimum cannot be sourced", async () => {
		// wallet 200k + venue 100k = 300k < user min 400k.
		const { planFunding, setFundingPrepends, clearFundingPrepends } = makeFiller(200_000n, [venue(100_000n)])
		const outs = [{ token: USDC_B32, amount: 500_000n }]

		const ok = await planFunding(order(400_000n), outs)

		expect(ok).toBe(false)
		expect(setFundingPrepends).not.toHaveBeenCalled()
		expect(clearFundingPrepends).toHaveBeenCalled()
	})
})
