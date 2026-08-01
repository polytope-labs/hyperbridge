import { FXFiller, type TradingPair } from "@/strategies/fx"
import { AssetRegistry } from "@/config/asset-registry"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { type HexString } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"

// Pure unit tests for the Uniswap price guard. Exercises the private
// `checkPriceGuard` directly so no chain access or venue mocking is needed.

const CHAIN = "EVM-8453"
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const REFERENCE = "1575" // exotic per USD

/** Builds an exotic-pair set + registry for tests: `token1` addresses traded against USDC and USDT. */
function exoticPairs(
	resolver: any,
	token1: Record<string, HexString>,
	maxOrderSize: number,
	bidPricePolicy?: FillerPricePolicy,
	askPricePolicy?: FillerPricePolicy,
): { pairs: TradingPair[]; registry: AssetRegistry } {
	const registry = new AssetRegistry(resolver, { EXOTIC: token1 })
	const pairs: TradingPair[] = ["USDC", "USDT"].map((token0) => ({
		token0,
		token1: "EXOTIC",
		maxOrderSize: new Decimal(maxOrderSize),
		bidPricePolicy,
		askPricePolicy,
	}))
	return { pairs, registry }
}


function makeFiller(priceGuard?: Record<string, { referencePrice: string; maxDeviationBps: number }>): FXFiller {
	const configService = {
		getUsdcAsset: () => "0x1111111111111111111111111111111111111111" as HexString,
		getUsdtAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
		getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
		getCNgnAsset: () => undefined,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any
	const contractService = {} as any
	const signer = { account: { address: "0x3333333333333333333333333333333333333333" } } as any
	const clientManager = {} as any

	const { pairs, registry } = exoticPairs(
		configService,
		{ [CHAIN]: EXOTIC },
		5000,
		// Bid above ask — an uncrossed book with a healthy round-trip spread.
		new FillerPricePolicy({ points: [{ amount: "0", price: "1590" }] }),
		new FillerPricePolicy({ points: [{ amount: "0", price: REFERENCE }] }),
	)
	return new FXFiller(signer, configService, clientManager, contractService, pairs, registry, {
		priceGuard,
	})
}

function check(filler: FXFiller, exoticPerUsd: string): boolean {
	return (filler as any).checkPriceGuard("order-1", CHAIN, new Decimal(exoticPerUsd))
}

describe("FXFiller Uniswap price guard", () => {
	it("passes every quote when no guard is configured", () => {
		const filler = makeFiller()
		expect(check(filler, "1575")).toBe(true)
		expect(check(filler, "5000")).toBe(true)
	})

	it("passes a quote inside the band", () => {
		const filler = makeFiller({ [CHAIN]: { referencePrice: REFERENCE, maxDeviationBps: 200 } })
		// 1% above and below — within the 2% band
		expect(check(filler, "1590")).toBe(true)
		expect(check(filler, "1560")).toBe(true)
		// exactly at the 2% edge (1575 * 1.02 = 1606.5)
		expect(check(filler, "1606.5")).toBe(true)
	})

	it("rejects a quote above the band", () => {
		const filler = makeFiller({ [CHAIN]: { referencePrice: REFERENCE, maxDeviationBps: 200 } })
		expect(check(filler, "1700")).toBe(false) // ~7.9% above
	})

	it("rejects a quote below the band", () => {
		const filler = makeFiller({ [CHAIN]: { referencePrice: REFERENCE, maxDeviationBps: 200 } })
		expect(check(filler, "1400")).toBe(false) // ~11% below
	})

	it("passes when no reference exists for the chain", () => {
		const filler = makeFiller({ "EVM-137": { referencePrice: REFERENCE, maxDeviationBps: 200 } })
		expect(check(filler, "5000")).toBe(true)
	})
})

describe("FXFiller price guard on confirmation sizing (referenceRate)", () => {
	// Venue-priced pair (no curves): referenceRate consults the pool quote,
	// which sizes the order's USD notional for confirmation depth. The guard
	// must run here exactly as on the trade-pricing path — a manipulated pool
	// understating the value would otherwise shrink the reorg protection.
	function makeVenueFiller(priceGuard?: Record<string, { referencePrice: string; maxDeviationBps: number }>) {
		const configService = {
			getUsdcAsset: () => "0x1111111111111111111111111111111111111111" as HexString,
			getUsdtAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
			getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
			getCNgnAsset: () => undefined,
			getMaxOverfillBps: () => 500n,
			getMaxConsecutiveClamps: () => 3,
		} as any
		const signer = { account: { address: "0x3333333333333333333333333333333333333333" } } as any
		const { pairs, registry } = exoticPairs(configService, { [CHAIN]: EXOTIC }, 5000)
		const filler = new FXFiller(signer, configService, {} as any, {} as any, pairs, registry, {
			fundingVenues: [{} as any],
			priceGuard,
		})
		return { filler, pair: pairs[0] }
	}

	const referenceRate = (filler: FXFiller, pair: TradingPair, exoticPerUsd: string) =>
		(filler as any).referenceRate(
			{ pair, inputIsToken0: true, token1Chain: CHAIN, token1Address: EXOTIC },
			async () => new Decimal(1).div(new Decimal(exoticPerUsd)), // venue quote: USD per token1
		)

	it("sizes with the pool rate when the quote is inside the band", async () => {
		const { filler, pair } = makeVenueFiller({ [CHAIN]: { referencePrice: REFERENCE, maxDeviationBps: 200 } })
		const rate = await referenceRate(filler, pair, "1580")
		expect(rate?.toFixed(0)).toBe("1580")
	})

	it("refuses to size when the pool quote breaches the guard band", async () => {
		const { filler, pair } = makeVenueFiller({ [CHAIN]: { referencePrice: REFERENCE, maxDeviationBps: 200 } })
		// ~11% below reference — a pool understating the exotic's value.
		expect(await referenceRate(filler, pair, "1400")).toBeNull()
		// ~8% above — overstating it (would inflate the notional instead).
		expect(await referenceRate(filler, pair, "1700")).toBeNull()
	})

	it("sizes unguarded when no reference is configured (guard is optional)", async () => {
		const { filler, pair } = makeVenueFiller()
		const rate = await referenceRate(filler, pair, "1400")
		expect(rate?.toFixed(0)).toBe("1400")
	})
})
