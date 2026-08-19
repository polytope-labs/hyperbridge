// Pool attribution for phantom legs, exercised against the real generated token registry rather
// than a fixture — the registry's decimals are the denominator of every published rate, so what
// matters is that the shipped data resolves, not that a stand-in does.

;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

import { resolvePoolLeg } from "@/services/liquidityPool.service"

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
