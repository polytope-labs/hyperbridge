import { liquidityRemoval } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import { Percent, Token } from "@uniswap/sdk-core"
import { Pool as V4Pool, Position as V4Position, V4PositionManager } from "@uniswap/v4-sdk"
import { decodeAbiParameters, decodeFunctionData, parseAbi } from "viem"
import { describe, it, expect } from "vitest"

// `removeCallParameters` re-derives the liquidity to decrease as
// `liquidityPercentage.multiply(position.liquidity).quotient`, which truncates.
// The planner credits a fill from what it expects the pool to pay out, so the
// figure it prices must be the liquidity the calldata actually carries — never
// the larger one it asked for, which reverts the fill sized against it.

// The position drained by the reverted Base fill, tokenId 2226801.
const POSITION_LIQUIDITY = 3_441_646_880_004n

// A 1e6 denominator loses up to 1/floor(desired * 1e6 / total) of the withdrawal
// — worst when that division lands just under a whole number. This slice is
// ~0.0159% of the position, where the old code shed 0.63%.
const WORST_CASE_SLICE = (POSITION_LIQUIDITY * 158_999n) / 1_000_000_000n

describe("liquidityRemoval", () => {
	it("removes the whole position exactly when asked for all of it", () => {
		const removal = liquidityRemoval(POSITION_LIQUIDITY, POSITION_LIQUIDITY)
		expect(removal?.liquidity).toBe(POSITION_LIQUIDITY)
		expect(removal?.percentage.equalTo(1)).toBe(true)
	})

	it("never encodes more liquidity than asked for", () => {
		for (const desired of [2n, 7n, 1_000n, 543_210_987n, POSITION_LIQUIDITY / 3n, POSITION_LIQUIDITY - 1n]) {
			const removal = liquidityRemoval(POSITION_LIQUIDITY, desired)
			expect(removal).not.toBeNull()
			expect(removal!.liquidity).toBeLessThanOrEqual(desired)
		}
	})

	it("loses at most one unit of liquidity to truncation, even on the worst slice", () => {
		const removal = liquidityRemoval(POSITION_LIQUIDITY, WORST_CASE_SLICE)
		expect(WORST_CASE_SLICE - removal!.liquidity).toBeLessThanOrEqual(1n)
	})

	it("beats the 1e6 denominator it replaced", () => {
		// What the old code encoded: floor(desired * 1e6 / total) / 1e6 * total.
		const coarsePct = (WORST_CASE_SLICE * 1_000_000n) / POSITION_LIQUIDITY
		const coarse = (coarsePct * POSITION_LIQUIDITY) / 1_000_000n
		const removal = liquidityRemoval(POSITION_LIQUIDITY, WORST_CASE_SLICE)

		expect(coarse).toBeLessThan(removal!.liquidity)
		// The old shortfall is a real fraction of the withdrawal, not a rounding unit:
		// on a $14.8k draw that is roughly $93 of credit the pool never pays out.
		expect(Number(WORST_CASE_SLICE - coarse) / Number(WORST_CASE_SLICE)).toBeGreaterThan(0.006)
	})

	it("caps a request for more than the position holds", () => {
		const removal = liquidityRemoval(POSITION_LIQUIDITY, POSITION_LIQUIDITY * 2n)
		expect(removal?.liquidity).toBe(POSITION_LIQUIDITY)
	})

	it("returns null rather than encoding a zero decrease", () => {
		// The SDK's ZERO_LIQUIDITY invariant throws on a zero partial position, so a
		// slice too thin to register must not reach it. Note the percentage alone is
		// not the test: for this position a single unit of liquidity yields a
		// non-zero percentage whose product with the total still floors to nothing.
		expect(liquidityRemoval(POSITION_LIQUIDITY, 0n)).toBeNull()
		expect(liquidityRemoval(0n, 100n)).toBeNull()
		expect(liquidityRemoval(10n ** 30n, 1n)).toBeNull()
		expect(liquidityRemoval(POSITION_LIQUIDITY, 1n)).toBeNull()
		// One unit up, it survives — the boundary is real, not a blanket rejection.
		expect(liquidityRemoval(POSITION_LIQUIDITY, 2n)?.liquidity).toBe(1n)
	})
})

// `liquidityRemoval` reproduces the SDK's own truncation so the planner can
// credit a fill from the liquidity the calldata will actually carry. That is a
// contract with `removeCallParameters`, not an implementation detail: if a
// v4-sdk upgrade changes how it derives the decrease, the credit silently goes
// back to overstating and fills revert. So assert it against the real encoder.
describe("liquidityRemoval matches what removeCallParameters encodes", () => {
	const TOKEN0 = new Token(8453, "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f", 18)
	const TOKEN1 = new Token(8453, "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913", 6)

	/** Runs the real SDK encoder and pulls the liquidity out of DECREASE_LIQUIDITY. */
	function encodedLiquidity(totalLiquidity: bigint, percentage: Percent): bigint {
		const pool = new V4Pool(
			TOKEN0,
			TOKEN1,
			3000,
			60,
			"0x0000000000000000000000000000000000000000",
			"79228162514264337593543950336",
			totalLiquidity.toString(),
			0,
		)
		const position = new V4Position({
			pool,
			tickLower: -600,
			tickUpper: 600,
			liquidity: totalLiquidity.toString(),
		})
		const { calldata } = V4PositionManager.removeCallParameters(position, {
			slippageTolerance: new Percent(50, 10_000),
			deadline: "1900000000",
			hookData: "0x",
			tokenId: "2226801",
			liquidityPercentage: percentage,
			burnToken: false,
		})

		const { args } = decodeFunctionData({
			abi: parseAbi(["function modifyLiquidities(bytes unlockData, uint256 deadline)"]),
			data: calldata as `0x${string}`,
		})
		const [, params] = decodeAbiParameters(
			[{ type: "bytes" }, { type: "bytes[]" }],
			args[0] as `0x${string}`,
		)
		// DECREASE_LIQUIDITY params: (tokenId, liquidity, amount0Min, amount1Min, hookData)
		const [, liquidity] = decodeAbiParameters(
			[{ type: "uint256" }, { type: "uint256" }, { type: "uint128" }, { type: "uint128" }, { type: "bytes" }],
			(params as readonly `0x${string}`[])[0],
		)
		return liquidity as bigint
	}

	it("encodes exactly the liquidity the planner credited", () => {
		for (const desired of [
			POSITION_LIQUIDITY,
			POSITION_LIQUIDITY - 1n,
			POSITION_LIQUIDITY / 2n,
			POSITION_LIQUIDITY / 3n,
			WORST_CASE_SLICE,
			1_000_000n,
			2n,
		]) {
			const removal = liquidityRemoval(POSITION_LIQUIDITY, desired)
			expect(removal, `no removal for ${desired}`).not.toBeNull()
			expect(encodedLiquidity(POSITION_LIQUIDITY, removal!.percentage)).toBe(removal!.liquidity)
		}
	})

	it("never hands the SDK a percentage it will reject", () => {
		// Every non-null result must encode without tripping ZERO_LIQUIDITY.
		for (let desired = 1n; desired <= 40n; desired++) {
			const removal = liquidityRemoval(POSITION_LIQUIDITY, desired)
			if (!removal) continue
			expect(removal.liquidity).toBeGreaterThan(0n)
			expect(encodedLiquidity(POSITION_LIQUIDITY, removal.percentage)).toBe(removal.liquidity)
		}
	})

	it("still encodes the whole position when the fill drains it", () => {
		const removal = liquidityRemoval(POSITION_LIQUIDITY, POSITION_LIQUIDITY)
		expect(encodedLiquidity(POSITION_LIQUIDITY, removal!.percentage)).toBe(POSITION_LIQUIDITY)
	})
})
