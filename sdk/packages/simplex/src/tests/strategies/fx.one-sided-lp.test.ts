import { FXFiller, type TradingPair } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { AssetRegistry } from "@/config/asset-registry"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"
import { parseUnits } from "viem"

// Pure unit tests for one-sided LP on FXFiller. One-sided LP is expressed per pair by
// omitting a bid/ask price curve: a direction without a curve is disabled, so the filler
// skips orders in that direction. Exercises `canFill` with mocked services so no chain
// access is needed.

const CHAIN = "EVM-97"
const STABLE = "0x1111111111111111111111111111111111111111" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

const FLAT = new FillerPricePolicy({ points: [{ amount: "0", price: "1500" }] })
// Bid for two-sided books — sits above the ask so round trips are profitable.
const FLAT_BID = new FillerPricePolicy({ points: [{ amount: "0", price: "1520" }] })

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
// `getTokenDecimals` and the filler-output cache are what `quotePhantomFill` needs on
// top of `canFill`; both test tokens are 18-decimal.
function makeContractService(): any {
	const cache = new Map<string, unknown>()
	const outputs = new Map<string, unknown>()
	return {
		getTokenDecimals: async () => 18,
		cacheService: {
			getPairClassifications: (id: string) => cache.get(id),
			setPairClassifications: (id: string, pairs: unknown) => cache.set(id, pairs),
			getFillerOutputs: (id: string) => outputs.get(id),
			setFillerOutputs: (id: string, value: unknown) => outputs.set(id, value),
		},
	}
}

function makeFiller(options: {
	bidPricePolicy?: FillerPricePolicy
	askPricePolicy?: FillerPricePolicy
	fundingVenues?: any[]
	side?: "bid" | "ask"
	contractService?: any
	maxOrderSize?: number
}): FXFiller {
	const { contractService: provided, bidPricePolicy, askPricePolicy, maxOrderSize, ...fillerOptions } = options
	const contractService = provided ?? makeContractService()
	const signer = { account: { address: SOLVER } } as any

	const { pairs, registry } = exoticPairs(
		configService,
		{ [CHAIN]: EXOTIC },
		maxOrderSize ?? 5000,
		bidPricePolicy,
		askPricePolicy,
	)
	return new FXFiller(signer, configService, {} as any, contractService, pairs, registry, fillerOptions)
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

/**
 * Mirrors a phantom probe as `pallet-intents-coprocessor` builds it: same chain on both sides,
 * a standard input amount, and an output whose requested amount is zero — the probe asks what
 * the filler would pay for the input, and never executes.
 */
function makePhantomOrder(id: string, input: HexString, output: HexString): Order {
	const order = makeOrder(id, input, output)
	order.output.assets[0].amount = 0n
	return order
}

describe("FXFiller one-sided LP", () => {
	it("fills both directions when both curves are set", async () => {
		const filler = makeFiller({ bidPricePolicy: FLAT_BID, askPricePolicy: FLAT })
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

	// One-sidedness is per pair: two pairs on the same engine can face opposite directions.
	it("gates each pair independently", async () => {
		const OTHER = "0x4444444444444444444444444444444444444444" as HexString
		const registry = new AssetRegistry(configService, {
			CNGN2: { [CHAIN]: EXOTIC },
			ZARP: { [CHAIN]: OTHER },
		})
		const pairs: TradingPair[] = [
			// ask-only: sells CNGN2 for USDC
			{ token0: "USDC", token1: "CNGN2", maxOrderSize: new Decimal(5000), askPricePolicy: FLAT },
			// bid-only: buys ZARP for USDC
			{ token0: "USDC", token1: "ZARP", maxOrderSize: new Decimal(5000), bidPricePolicy: FLAT },
		]
		const signer = { account: { address: SOLVER } } as any
		const filler = new FXFiller(signer, configService, {} as any, makeContractService(), pairs, registry)

		// USDC→CNGN2 allowed (ask side), CNGN2→USDC rejected.
		expect(await filler.canFill(makeOrder("m", STABLE, EXOTIC))).toBe(true)
		expect(await filler.canFill(makeOrder("n", EXOTIC, STABLE))).toBe(false)
		// ZARP→USDC allowed (bid side), USDC→ZARP rejected.
		expect(await filler.canFill(makeOrder("o", OTHER, STABLE))).toBe(true)
		expect(await filler.canFill(makeOrder("p", STABLE, OTHER))).toBe(false)
	})

	// The pallet fixes each probe's direction (token_a in, token_b out), so a one-sided LP only
	// ever sees some probes facing its enabled side — and it has to bid on those. Quoting is the
	// whole of the phantom bid: the filler feeds these outputs straight into the UserOp, with no
	// balance check or gas estimation in the way.
	describe("phantom probes", () => {
		it("ask-only quotes a probe in its enabled (sell exotic) direction", async () => {
			const filler = makeFiller({ askPricePolicy: FLAT })
			const outputs = await filler.quotePhantomFill(makePhantomOrder("ph-a", STABLE, EXOTIC))

			expect(outputs).not.toBeNull()
			expect(outputs).toHaveLength(1)
			expect(outputs?.[0].token).toBe(bytes20ToBytes32(EXOTIC))
			// 100 token0 at a flat ask of 1500 token1/token0 — quoted in full despite the probe
			// requesting zero, which is the point: a zero-output probe must not yield a zero quote.
			expect(outputs?.[0].amount).toBe(parseUnits("150000", 18))
		})

		it("bid-only quotes a probe in its enabled (buy exotic) direction", async () => {
			const filler = makeFiller({ bidPricePolicy: FLAT })
			const outputs = await filler.quotePhantomFill(makePhantomOrder("ph-b", EXOTIC, STABLE))

			expect(outputs).not.toBeNull()
			expect(outputs).toHaveLength(1)
			expect(outputs?.[0].token).toBe(bytes20ToBytes32(STABLE))
			// 100 token1 back through the same flat rate: 100/1500 token0.
			expect(outputs?.[0].amount).toBeGreaterThan(0n)
		})

		it("two-sided quotes probes in both directions", async () => {
			const filler = makeFiller({ bidPricePolicy: FLAT_BID, askPricePolicy: FLAT })
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-c", STABLE, EXOTIC))).not.toBeNull()
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-d", EXOTIC, STABLE))).not.toBeNull()
		})

		// The other half of one-sidedness: a probe facing the disabled side gets no quote at all.
		// Pricing it off the curve we *do* have would publish an offer the filler would not honour,
		// and every accepted quote moves the median the protocol prices intents against.
		it("declines a probe facing its disabled side", async () => {
			const askOnly = makeFiller({ askPricePolicy: FLAT })
			expect(await askOnly.quotePhantomFill(makePhantomOrder("ph-e", EXOTIC, STABLE))).toBeNull()

			const bidOnly = makeFiller({ bidPricePolicy: FLAT })
			expect(await bidOnly.quotePhantomFill(makePhantomOrder("ph-f", STABLE, EXOTIC))).toBeNull()
		})

		// Per pair, not per engine: one pair being unquotable in the probe's direction must not
		// stop another pair's probe from being quoted.
		it("quotes each pair's probe on that pair's own enabled side", async () => {
			const OTHER = "0x4444444444444444444444444444444444444444" as HexString
			const registry = new AssetRegistry(configService, {
				CNGN2: { [CHAIN]: EXOTIC },
				ZARP: { [CHAIN]: OTHER },
			})
			const pairs: TradingPair[] = [
				{ token0: "USDC", token1: "CNGN2", maxOrderSize: new Decimal(5000), askPricePolicy: FLAT },
				{ token0: "USDC", token1: "ZARP", maxOrderSize: new Decimal(5000), bidPricePolicy: FLAT },
			]
			const signer = { account: { address: SOLVER } } as any
			const filler = new FXFiller(signer, configService, {} as any, makeContractService(), pairs, registry)

			// CNGN2 is ask-only: quotes USDC→CNGN2, declines the reverse.
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-g", STABLE, EXOTIC))).not.toBeNull()
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-h", EXOTIC, STABLE))).toBeNull()
			// ZARP is bid-only: the opposite pattern, on the same engine.
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-i", OTHER, STABLE))).not.toBeNull()
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-j", STABLE, OTHER))).toBeNull()
		})

		// A probe is a price quote, not an allocation — it commits no capital, so the pair's
		// per-order exposure cap must not ration its quantity. It used to: the quoted output was
		// clamped to `maxOrderSize` while every consumer still divides by the FULL standard
		// amount, so a pair capped below the probe published a proportionally worse price with
		// nothing to signal it. At maxOrderSize 10 against a 100-token probe that is a rate 10x
		// too low. The cap still governs real fills.
		it("quotes the full probe even when the pair's maxOrderSize is far below it", async () => {
			const uncapped = makeFiller({ askPricePolicy: FLAT, maxOrderSize: 5000 })
			const capped = makeFiller({ askPricePolicy: FLAT, maxOrderSize: 10 })

			const a = await uncapped.quotePhantomFill(makePhantomOrder("ph-cap-a", STABLE, EXOTIC))
			const b = await capped.quotePhantomFill(makePhantomOrder("ph-cap-b", STABLE, EXOTIC))

			expect(a).not.toBeNull()
			expect(b).not.toBeNull()
			// 100 token0 in at a flat 1500 -> 150_000 token1, regardless of the cap.
			expect(a![0].amount).toBe(parseUnits("150000", 18))
			expect(b![0].amount).toBe(a![0].amount)
		})

		it("declines every probe once halted", async () => {
			const filler = makeFiller({ askPricePolicy: FLAT })
			// The halt the clamp counter sets, without driving the clamped fills that set it.
			;(filler as unknown as { halted: boolean }).halted = true
			expect(await filler.quotePhantomFill(makePhantomOrder("ph-k", STABLE, EXOTIC))).toBeNull()
		})
	})

	// Regression: the gate must run even when another strategy already cached the
	// (intrinsic) classification for this order id under the shared cache.
	it("enforces one-sided even when the classification is already cached by another strategy", async () => {
		const shared = makeContractService()
		const twoSided = makeFiller({ bidPricePolicy: FLAT_BID, askPricePolicy: FLAT, contractService: shared })
		const askOnly = makeFiller({ askPricePolicy: FLAT, contractService: shared })

		// Two-sided filler classifies and caches the exotic-in order.
		expect(await twoSided.canFill(makeOrder("shared-1", EXOTIC, STABLE))).toBe(true)
		// Ask-only filler reads the cached classification but must still reject the bid-side order.
		expect(await askOnly.canFill(makeOrder("shared-1", EXOTIC, STABLE))).toBe(false)
	})
})
