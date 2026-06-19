import { FXFiller } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { type HexString } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"

// Pure unit tests for the Uniswap price guard. Exercises the private
// `checkPriceGuard` directly so no chain access or venue mocking is needed.

const CHAIN = "EVM-8453"
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const REFERENCE = "1575" // exotic per USD

function makeFiller(priceGuard?: { maxDeviationBps: number; referencePrices: Record<string, string> }): FXFiller {
	const configService = {
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any
	const contractService = {} as any
	const signer = { account: { address: "0x3333333333333333333333333333333333333333" } } as any
	const clientManager = {} as any

	return new FXFiller(signer, configService, clientManager, contractService, 5000, { [CHAIN]: EXOTIC }, {
		bidPricePolicy: new FillerPricePolicy({ points: [{ amount: "0", price: REFERENCE }] }),
		askPricePolicy: new FillerPricePolicy({ points: [{ amount: "0", price: REFERENCE }] }),
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
		const filler = makeFiller({ maxDeviationBps: 200, referencePrices: { [CHAIN]: REFERENCE } })
		// 1% above and below — within the 2% band
		expect(check(filler, "1590")).toBe(true)
		expect(check(filler, "1560")).toBe(true)
		// exactly at the 2% edge (1575 * 1.02 = 1606.5)
		expect(check(filler, "1606.5")).toBe(true)
	})

	it("rejects a quote above the band", () => {
		const filler = makeFiller({ maxDeviationBps: 200, referencePrices: { [CHAIN]: REFERENCE } })
		expect(check(filler, "1700")).toBe(false) // ~7.9% above
	})

	it("rejects a quote below the band", () => {
		const filler = makeFiller({ maxDeviationBps: 200, referencePrices: { [CHAIN]: REFERENCE } })
		expect(check(filler, "1400")).toBe(false) // ~11% below
	})

	it("passes when no reference exists for the chain", () => {
		const filler = makeFiller({ maxDeviationBps: 200, referencePrices: { "EVM-137": REFERENCE } })
		expect(check(filler, "5000")).toBe(true)
	})
})
