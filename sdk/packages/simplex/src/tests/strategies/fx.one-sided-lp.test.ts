import { FXFiller } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { parseUnits } from "viem"

// Pure unit tests for one-sided LP on FXFiller. One-sided LP is expressed by omitting
// a bid/ask price curve: a side without a curve is disabled, so the filler skips orders
// in that direction. Exercises `canFill` with mocked services so no chain access is needed.

const CHAIN = "EVM-97"
const STABLE = "0x1111111111111111111111111111111111111111" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

const FLAT = new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] })

function makeFiller(options: {
	bidPricePolicy?: FillerPricePolicy
	askPricePolicy?: FillerPricePolicy
	fundingVenues?: any[]
	side?: "bid" | "ask"
}): FXFiller {
	const configService = {
		getUsdcAsset: () => STABLE,
		getUsdtAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any

	const cache = new Map<string, unknown>()
	const contractService = {
		cacheService: {
			getPairClassifications: (id: string) => cache.get(id),
			setPairClassifications: (id: string, pairs: unknown) => cache.set(id, pairs),
		},
	} as any

	const signer = { account: { address: SOLVER } } as any
	const clientManager = {} as any

	return new FXFiller(signer, configService, clientManager, contractService, 5000, { [CHAIN]: EXOTIC }, options)
}

function makeOrder(id: string, input: HexString, output: HexString): Order {
	const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(input), amount: parseUnits("100", 18) }]
	const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(output), amount: parseUnits("100", 18) }]
	return {
		id,
		user: bytes20ToBytes32(SOLVER),
		source: CHAIN,
		destination: CHAIN,
		deadline: 0n,
		nonce: 0n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs,
		output: { beneficiary: bytes20ToBytes32(SOLVER), assets: outputs, call: "0x" as HexString },
	} as unknown as Order
}

describe("FXFiller one-sided LP", () => {
	it("fills both directions when both curves are set", async () => {
		const filler = makeFiller({ bidPricePolicy: FLAT, askPricePolicy: FLAT })
		// stable in, exotic out
		expect(await filler.canFill(makeOrder("a", STABLE, EXOTIC))).toBe(true)
		// exotic in, stable out
		expect(await filler.canFill(makeOrder("b", EXOTIC, STABLE))).toBe(true)
	})

	it("ask-only accepts stable-in/exotic-out (sell exotic) and rejects the reverse", async () => {
		const filler = makeFiller({ askPricePolicy: FLAT })
		expect(await filler.canFill(makeOrder("c", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("d", EXOTIC, STABLE))).toBe(false)
	})

	it("bid-only accepts exotic-in/stable-out (buy exotic) and rejects the reverse", async () => {
		const filler = makeFiller({ bidPricePolicy: FLAT })
		expect(await filler.canFill(makeOrder("e", EXOTIC, STABLE))).toBe(true)
		expect(await filler.canFill(makeOrder("f", STABLE, EXOTIC))).toBe(false)
	})

	// Pool pricing (no curves): one-sided LP via the venue `side` switch.
	const VENUE = [{ name: "UniswapV4" }]

	it("venue side=ask sells exotic only and rejects the reverse", async () => {
		const filler = makeFiller({ fundingVenues: VENUE, side: "ask" })
		expect(await filler.canFill(makeOrder("g", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("h", EXOTIC, STABLE))).toBe(false)
	})

	it("venue side=bid buys exotic only and rejects the reverse", async () => {
		const filler = makeFiller({ fundingVenues: VENUE, side: "bid" })
		expect(await filler.canFill(makeOrder("i", EXOTIC, STABLE))).toBe(true)
		expect(await filler.canFill(makeOrder("j", STABLE, EXOTIC))).toBe(false)
	})

	it("venue with no side fills both directions", async () => {
		const filler = makeFiller({ fundingVenues: VENUE })
		expect(await filler.canFill(makeOrder("k", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("l", EXOTIC, STABLE))).toBe(true)
	})

	it("rejects 'side' combined with static curves", () => {
		expect(() => makeFiller({ fundingVenues: VENUE, side: "ask", askPricePolicy: FLAT })).toThrow()
	})
})
