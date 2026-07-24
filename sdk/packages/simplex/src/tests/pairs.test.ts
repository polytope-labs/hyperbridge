import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"
import { parseUnits } from "viem"
import { bytes20ToBytes32, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { AssetRegistry, KNOWN_ASSETS, validateAssetDefinitions, type BuiltinAssetResolver } from "@/config/asset-registry"
import { validatePairConfigs } from "@/config/pairs"
import { FXFiller, type TradingPair } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"

// Pure unit tests for the asset registry, [[pairs]] validation, and the
// pairs-driven FX engine's leg math (via quotePhantomFill, mocked services).

const CHAIN = "EVM-56"
const OTHER_CHAIN = "EVM-8453"
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const USDT = "0x4444444444444444444444444444444444444444" as HexString
const CNGN = "0x2222222222222222222222222222222222222222" as HexString
const ZARP = "0x5555555555555555555555555555555555555555" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString

const resolver: BuiltinAssetResolver = {
	getUsdcAsset: (chain: string) => {
		if (chain === OTHER_CHAIN) throw new Error("not configured")
		return USDC
	},
	getUsdtAsset: () => USDT,
	getDaiAsset: () => {
		throw new Error("not configured")
	},
	getCNgnAsset: () => undefined,
}

describe("AssetRegistry", () => {
	it("resolves built-in symbols per chain, case-insensitively", () => {
		const registry = new AssetRegistry(resolver)
		expect(registry.getAddress("USDC", CHAIN)).toBe(USDC)
		expect(registry.getAddress("usdc", CHAIN)).toBe(USDC)
		expect(registry.getAddress("USDT", CHAIN)).toBe(USDT)
		// Resolver throws / returns undefined → absent, not an error.
		expect(registry.getAddress("USDC", OTHER_CHAIN)).toBeNull()
		expect(registry.getAddress("DAI", CHAIN)).toBeNull()
		expect(registry.getAddress("CNGN", CHAIN)).toBeNull()
	})

	it("user [assets] entries extend and override built-ins per chain", () => {
		const registry = new AssetRegistry(resolver, {
			CNGN: { [CHAIN]: CNGN },
			USDC: { [OTHER_CHAIN]: ZARP },
		})
		expect(registry.getAddress("CNGN", CHAIN)).toBe(CNGN)
		expect(registry.getAddress("cNGN", CHAIN)).toBe(CNGN)
		expect(registry.getAddress("CNGN", OTHER_CHAIN)).toBeNull()
		// User address fills the chain the built-in resolver can't serve…
		expect(registry.getAddress("USDC", OTHER_CHAIN)).toBe(ZARP)
		// …while other chains still resolve from the built-in registry.
		expect(registry.getAddress("USDC", CHAIN)).toBe(USDC)
	})

	it("ships only curated addresses that pass the address validator", () => {
		// Guards KNOWN_ASSETS itself against transcription slips (bad EIP-55
		// checksums, zero addresses) — the validator otherwise only sees [assets].
		expect(() => validateAssetDefinitions(KNOWN_ASSETS)).not.toThrow()
	})

	it("treats SDK sentinel values ('0x', zero address) as absent", () => {
		// The SDK chain registry never throws for unknown chains/assets — it
		// returns "0x" or stores a literal zero address. Neither may leak out:
		// the zero address doubles as the native-token sentinel in the fill path.
		const sentinelResolver: BuiltinAssetResolver = {
			getUsdcAsset: () => USDC,
			getUsdtAsset: () => "0x" as HexString,
			getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
			getCNgnAsset: () => undefined,
		}
		const registry = new AssetRegistry(sentinelResolver)
		expect(registry.getAddress("USDC", CHAIN)).toBe(USDC)
		expect(registry.getAddress("USDT", CHAIN)).toBeNull()
		expect(registry.getAddress("DAI", CHAIN)).toBeNull()
		// A user [assets] entry may not smuggle the zero address in either.
		expect(() =>
			validateAssetDefinitions({
				FOO: { [CHAIN]: "0x0000000000000000000000000000000000000000" as HexString },
			}),
		).toThrow(/invalid address/)
	})

	it("ships curated assets with zero user configuration", () => {
		const registry = new AssetRegistry(resolver)
		// Addresses verified on-chain before inclusion in KNOWN_ASSETS.
		expect(registry.getAddress("ZARP", "EVM-137")).toBe("0xb755506531786C8aC63B756BaB1ac387bACB0C04")
		expect(registry.getAddress("zarp", "EVM-1")).toBe("0xb755506531786C8aC63B756BaB1ac387bACB0C04")
		expect(registry.getAddress("EURC", "EVM-8453")).toBe("0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1adb42")
		expect(registry.getAddress("XSGD", "EVM-137")).toBe("0xDC3326e71D45186F113a2F448984CA0e8D201995")
		// Not deployed there → absent, not an error.
		expect(registry.getAddress("EURC", "EVM-56")).toBeNull()
	})

	it("rejects malformed definitions", () => {
		expect(() => validateAssetDefinitions({ FOO: { [CHAIN]: "0xnope" as HexString } })).toThrow(/invalid address/)
		expect(() => validateAssetDefinitions({ FOO: {} })).toThrow(/at least one chain/)
		// Case-insensitive duplicate.
		expect(() =>
			validateAssetDefinitions({
				foo: { [CHAIN]: ZARP },
				FOO: { [CHAIN]: ZARP },
			}),
		).toThrow(/twice/)
	})
})

describe("validatePairConfigs", () => {
	const assets = { CNGN: { [CHAIN]: CNGN } }
	const CURVE = [{ amount: "0", price: "1500" }]
	const SIZE = "5000"

	it("accepts arbitrary well-formed pairs", () => {
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, bidPriceCurve: CURVE, askPriceCurve: CURVE },
					{ token0: "USDT", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "ZARP", token1: "CNGN", maxOrderSize: "90000", bidPriceCurve: CURVE },
				],
				assets,
			),
		).not.toThrow()
	})

	it("accepts registry-shipped symbols with no [assets] config at all", () => {
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
				{ token0: "ZARP", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
				{ token0: "EURC", token1: "XSGD", maxOrderSize: SIZE, bidPriceCurve: CURVE },
			]),
		).not.toThrow()
	})

	it("rejects unknown symbols and duplicates", () => {
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "WAT", maxOrderSize: SIZE, askPriceCurve: CURVE }], assets),
		).toThrow(/unknown symbol/)
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "usdc", token1: "cngn", maxOrderSize: SIZE, bidPriceCurve: CURVE },
				],
				assets,
			),
		).toThrow(/declared twice/)
		// The reverse orientation is the same market — accepting both would make
		// leg matching declaration-order dependent (and price legs in the wrong unit).
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "CNGN", token1: "USDC", maxOrderSize: SIZE, askPriceCurve: CURVE },
				],
				assets,
			),
		).toThrow(/already declared/)
	})

	it("accepts same-token pairs that are ask-only with prices at or below par", () => {
		const PAR_CURVE = [
			{ amount: "100", price: "0.99" },
			{ amount: "100000", price: "0.999" },
		]
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "usdc", maxOrderSize: SIZE, askPriceCurve: PAR_CURVE }]),
		).not.toThrow()
		// Bid curve is meaningless — both directions are the same market.
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "USDC", maxOrderSize: SIZE, askPriceCurve: PAR_CURVE, bidPriceCurve: PAR_CURVE },
			]),
		).toThrow(/ask-only/)
		// Above par pays out more than received.
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "USDC", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "1.01" }] },
			]),
		).toThrow(/more than received/)
		// Venue pricing cannot substitute for the curve on same-token pairs.
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "USDC", maxOrderSize: SIZE }], undefined, true),
		).toThrow(/askPriceCurve/)
	})

	it("rejects non-USD-stable same-token pairs (spread would be valued in the wrong unit)", () => {
		// CNGN/CNGN is a known pair of symbols but the realized spread is credited
		// into a USD-denominated gate — only USD-stable same-token markets are sound.
		expect(() =>
			validatePairConfigs(
				[{ token0: "CNGN", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "0.99" }] }],
				assets,
			),
		).toThrow(/USD-stable/)
		// USDC/USDC and USDT/USDT are fine.
		expect(() =>
			validatePairConfigs([{ token0: "USDT", token1: "USDT", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "0.999" }] }]),
		).not.toThrow()
	})

	it("requires a positive maxOrderSize", () => {
		expect(() => validatePairConfigs([{ token0: "USDC", token1: "CNGN", askPriceCurve: CURVE } as any])).toThrow(
			/maxOrderSize' is required/,
		)
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "CNGN", maxOrderSize: "0", askPriceCurve: CURVE }]),
		).toThrow(/positive/)
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "CNGN", maxOrderSize: "abc", askPriceCurve: CURVE }]),
		).toThrow(/decimal string/)
	})

	it("requires a curve unless venue pricing is available", () => {
		expect(() => validatePairConfigs([{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE }], assets)).toThrow(
			/price curve/,
		)
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE }], assets, true),
		).not.toThrow()
	})
})

// ---------------------------------------------------------------------------
// FX engine leg math with pair-local rates and caps, via quotePhantomFill
// ---------------------------------------------------------------------------

function makeContractService(): any {
	const cache = new Map<string, unknown>()
	const decimals: Record<string, number> = {
		[USDC.toLowerCase()]: 6,
		[USDT.toLowerCase()]: 6,
		[CNGN.toLowerCase()]: 18,
		[ZARP.toLowerCase()]: 18,
	}
	return {
		cacheService: {
			getPairClassifications: (id: string) => cache.get(`pc:${id}`),
			setPairClassifications: (id: string, pairs: unknown) => cache.set(`pc:${id}`, pairs),
			setFillerOutputs: (id: string, outputs: unknown) => cache.set(`fo:${id}`, outputs),
		},
		getTokenDecimals: async (token: string) => decimals[token.toLowerCase()] ?? 18,
	}
}

const flat = (price: string) => new FillerPricePolicy({ points: [{ amount: "0", price }] })

function makeFiller(pairs: TradingPair[]) {
	const configService = {
		...resolver,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any
	const registry = new AssetRegistry(configService, {
		CNGN: { [CHAIN]: CNGN },
		ZARP: { [CHAIN]: ZARP },
	})
	const signer = { account: { address: SOLVER } } as any
	return new FXFiller(signer, configService, {} as any, makeContractService(), pairs, registry)
}

function makeOrder(id: string, input: TokenInfo, output: TokenInfo): Order {
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
		inputs: [input],
		output: { beneficiary: bytes20ToBytes32(SOLVER), assets: [output], call: "0x" as HexString },
	} as unknown as Order
}

const size = (n: string) => new Decimal(n)

describe("FXFiller pairs engine", () => {
	it("prices the ask leg at the pair's rate (token1 per token0)", async () => {
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") },
		])
		const order = makeOrder(
			"ask",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		const outputs = await filler.quotePhantomFill(order)
		// 1000 USDC × 1500 CNGN/USDC = 1,500,000 CNGN
		expect(outputs?.[0].amount).toBe(parseUnits("1500000", 18))
	})

	it("prices the bid leg at the pair's rate", async () => {
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), bidPricePolicy: flat("1500") },
		])
		const order = makeOrder(
			"bid",
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1500", 18) },
			{ token: bytes20ToBytes32(USDC), amount: 0n },
		)
		const outputs = await filler.quotePhantomFill(order)
		// 1500 CNGN ÷ 1500 CNGN/USDC = 1 USDC
		expect(outputs?.[0].amount).toBe(parseUnits("1", 6))
	})

	it("routes each leg through its own pair's curve", async () => {
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") },
			{ token0: "USDT", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1000") },
		])
		const usdcLeg = makeOrder(
			"multi-1",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		const usdtLeg = makeOrder(
			"multi-2",
			{ token: bytes20ToBytes32(USDT), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		expect((await filler.quotePhantomFill(usdcLeg))?.[0].amount).toBe(parseUnits("150000", 18))
		expect((await filler.quotePhantomFill(usdtLeg))?.[0].amount).toBe(parseUnits("100000", 18))
	})

	it("caps each pair at its own maxOrderSize, in token0 units", async () => {
		// ZARP-quoted pair: everything is denominated in ZARP — no USD anywhere.
		const filler = makeFiller([
			{ token0: "ZARP", token1: "CNGN", maxOrderSize: size("100000"), askPricePolicy: flat("100") },
		])

		const small = makeOrder(
			"zarp-small",
			{ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		// 1000 ZARP (under the cap) × 100 CNGN/ZARP = 100,000 CNGN
		expect((await filler.quotePhantomFill(small))?.[0].amount).toBe(parseUnits("100000", 18))
		// Order sizing is the token0 notional — fed to confirmation curves as-is.
		const sized = await filler.getOrderUsdValue(small)
		expect(sized?.inputUsd.eq(new Decimal(1000))).toBe(true)

		const large = makeOrder(
			"zarp-large",
			{ token: bytes20ToBytes32(ZARP), amount: parseUnits("200000", 18) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		// 200,000 ZARP → capped at maxOrderSize 100,000 ZARP → 10,000,000 CNGN
		expect((await filler.quotePhantomFill(large))?.[0].amount).toBe(parseUnits("10000000", 18))
	})

	it("sizes bid legs through the pair's own curve", async () => {
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), bidPricePolicy: flat("1500") },
		])
		const order = makeOrder(
			"bid-sizing",
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("3000000", 18) },
			{ token: bytes20ToBytes32(USDC), amount: 0n },
		)
		// 3,000,000 CNGN ÷ 1500 = 2000 USDC notional (under the 5000 cap)
		const sized = await filler.getOrderUsdValue(order)
		expect(sized?.inputUsd.eq(new Decimal(2000))).toBe(true)
		expect((await filler.quotePhantomFill(order))?.[0].amount).toBe(parseUnits("2000", 6))

		// 15,000,000 CNGN ÷ 1500 = 10,000 USDC notional → capped at 5000 USDC out.
		const large = makeOrder(
			"bid-sizing-large",
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("15000000", 18) },
			{ token: bytes20ToBytes32(USDC), amount: 0n },
		)
		expect((await filler.quotePhantomFill(large))?.[0].amount).toBe(parseUnits("5000", 6))
	})

	it("quotes same-token pairs at the below-par ask, across differing decimals", async () => {
		// USDC is 6-decimal on CHAIN and 18-decimal on CHAIN2 (à la BSC).
		const CHAIN2 = "EVM-97"
		const USDC18 = "0x6666666666666666666666666666666666666666" as HexString
		const configService = {
			getUsdcAsset: (chain: string) => (chain === CHAIN2 ? USDC18 : USDC),
			getUsdtAsset: () => USDT,
			getDaiAsset: () => {
				throw new Error("not configured")
			},
			getCNgnAsset: () => undefined,
			getMaxOverfillBps: () => 500n,
			getMaxConsecutiveClamps: () => 3,
		} as any
		const contractService = makeContractService()
		// Extend the decimals mock for the 18-decimal deployment.
		const inner = contractService.getTokenDecimals
		contractService.getTokenDecimals = async (token: string) =>
			token.toLowerCase() === USDC18.toLowerCase() ? 18 : inner(token)

		const pairs: TradingPair[] = [
			// 50 bps spread, capped at 10,000 USDC per order.
			{ token0: "USDC", token1: "USDC", maxOrderSize: size("10000"), askPricePolicy: flat("0.995") },
		]
		const registry = new AssetRegistry(configService)
		const signer = { account: { address: SOLVER } } as any
		const filler = new FXFiller(signer, configService, {} as any, contractService, pairs, registry)

		const order = {
			...makeOrder(
				"same-token",
				{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
				{ token: bytes20ToBytes32(USDC18), amount: 0n },
			),
			destination: CHAIN2,
		} as unknown as Order
		// 1000 USDC in → 995 USDC out (0.995), scaled to the destination's 18 decimals.
		expect((await filler.quotePhantomFill(order))?.[0].amount).toBe(parseUnits("995", 18))

		const large = {
			...makeOrder(
				"same-token-capped",
				{ token: bytes20ToBytes32(USDC), amount: parseUnits("20000", 6) },
				{ token: bytes20ToBytes32(USDC18), amount: 0n },
			),
			destination: CHAIN2,
		} as unknown as Order
		// 20,000 USDC → capped at maxOrderSize 10,000 → 9,950 out.
		expect((await filler.quotePhantomFill(large))?.[0].amount).toBe(parseUnits("9950", 18))
	})

	it("prefers explicitly configured curves over venue pricing", async () => {
		// The pair has curves AND a venue is available — the curve must win; the
		// pool only prices pairs with no curves at all.
		const venue = {
			name: "UniswapV4",
			initialise: async () => {},
			getExoticTokenPrice: async () => new Decimal("0.001"), // pool says 1000 CNGN/USD
			walletReserveForToken: () => 0n,
			planWithdrawalForToken: async () => ({ calls: [], credited: 0n }),
		} as any
		const configService = {
			...resolver,
			getMaxOverfillBps: () => 500n,
			getMaxConsecutiveClamps: () => 3,
		} as any
		const registry = new AssetRegistry(configService, { CNGN: { [CHAIN]: CNGN } })
		const signer = { account: { address: SOLVER } } as any
		const filler = new FXFiller(
			signer,
			configService,
			{} as any,
			makeContractService(),
			[{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") }],
			registry,
			{ fundingVenues: [venue] },
		)
		const order = makeOrder(
			"curve-beats-venue",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		// Curve rate 1500, venue rate would be 1000 — expect the curve's output.
		expect((await filler.quotePhantomFill(order))?.[0].amount).toBe(parseUnits("150000", 18))
	})

	it("rejects duplicate and reverse-duplicate engine pairs", () => {
		const registry = new AssetRegistry(resolver as any, { CNGN: { [CHAIN]: CNGN } })
		const signer = { account: { address: SOLVER } } as any
		const build = (pairs: TradingPair[]) =>
			new FXFiller(
				signer,
				{ getMaxOverfillBps: () => 500n, getMaxConsecutiveClamps: () => 3 } as any,
				{} as any,
				makeContractService(),
				pairs,
				registry,
			)
		expect(() =>
			build([
				{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") },
				{ token0: "CNGN", token1: "USDC", maxOrderSize: size("5000"), askPricePolicy: flat("0.0006") },
			]),
		).toThrow(/duplicate market/)
	})

	it("rejects same-token engine pairs with a bid policy or above-par prices", () => {
		const registry = new AssetRegistry(resolver as any)
		const signer = { account: { address: SOLVER } } as any
		const build = (pair: TradingPair) =>
			new FXFiller(signer, { getMaxOverfillBps: () => 500n, getMaxConsecutiveClamps: () => 3 } as any, {} as any, makeContractService(), [pair], registry)

		expect(() =>
			build({ token0: "USDC", token1: "USDC", maxOrderSize: size("5000"), askPricePolicy: flat("0.995"), bidPricePolicy: flat("0.995") }),
		).toThrow(/ask-only/)
		expect(() =>
			build({ token0: "USDC", token1: "USDC", maxOrderSize: size("5000"), askPricePolicy: flat("1.5") }),
		).toThrow(/must not exceed 1/)
	})

	it("rejects orders whose legs match no configured pair", async () => {
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") },
		])
		const order = makeOrder(
			"no-pair",
			{ token: bytes20ToBytes32(USDT), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		expect(await filler.canFill(order)).toBe(false)
		expect(await filler.quotePhantomFill(order)).toBeNull()
	})
})

describe("FXFiller same-token markets (cross-chain only, USD-stable only)", () => {
	const CHAIN_A = "EVM-1"
	const CHAIN_B = "EVM-8453"
	// USDC resolves to the same address on both chains (USDC-style deployment).
	const cfg = {
		getUsdcAsset: () => USDC,
		getUsdtAsset: () => USDT,
		getDaiAsset: () => {
			throw new Error("not configured")
		},
		getCNgnAsset: () => undefined,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any
	const signer = { account: { address: SOLVER } } as any
	const usdcUsdc = (): TradingPair[] => [
		{ token0: "USDC", token1: "USDC", maxOrderSize: size("100000"), askPricePolicy: flat("0.999") },
	]

	function sameTokenOrder(source: string, destination: string): Order {
		return {
			id: "st",
			user: bytes20ToBytes32(SOLVER),
			source,
			destination,
			deadline: 0n,
			nonce: 0n,
			fees: 0n,
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs: [{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) }],
			output: {
				beneficiary: bytes20ToBytes32(SOLVER),
				assets: [{ token: bytes20ToBytes32(USDC), amount: parseUnits("999", 6) }],
				call: "0x" as HexString,
			},
		} as unknown as Order
	}

	it("rejects a same-chain same-token order (a self-swap paying more than received)", async () => {
		const filler = new FXFiller(signer, cfg, {} as any, makeContractService(), usdcUsdc(), new AssetRegistry(cfg))
		expect(await filler.canFill(sameTokenOrder(CHAIN_A, CHAIN_A))).toBe(false)
	})

	it("accepts a cross-chain same-token order (the same-asset transfer market)", async () => {
		const filler = new FXFiller(signer, cfg, {} as any, makeContractService(), usdcUsdc(), new AssetRegistry(cfg))
		expect(await filler.canFill(sameTokenOrder(CHAIN_A, CHAIN_B))).toBe(true)
	})

	it("rejects a non-USD-stable same-token pair at construction", () => {
		const registry = new AssetRegistry(cfg, { CNGN: { [CHAIN_A]: CNGN, [CHAIN_B]: CNGN } })
		expect(
			() =>
				new FXFiller(
					signer,
					cfg,
					{} as any,
					makeContractService(),
					[{ token0: "CNGN", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("0.99") }],
					registry,
				),
		).toThrow(/USD-stable/)
	})
})
