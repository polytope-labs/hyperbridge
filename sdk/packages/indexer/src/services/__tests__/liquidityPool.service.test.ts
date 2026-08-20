// Pool attribution for phantom legs, exercised against the real generated token registry rather
// than a fixture — the registry's decimals are the denominator of every published rate, so what
// matters is that the shipped data resolves, not that a stand-in does.

;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

import { poolRateFromQuote, resolvePoolLeg } from "@/services/liquidityPool.service"

const BSC = "EVM-56"
const CNGN_BSC = "0xa8aea66b361a8d53e8865c62d142167af28af058"
const USDT_BSC = "0x55d398326f99059ff775485246999027b3197955"

const leg = (tokenA: string, tokenB: string, standardAmount: bigint) => ({
	legIndex: 0,
	tokenA,
	tokenB,
	standardAmount,
})

describe("resolvePoolLeg on BNB Chain", () => {
	// cNGN sits at 6 decimals beside 18-decimal Binance-pegged stables, so both directions of the
	// pair carry different standard amounts and normalize by different scales. Getting either
	// backwards is silent, so pin both.
	it("resolves cNGN -> USDT as the pool's SELL side, scaled by USDT's 18 decimals", () => {
		expect(resolvePoolLeg(BSC, leg(CNGN_BSC, USDT_BSC, 10n ** 6n))).toEqual({
			poolId: "cNGN-USDT",
			direction: "SELL",
			token0Symbol: "cNGN",
			token1Symbol: "USDT",
			outDecimals: 18,
			inDecimals: 6,
		})
	})

	it("resolves USDT -> cNGN as the same pool's BUY side, scaled by cNGN's 6 decimals", () => {
		expect(resolvePoolLeg(BSC, leg(USDT_BSC, CNGN_BSC, 10n ** 18n))).toEqual({
			poolId: "cNGN-USDT",
			direction: "BUY",
			token0Symbol: "cNGN",
			token1Symbol: "USDT",
			outDecimals: 6,
			inDecimals: 18,
		})
	})

	// The misconfiguration this chain invites: copying 1e18 from the stables next to cNGN in the
	// pallet config. The standard amount is the rate's denominator, so accepting it would publish
	// every cNGN price on BSC off by 1e12. Refusing attribution is the safe failure.
	it("refuses a cNGN leg whose standard amount was copied from its 18-decimal neighbours", () => {
		expect(resolvePoolLeg(BSC, leg(CNGN_BSC, USDT_BSC, 10n ** 18n))).toBeNull()
	})

	// Raising the probe size is how the pallet buys quote precision: one whole cNGN priced into
	// 18-decimal USDT is fine, but into a 6-decimal stable it affords ~3 digits. Attribution is
	// size-agnostic — the rate is renormalized from `inDecimals` and the leg's own standard
	// amount — so a bigger probe resolves exactly like a one-unit one.
	it("accepts a 1000-token probe", () => {
		expect(resolvePoolLeg(BSC, leg(CNGN_BSC, USDT_BSC, 1000n * 10n ** 6n))).toEqual({
			poolId: "cNGN-USDT",
			direction: "SELL",
			token0Symbol: "cNGN",
			token1Symbol: "USDT",
			outDecimals: 18,
			inDecimals: 6,
		})
	})

	// Nothing requires the probe to be a whole number of tokens; the renormalization is exact
	// arithmetic on the raw value, so an odd size is priced correctly rather than refused.
	it("accepts a standard amount that is not a whole number of input tokens", () => {
		expect(resolvePoolLeg(BSC, leg(CNGN_BSC, USDT_BSC, 10n ** 6n + 1n))).not.toBeNull()
	})

	it("refuses a zero standard amount", () => {
		expect(resolvePoolLeg(BSC, leg(CNGN_BSC, USDT_BSC, 0n))).toBeNull()
	})

	// The other half of the tripwire: a probe far BELOW one unit is the same decimals bug seen
	// from the other side (18-decimal amount read against a 6-decimal registry entry).
	it("refuses a standard amount far below one input token", () => {
		expect(resolvePoolLeg(BSC, leg(USDT_BSC, CNGN_BSC, 10n ** 6n))).toBeNull()
	})
})

// The rate a pool publishes is `output units per ONE whole input token`, but a leg is quoted
// against whatever standard amount the pallet configured. These pin that renormalization against
// values read off mainnet on 2026-08-19, so a probe-size change cannot move a published rate.
describe("poolRateFromQuote", () => {
	const cases = [
		{ name: "Base cNGN(6) -> USDC(6)", medianPrice: 715n, inDecimals: 6, outDecimals: 6, expected: 715000000000000n },
		{
			name: "Base USDC(6) -> cNGN(6)",
			medianPrice: 1393000000n,
			inDecimals: 6,
			outDecimals: 6,
			expected: 1393000000000000000000n,
		},
		{
			name: "BSC cNGN(6) -> USDT(18)",
			medianPrice: 716845878136200n,
			inDecimals: 6,
			outDecimals: 18,
			expected: 716845878136200n,
		},
		{
			name: "BSC USDT(18) -> cNGN(6)",
			medianPrice: 1393000000n,
			inDecimals: 18,
			outDecimals: 6,
			expected: 1393000000000000000000n,
		},
	]

	// The legacy probe, and the one still deployed. At exactly one whole unit the two powers in
	// the renormalization cancel, so this must reproduce what the indexer published before the
	// standard amount became a variable at all.
	it.each(cases)("reproduces the live rate at a 1-unit probe: $name", ({ medianPrice, inDecimals, outDecimals, expected }) => {
		const standardAmount = 10n ** BigInt(inDecimals)
		expect(poolRateFromQuote(medianPrice, { inDecimals, outDecimals }, standardAmount)).toBe(expected)
	})

	// A bigger probe draws a proportionally bigger quote, so the rate must not move.
	it.each(cases)("yields the identical rate at a 1000-unit probe: $name", ({ medianPrice, inDecimals, outDecimals, expected }) => {
		const standardAmount = 1000n * 10n ** BigInt(inDecimals)
		expect(poolRateFromQuote(medianPrice * 1000n, { inDecimals, outDecimals }, standardAmount)).toBe(expected)
	})

	// The precision the bump is for: a 6-decimal output only affords ~3 digits against one whole
	// token, so the same curve resolves ~1000x finer at a 1000-token probe.
	it("carries the extra digits a larger probe buys", () => {
		const oneUnit = poolRateFromQuote(715n, { inDecimals: 6, outDecimals: 6 }, 10n ** 6n)
		const thousand = poolRateFromQuote(715307n, { inDecimals: 6, outDecimals: 6 }, 1000n * 10n ** 6n)
		expect(oneUnit).toBe(715000000000000n)
		expect(thousand).toBe(715307000000000n)
	})

	it("handles a probe that is not a whole number of input tokens", () => {
		// 1.5 whole tokens quoted at the same rate as the 1-unit Base case.
		expect(poolRateFromQuote(1072n, { inDecimals: 6, outDecimals: 6 }, 1_500_000n)).toBe(714666666666666n)
	})
})
