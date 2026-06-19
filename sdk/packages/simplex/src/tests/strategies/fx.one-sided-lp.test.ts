import { FXFiller, AccumulationSide } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { parseUnits } from "viem"

// Pure unit tests for the one-sided LP (`accumulate`) constraint on FXFiller.
// Exercises `canFill` directly with mocked services so no chain access is needed.

const CHAIN = "EVM-97"
const STABLE = "0x1111111111111111111111111111111111111111" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

function makeFiller(accumulate?: AccumulationSide): FXFiller {
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

	return new FXFiller(signer, configService, clientManager, contractService, 5000, { [CHAIN]: EXOTIC }, {
		bidPricePolicy: new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] }),
		askPricePolicy: new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] }),
		accumulate,
	})
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
	it("fills both directions when accumulate is unset", async () => {
		const filler = makeFiller()
		// stable in, exotic out
		expect(await filler.canFill(makeOrder("a", STABLE, EXOTIC))).toBe(true)
		// exotic in, stable out
		expect(await filler.canFill(makeOrder("b", EXOTIC, STABLE))).toBe(true)
	})

	it("accumulate=stable accepts stable-in/exotic-out and rejects the reverse", async () => {
		const filler = makeFiller(AccumulationSide.Stable)
		expect(await filler.canFill(makeOrder("c", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("d", EXOTIC, STABLE))).toBe(false)
	})

	it("accumulate=exotic accepts exotic-in/stable-out and rejects the reverse", async () => {
		const filler = makeFiller(AccumulationSide.Exotic)
		expect(await filler.canFill(makeOrder("e", EXOTIC, STABLE))).toBe(true)
		expect(await filler.canFill(makeOrder("f", STABLE, EXOTIC))).toBe(false)
	})
})
