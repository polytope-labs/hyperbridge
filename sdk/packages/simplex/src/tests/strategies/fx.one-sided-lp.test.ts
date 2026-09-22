import { FXFiller, type TradingPair } from "@/strategies/fx"
import { AssetRegistry } from "@/config/asset-registry"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { parseUnits } from "viem"
import type { LimitOrderSide } from "@/data/types"
import { limitOrderStore } from "../helpers/limit-orders"

// Pure unit tests for one-sided LP on FXFiller. One-sided LP is expressed per pair by
// omitting a bid/ask price curve: a direction without a curve is disabled, so the filler
// skips orders in that direction. Exercises `canFill` with mocked services so no chain
// access is needed.

const CHAIN = "EVM-97"
const STABLE = "0x1111111111111111111111111111111111111111" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

/** Builds an exotic-pair set + registry for tests: `token1` addresses traded against USDC and USDT. */
function exoticPairs(
	resolver: any,
	token1: Record<string, HexString>,
): { pairs: TradingPair[]; registry: AssetRegistry } {
	const registry = new AssetRegistry(resolver, { EXOTIC: token1 })
	const pairs: TradingPair[] = ["USDC", "USDT"].map((token0) => ({ token0, token1: "EXOTIC" }))
	return { pairs, registry }
}


const configService = {
	getUsdcAsset: () => STABLE,
	getUsdtAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
	getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
	getCNgnAsset: () => undefined,
	getMaxOverfillBps: () => 500n,
	getMaxConsecutiveClamps: () => 3,
} as any

// Mirrors how simplex.ts shares one contractService (and its classification cache)
// across every strategy. Pass the same instance to two fillers to exercise that.
// Both test tokens are 18-decimal.
function makeContractService(): any {
	const cache = new Map<string, unknown>()
	const outputs = new Map<string, unknown>()
	const bidPlans = new Map<string, unknown>()
	return {
		getTokenDecimals: async () => 18,
		cacheService: {
			getPairClassifications: (id: string) => cache.get(id),
			setPairClassifications: (id: string, pairs: unknown) => cache.set(id, pairs),
			getFillerOutputs: (id: string) => outputs.get(id),
			setFillerOutputs: (id: string, value: unknown) => outputs.set(id, value),
			setMatchedLimitOrder: () => {},
			setBidPlans: (id: string, plans: unknown) => bidPlans.set(id, plans),
			getBidPlans: (id: string) => bidPlans.get(id) ?? [],
			clearBidPlans: (id: string) => bidPlans.delete(id),
		},
	}
}

/**
 * A filler whose operator has posted `sides` of the USDC/EXOTIC book.
 *
 * One-sidedness is which sides are open: a bid takes the base in and pays the
 * quote out, an ask the other way round, so posting only one leaves the reverse
 * direction unserved.
 */
async function makeFiller(options: {
	sides: LimitOrderSide[]
	fundingVenues?: any[]
	contractService?: any
}): Promise<FXFiller> {
	const { contractService: provided, sides, ...fillerOptions } = options
	const contractService = provided ?? makeContractService()
	const signer = { address: SOLVER } as any

	const { pairs, registry } = exoticPairs(configService, { [CHAIN]: EXOTIC })
	return new FXFiller(signer, configService, {} as any, contractService, pairs, registry, {
		...fillerOptions,
		limitOrders: await limitOrderStore(
			sides.map((side) => ({ base: "USDC", quote: "EXOTIC", side, fillChain: CHAIN, price: "1", size: "100000" })),
		),
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
	it("fills both directions when the operator has posted both sides", async () => {
		const filler = await makeFiller({ sides: ["BID", "ASK"] })
		// stable in, exotic out
		expect(await filler.canFill(makeOrder("a", STABLE, EXOTIC))).toBe(true)
		// exotic in, stable out
		expect(await filler.canFill(makeOrder("b", EXOTIC, STABLE))).toBe(true)
	})

	it("a bid alone takes the stable in and pays the exotic out, and refuses the reverse", async () => {
		const filler = await makeFiller({ sides: ["BID"] })
		expect(await filler.canFill(makeOrder("c", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("d", EXOTIC, STABLE))).toBe(false)
	})

	it("an ask alone takes the exotic in and pays the stable out, and refuses the reverse", async () => {
		const filler = await makeFiller({ sides: ["ASK"] })
		expect(await filler.canFill(makeOrder("e", EXOTIC, STABLE))).toBe(true)
		expect(await filler.canFill(makeOrder("f", STABLE, EXOTIC))).toBe(false)
	})

	// One-sidedness is per pair: two pairs on the same engine can face opposite directions.
	it("gates each book independently", async () => {
		const OTHER = "0x4444444444444444444444444444444444444444" as HexString
		const registry = new AssetRegistry(configService, {
			CNGN2: { [CHAIN]: EXOTIC },
			ZARP: { [CHAIN]: OTHER },
		})
		const pairs: TradingPair[] = [
			{ token0: "USDC", token1: "CNGN2" },
			{ token0: "USDC", token1: "ZARP" },
		]
		const signer = { address: SOLVER } as any
		const filler = new FXFiller(signer, configService, {} as any, makeContractService(), pairs, registry, {
			limitOrders: await limitOrderStore([
				// Pays CNGN2 out for USDC, and pays USDC out for ZARP.
				{ base: "USDC", quote: "CNGN2", side: "BID", fillChain: CHAIN, price: "1", size: "100000" },
				{ base: "USDC", quote: "ZARP", side: "ASK", fillChain: CHAIN, price: "1", size: "100000" },
			]),
		})

		expect(await filler.canFill(makeOrder("m", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("n", EXOTIC, STABLE))).toBe(false)
		expect(await filler.canFill(makeOrder("o", OTHER, STABLE))).toBe(true)
		expect(await filler.canFill(makeOrder("p", STABLE, OTHER))).toBe(false)
	})
})
