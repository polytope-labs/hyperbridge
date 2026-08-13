import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"
import { parseUnits } from "viem"
import { bytes20ToBytes32, ChainConfigService, type HexString, type Order, type TokenInfo } from "@hyperbridge/sdk"
import { AssetRegistry, validateAssetDefinitions, type BuiltinAssetResolver } from "@/config/asset-registry"
import {
	assertPairSymbolsResolve,
	pickAnchorStable,
	validatePairConfigs,
	type PairAddressResolver,
} from "@/config/pairs"
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
	getAssetBySymbol: () => undefined,
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

	it("treats SDK sentinel values ('0x', zero address) as absent", () => {
		// The SDK chain registry never throws for unknown chains/assets — it
		// returns "0x" or stores a literal zero address. Neither may leak out:
		// the zero address doubles as the native-token sentinel in the fill path.
		const sentinelResolver: BuiltinAssetResolver = {
			getUsdcAsset: () => USDC,
			getUsdtAsset: () => "0x" as HexString,
			getDaiAsset: () => "0x0000000000000000000000000000000000000000" as HexString,
			getCNgnAsset: () => undefined,
			getAssetBySymbol: () => undefined,
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

	it("ships curated assets with zero user configuration, from the SDK chain registry", () => {
		// One source of truth: the registry resolves through the SDK's chain.ts
		// asset table, so these pins guard the shipped data itself. Addresses
		// verified on-chain (symbol()/decimals()) before inclusion there.
		const sdk = new ChainConfigService({})
		const sdkResolver: BuiltinAssetResolver = {
			getUsdcAsset: (chain) => sdk.getUsdcAsset(chain),
			getUsdtAsset: (chain) => sdk.getUsdtAsset(chain),
			getDaiAsset: (chain) => sdk.getDaiAsset(chain),
			getCNgnAsset: (chain) => sdk.getCNgnAsset(chain),
			getAssetBySymbol: (chain, symbol) => sdk.getAssetBySymbol(chain, symbol),
		}
		const registry = new AssetRegistry(sdkResolver)
		expect(registry.getAddress("ZARP", "EVM-137")).toBe("0xb755506531786C8aC63B756BaB1ac387bACB0C04")
		expect(registry.getAddress("zarp", "EVM-1")).toBe("0xb755506531786C8aC63B756BaB1ac387bACB0C04")
		expect(registry.getAddress("EURC", "EVM-8453")).toBe("0x60a3E35Cc302bFA44Cb288Bc5a4F316Fdb1adb42")
		expect(registry.getAddress("XSGD", "EVM-137")).toBe("0xDC3326e71D45186F113a2F448984CA0e8D201995")
		expect(registry.getAddress("TRYB", "EVM-1")).toBe("0x2C537E5624e4af88A7ae4060C022609376C8D0EB")
		expect(registry.getAddress("USDR", "EVM-1")).toBe("0x9623DfB044D5612Ce0c0F1606973CCAEFd03CD05")
		expect(registry.getAddress("USDR", "EVM-8453")).toBe("0x3B5F2810fB2168FfA9C73160F97BF9f2461fFa5c")
		expect(registry.getAddress("usdr", "EVM-137")).toBe("0x3B5F2810fB2168FfA9C73160F97BF9f2461fFa5c")
		expect(registry.getAddress("CNGN", "EVM-56")).toBe("0xa8AEA66B361a8d53e8865c62D142167Af28Af058")
		// cNGN on BNB Chain is 6-decimal while the Binance-pegged stables beside it are 18. Every
		// rate divides by this, so pin it: inheriting 18 from its neighbours would silently scale
		// every cNGN quote on that chain by 1e12.
		expect(sdk.getCNgnDecimals("EVM-56")).toBe(6)
		expect(sdk.getUsdcDecimals("EVM-56")).toBe(18)
		// Not deployed there → absent, not an error.
		expect(registry.getAddress("EURC", "EVM-56")).toBeNull()
		expect(registry.getAddress("USDR", "EVM-42161")).toBeNull()
	})

	it("rejects malformed definitions", () => {
		expect(() => validateAssetDefinitions({ FOO: { [CHAIN]: "0xnope" as HexString } })).toThrow(/invalid address/)
		expect(() => validateAssetDefinitions({ FOO: {} })).toThrow(/at least one chain/)
		// A typo'd chain key would otherwise silently mean "not deployed there".
		expect(() => validateAssetDefinitions({ FOO: { EVM137: ZARP } })).toThrow(/chain key/)
		expect(() => validateAssetDefinitions({ FOO: { "evm-137": ZARP } })).toThrow(/chain key/)
		// Case-insensitive duplicate.
		expect(() =>
			validateAssetDefinitions({
				foo: { [CHAIN]: ZARP },
				FOO: { [CHAIN]: ZARP },
			}),
		).toThrow(/twice/)
	})

	it("addAssets registers new symbols live and invalidates the negative address cache", () => {
		const registry = new AssetRegistry(resolver)
		// The null result below is cached — addAssets must invalidate it.
		expect(registry.getAddress("BRZ", CHAIN)).toBeNull()
		registry.addAssets({ BRZ: { [CHAIN]: ZARP } })
		expect(registry.getAddress("BRZ", CHAIN)).toBe(ZARP)
		expect(registry.getAddress("brz", CHAIN)).toBe(ZARP)

		expect(() => registry.addAssets({ USDC: { [CHAIN]: CNGN } })).toThrow(/already defined/)
		expect(() => registry.addAssets({ brz: { [CHAIN]: CNGN } })).toThrow(/already defined/)
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
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, bidPriceCurve: [{ amount: "0", price: "1580" }], askPriceCurve: [{ amount: "0", price: "1550" }] },
					{ token0: "USDT", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "ZARP", token1: "CNGN", maxOrderSize: "90000", bidPriceCurve: CURVE },
				],
				assets,
			),
		).not.toThrow()
	})

	it("accepts crossed and zero-spread two-sided books — sides are quoted independently", () => {
		// Each side fills at its own curve; crossing only makes a full round
		// trip lose money. The wizards warn instead of rejecting.
		expect(() =>
			validatePairConfigs(
				[
					{
						token0: "USDC",
						token1: "CNGN",
						maxOrderSize: SIZE,
						bidPriceCurve: [{ amount: "0", price: "1540" }],
						askPriceCurve: [{ amount: "0", price: "1550" }],
					},
				],
				assets,
			),
		).not.toThrow()
		expect(() =>
			validatePairConfigs(
				[{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, bidPriceCurve: CURVE, askPriceCurve: CURVE }],
				assets,
			),
		).not.toThrow()
	})

	it("accepts registry-shipped symbols with no [assets] config at all", () => {
		// ZARP anchors through CNGN (via USDC/CNGN), EURC directly via USDC/EURC.
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
				{ token0: "ZARP", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
				{ token0: "USDC", token1: "EURC", maxOrderSize: SIZE, askPriceCurve: CURVE },
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

	it("accepts same-token pairs that are ask-only with prices below par", () => {
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
		// At or above par the spread is zero or negative — never fillable.
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "USDC", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "1.01" }] },
			]),
		).toThrow(/strictly below 1/)
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "USDC", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "1" }] },
			]),
		).toThrow(/strictly below 1/)
		// Venue pricing cannot substitute for the curve on same-token pairs.
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "USDC", maxOrderSize: SIZE }], undefined, true),
		).toThrow(/askPriceCurve/)
	})

	it("accepts non-USD same-token pairs (e.g. CNGN/CNGN) when a USD pair anchors the asset", () => {
		// The USDC/CNGN curve is the price feed that lets confirmation depth
		// size CNGN notionals in USD; without it the config must not start.
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "CNGN", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "0.99" }] },
				],
				assets,
			),
		).not.toThrow()
		// The ask-only and below-par rules still apply to any same-token pair.
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "CNGN", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "1.01" }] },
				],
				assets,
			),
		).toThrow(/strictly below 1/)
	})

	it("rejects any pair whose token0 has no USD anchor — direct or transitive", () => {
		// A lone non-USD market has no path to a $1 anchor.
		expect(() =>
			validatePairConfigs([{ token0: "ZARP", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE }], assets),
		).toThrow(/no USD anchor for ZARP/)
		// Same-token non-USD alone is equally unanchored.
		expect(() =>
			validatePairConfigs(
				[{ token0: "CNGN", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "0.99" }] }],
				assets,
			),
		).toThrow(/no USD anchor for CNGN/)
		// USDC/CNGN anchors CNGN, whose curve then anchors ZARP: two hops.
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
					{ token0: "ZARP", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: CURVE },
				],
				assets,
			),
		).not.toThrow()
	})

	it("referenceOnly pairs anchor without opening a market and need no maxOrderSize", () => {
		// A lone CNGN/CNGN transfer market anchored by a reference USDC/CNGN quote.
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN", referenceOnly: true, askPriceCurve: [{ amount: "0", price: "1565" }] },
					{ token0: "CNGN", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "0.995" }] },
				],
				assets,
			),
		).not.toThrow()
		// A reference pair without a curve has nothing to reference.
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "CNGN", referenceOnly: true }], assets, true),
		).toThrow(/the curve IS the reference/)
		// Same-token pairs carry no FX rate to reference.
		expect(() =>
			validatePairConfigs(
				[
					{
						token0: "USDC",
						token1: "USDC",
						referenceOnly: true,
						askPriceCurve: [{ amount: "0", price: "0.99" }],
					},
				],
				assets,
			),
		).toThrow(/carries no FX rate/)
	})

	it("rejects non-positive curve prices (they would poison the USD anchor math)", () => {
		expect(() =>
			validatePairConfigs(
				[{ token0: "USDC", token1: "CNGN", maxOrderSize: SIZE, askPriceCurve: [{ amount: "0", price: "0" }] }],
				assets,
			),
		).toThrow(/must be positive/)
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
		// ZARP-quoted pair: pricing and the cap are denominated in ZARP. The
		// USDC/ZARP pair only anchors ZARP for confirmation sizing.
		const filler = makeFiller([
			{ token0: "USDC", token1: "ZARP", maxOrderSize: size("100000"), askPricePolicy: flat("18") },
			{ token0: "ZARP", token1: "CNGN", maxOrderSize: size("100000"), askPricePolicy: flat("100") },
		])

		const small = makeOrder(
			"zarp-small",
			{ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		// 1000 ZARP (under the cap) × 100 CNGN/ZARP = 100,000 CNGN
		expect((await filler.quotePhantomFill(small))?.[0].amount).toBe(parseUnits("100000", 18))
		// Confirmation sizing converts the token0 notional to USD via the
		// anchor factor: 1000 ZARP ÷ 18 ZARP-per-USDC ≈ $55.56, not "$1000".
		const sized = await filler.getOrderUsdValue(small)
		expect(sized?.inputUsd.toFixed(4)).toBe("55.5556")

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
		).toThrow(/strictly below 1/)
		// Par is a zero spread — the pair could never fill anything.
		expect(() =>
			build({ token0: "USDC", token1: "USDC", maxOrderSize: size("5000"), askPricePolicy: flat("1") }),
		).toThrow(/strictly below 1/)
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

	it("accepts same-chain cross-asset orders — only same-token pairs are chain-restricted", async () => {
		// makeOrder is source == destination: an on-chain USDC→CNGN swap. The
		// same-chain rejection applies to same-token self-swaps, never to FX.
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") },
		])
		const order = makeOrder(
			"same-chain-fx",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		expect(await filler.canFill(order)).toBe(true)
	})
})

describe("FXFiller same-token markets (cross-chain only)", () => {
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

})

describe("FXFiller profit gates (fees cover execution; spread independently positive)", () => {
	const SRC = "EVM-1"
	const DST = "EVM-8453"
	const cfg = {
		getUsdcAsset: () => USDC,
		getUsdtAsset: () => USDT,
		getDaiAsset: () => {
			throw new Error("nc")
		},
		getCNgnAsset: () => undefined,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any
	const signer = { account: { address: SOLVER } } as any

	const decimalsByAddr: Record<string, number> = {
		[USDC.toLowerCase()]: 6,
		[USDT.toLowerCase()]: 6,
		[CNGN.toLowerCase()]: 18,
		[ZARP.toLowerCase()]: 18,
	}

	// contractService + clientManager mocked just enough to drive calculateProfitability.
	function gateFiller(
		pairs: TradingPair[],
		registry: AssetRegistry,
		estimate: { fillGas: bigint; relayer: bigint },
		options?: { fundingVenues?: unknown[] },
	) {
		const cache = new Map<string, unknown>()
		const contractService = {
			cacheService: {
				getPairClassifications: (id: string) => cache.get(`pc:${id}`),
				setPairClassifications: (id: string, v: unknown) => cache.set(`pc:${id}`, v),
				setFillerOutputs: () => {},
				setFundingPrepends: () => {},
				clearFundingPrepends: () => {},
			},
			getFeeTokenWithDecimals: async () => ({ decimals: 6, address: USDC }),
			getTokenDecimals: async (token: string) => decimalsByAddr[token.toLowerCase()] ?? 18,
			estimateGasFillPost: async () => ({
				totalCostInSourceFeeToken: estimate.fillGas,
				relayerFeeInSourceFeeToken: estimate.relayer,
				dispatchFee: 0n,
				callGasLimit: 0n,
			}),
		} as any
		const destClient = {
			chain: { blockTime: 2000 },
			getBlock: async () => ({ number: 100n, timestamp: 0n }),
			getBalance: async () => 10n ** 30n,
			readContract: async () => 10n ** 30n, // balanceOf: effectively unlimited
		}
		const clientManager = { getPublicClient: () => destClient } as any
		return new FXFiller(signer, cfg, clientManager, contractService, pairs, registry, {
			fundingVenues: (options?.fundingVenues ?? []) as any,
		})
	}

	function order(id: string, input: TokenInfo, output: TokenInfo, fees: bigint): Order {
		return {
			id,
			user: bytes20ToBytes32(SOLVER),
			source: SRC,
			destination: DST,
			deadline: 1_000_000n,
			nonce: 0n,
			fees,
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs: [input],
			output: { beneficiary: bytes20ToBytes32(SOLVER), assets: [output], call: "0x" as HexString },
		} as unknown as Order
	}

	const usdcOnBoth = () => new AssetRegistry(cfg, { CNGN: { [SRC]: CNGN, [DST]: CNGN } })

	it("gate 1: rejects when fees do not cover fill gas + relayer fee", async () => {
		const filler = gateFiller(
			[{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000"), bidPricePolicy: flat("1500"), askPricePolicy: flat("1450") }],
			usdcOnBoth(),
			{ fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, // exec cost = $5
		)
		const o = order(
			"g1-fail",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1400000", 18) },
			parseUnits("4", 6), // fees $4 < $5 exec cost
		)
		expect(await filler.calculateProfitability(o)).toBe(0)
	})

	it("gate 1: passes when fees cover execution and the FX spread is positive", async () => {
		const filler = gateFiller(
			[{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000"), bidPricePolicy: flat("1500"), askPricePolicy: flat("1450") }],
			usdcOnBoth(),
			{ fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) },
		)
		const o = order(
			"g1-pass",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1400000", 18) },
			parseUnits("10", 6), // fees $10 > $5 exec cost; 1.4M CNGN within the 1450 ask curve
		)
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
	})

	it("gate 2: rejects a same-token order whose realized spread is zero", async () => {
		// The book is healthy (ask 0.999) but the ORDER demands a 1:1 payout —
		// the realized spread on this fill is zero, so the per-leg gate refuses.
		const filler = gateFiller(
			[{ token0: "USDC", token1: "USDC", maxOrderSize: size("100000"), askPricePolicy: flat("0.999") }],
			new AssetRegistry(cfg),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"g2-fail",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			parseUnits("100", 6), // fees easily cover exec, but spread on the fill = 0
		)
		expect(await filler.calculateProfitability(o)).toBe(0)
	})

	it("rounds the credited input DOWN — decimal dust cannot flip break-even to profitable", async () => {
		// CNGN is 18-dec on the source and 6-dec on the destination. The input
		// exceeds the requested output by one atto-unit: ceiling that credit to
		// 6 decimals would fabricate "+1 unit" of spread and pass the gate.
		const CNGN6 = "0x7777777777777777777777777777777777777777" as HexString
		decimalsByAddr[CNGN6.toLowerCase()] = 6
		const registry = new AssetRegistry(cfg, {
			CNGN: { [SRC]: CNGN, [DST]: CNGN6 },
		})
		const filler = gateFiller(
			[
				{ token0: "USDC", token1: "CNGN", maxOrderSize: size("5000"), askPricePolicy: flat("1500") },
				{ token0: "CNGN", token1: "CNGN", maxOrderSize: size("5000000"), askPricePolicy: flat("0.999") },
			],
			registry,
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"floor-dust",
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1000", 18) + 1n }, // 1000.000000000000000001
			{ token: bytes20ToBytes32(CNGN6), amount: parseUnits("1000", 6) }, // demands full par payout
			parseUnits("100", 6),
		)
		expect(await filler.calculateProfitability(o)).toBe(0)
	})

	it("gate 2: passes a same-token order with a below-par spread and covering fees", async () => {
		const filler = gateFiller(
			[{ token0: "USDC", token1: "USDC", maxOrderSize: size("100000"), askPricePolicy: flat("0.999") }],
			new AssetRegistry(cfg),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"g2-pass",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("999", 6) },
			parseUnits("100", 6),
		)
		// 1000 in, 999 out (ask 0.999) → $1 spread; fees cover exec → profitable.
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
	})

	// Two arbitrary NON-USD tokens: ZARP/CNGN. token0=ZARP, curves in CNGN-per-ZARP.
	// Pricing and sizing work in the pair's own units; the USDC/ZARP anchor
	// pair exists so confirmation depth can price ZARP in USD.
	const zarpCngnRegistry = () =>
		new AssetRegistry(cfg, { ZARP: { [SRC]: ZARP, [DST]: ZARP }, CNGN: { [SRC]: CNGN, [DST]: CNGN } })
	const usdcZarpAnchor = (): TradingPair => ({
		token0: "USDC",
		token1: "ZARP",
		maxOrderSize: size("100000"),
		bidPricePolicy: flat("18.2"),
		askPricePolicy: flat("17.8"), // mid 18 ZARP per USDC → ZARP ≈ $1/18
	})

	it("fills inside a crossed book when the order is within its own side's curve", async () => {
		// bid 1392 / ask 1400 CNGN per USDC (crossed). The user wants 1396.3 CNGN
		// for 1 USDC — within the 1400 the ask curve pays, so the filler bids.
		// The negative round-trip FX margin is report-only and never gates.
		const filler = gateFiller(
			[{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000"), bidPricePolicy: flat("1392"), askPricePolicy: flat("1400") }],
			usdcOnBoth(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"crossed-fill",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1396.3015", 18) },
			parseUnits("10", 6),
		)
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
	})

	it("rejects construction when a pair's token0 has no USD anchor", () => {
		expect(() =>
			gateFiller(
				[{ token0: "ZARP", token1: "CNGN", maxOrderSize: size("1000000"), askPricePolicy: flat("95") }],
				zarpCngnRegistry(),
				{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
			),
		).toThrow(/no USD anchor for ZARP/)
	})

	it("rejects an order when a leg demands more than its own ask curve yields", async () => {
		// The USDC/CNGN leg demands 1.6M CNGN but the 1450 ask pays at most
		// 1.45M for 1000 USDC. Each leg is checked against its own curve only —
		// the comfortably-within-curve ZARP/CNGN leg cannot rescue the order.
		const filler = gateFiller(
			[
				{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000"), bidPricePolicy: flat("1500"), askPricePolicy: flat("1450") },
				{ token0: "ZARP", token1: "CNGN", maxOrderSize: size("1000000"), bidPricePolicy: flat("100"), askPricePolicy: flat("95") },
			],
			zarpCngnRegistry(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"beyond-curve",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1600000", 18) }, // > 1000 × 1450
			parseUnits("100", 6),
		)
		o.inputs.push({ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) })
		o.output.assets.push({ token: bytes20ToBytes32(CNGN), amount: parseUnits("40000", 18) }) // ≤ 1000 × 95
		expect(await filler.calculateProfitability(o)).toBe(0)
	})

	it("fills a multi-pair order when every leg is within its own curve", async () => {
		const filler = gateFiller(
			[
				{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000"), bidPricePolicy: flat("1500"), askPricePolicy: flat("1450") },
				{ token0: "ZARP", token1: "CNGN", maxOrderSize: size("1000000"), bidPricePolicy: flat("100"), askPricePolicy: flat("95") },
			],
			zarpCngnRegistry(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"all-legs-win",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1400000", 18) }, // ≤ 1000 × 1450
			parseUnits("100", 6),
		)
		o.inputs.push({ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) })
		o.output.assets.push({ token: bytes20ToBytes32(CNGN), amount: parseUnits("94000", 18) }) // ≤ 1000 × 95
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
	})

	it("venue-priced legs fill at the pool mid — gated by fees and the price guard only", async () => {
		// A pool mid is one price, not a book. Like curve legs, venue legs are
		// gated by execution cost (gate 1) and the price guard, never by a
		// cross-curve margin.
		const venueStub = {
			walletReserveForToken: () => 0n,
			planWithdrawalForToken: async () => ({ calls: [], credited: 0n }),
		} as any
		const filler = gateFiller(
			[{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000") }], // curve-less → venue-priced
			usdcOnBoth(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
			{ fundingVenues: [venueStub] },
		)
		// Pool mid: 1500 CNGN per USD (USD per CNGN = 1/1500).
		;(filler as any).getVenueUsdPrice = async () => new Decimal(1).div(new Decimal("1500"))
		const o = order(
			"venue-fair",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("150000", 18) }, // exactly 100 × 1500
			parseUnits("10", 6),
		)
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
	})

	it("referenceOnly pairs feed the anchor graph but never match orders", async () => {
		const filler = gateFiller(
			[
				// Reference quote: anchors CNGN at $1/1565 (mid of 1580/1550), no market.
				{
					token0: "USDC",
					token1: "CNGN",
					maxOrderSize: size("0"),
					referenceOnly: true,
					bidPricePolicy: flat("1580"),
					askPricePolicy: flat("1550"),
				},
				{ token0: "CNGN", token1: "CNGN", maxOrderSize: size("5000000"), askPricePolicy: flat("0.995") },
			],
			usdcOnBoth(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		// A USDC→CNGN order would match the reference pair if it were a market — it must not.
		const fx = order(
			"ref-no-market",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1500000", 18) },
			parseUnits("10", 6),
		)
		expect(await filler.canFill(fx)).toBe(false)
		// The same-token market fills, and its confirmation notional is priced
		// through the reference edge: 1,565,000 CNGN ÷ 1565 = $1000.
		const transfer = order(
			"ref-anchored",
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1565000", 18) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1500000", 18) },
			parseUnits("10", 6),
		)
		expect(await filler.canFill(transfer)).toBe(true)
		expect((await filler.getOrderUsdValue(transfer))?.inputUsd.toFixed(4)).toBe("1000.0000")
	})

	it("keeps USD stables pinned at $1 — a stable/stable curve contributes no FX edge", async () => {
		// A (mis-set) USDC/USDT curve at 0.95 must never re-price USDT: both
		// sides are $1 anchors, so 1000 USDT reads as $1000, not $950 or $1052.
		const filler = gateFiller(
			[
				{ token0: "USDC", token1: "USDT", maxOrderSize: size("100000"), askPricePolicy: flat("0.95") },
				{ token0: "USDT", token1: "CNGN", maxOrderSize: size("100000"), askPricePolicy: flat("1500") },
			],
			usdcOnBoth(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"stable-pin",
			{ token: bytes20ToBytes32(USDT), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1400000", 18) },
			parseUnits("10", 6),
		)
		expect((await filler.getOrderUsdValue(o))?.inputUsd.toFixed(6)).toBe("1000.000000")
	})

	it("resolves conflicting anchor routes deterministically — first declared pair wins", async () => {
		// USDC/ZARP (mid 18) is declared before USDT/ZARP (mid 20), so ZARP's
		// factor comes from the first: 1800 ZARP = $100, not $90.
		const filler = gateFiller(
			[
				usdcZarpAnchor(),
				{ token0: "USDT", token1: "ZARP", maxOrderSize: size("100000"), askPricePolicy: flat("20") },
				{ token0: "ZARP", token1: "CNGN", maxOrderSize: size("1000000"), bidPricePolicy: flat("100"), askPricePolicy: flat("95") },
			],
			zarpCngnRegistry(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"first-edge",
			{ token: bytes20ToBytes32(ZARP), amount: parseUnits("1800", 18) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("170000", 18) },
			parseUnits("10", 6),
		)
		expect((await filler.getOrderUsdValue(o))?.inputUsd.toFixed(6)).toBe("100.000000")
	})

	it("addPair opens a live market; duplicate, reverse and unanchored adds are rejected atomically", async () => {
		const filler = gateFiller(
			[{ token0: "USDC", token1: "CNGN", maxOrderSize: size("100000"), askPricePolicy: flat("1500") }],
			usdcOnBoth(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		const o = order(
			"added-market",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
			{ token: bytes20ToBytes32(USDT), amount: parseUnits("999", 6) },
			parseUnits("100", 6),
		)
		expect(await filler.canFill(o)).toBe(false)

		filler.addPair({
			token0: "USDC",
			token1: "USDT",
			maxOrderSize: size("100000"),
			bidPricePolicy: flat("1.005"),
			askPricePolicy: flat("0.999"),
		})
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)

		const usdtPair = () => ({ token0: "USDT", token1: "USDC", maxOrderSize: size("1"), askPricePolicy: flat("0.9") })
		expect(() => filler.addPair({ ...usdtPair(), token0: "USDC", token1: "USDT" })).toThrow(/duplicate market/)
		expect(() => filler.addPair(usdtPair())).toThrow(/duplicate market/)
		// ZARP/ZWL is an island — no curve path to a USD stable.
		expect(() =>
			filler.addPair({ token0: "ZARP", token1: "ZWL", maxOrderSize: size("1000"), askPricePolicy: flat("95") }),
		).toThrow(/no USD anchor/)
	})

	it("removePair closes a live market; anchor-orphaning and last-market removals are rejected", async () => {
		const reference: TradingPair = {
			token0: "USDC",
			token1: "ZARP",
			maxOrderSize: size("0"),
			referenceOnly: true,
			askPricePolicy: flat("17.8"),
		}
		const zarpCngn: TradingPair = {
			token0: "ZARP",
			token1: "CNGN",
			maxOrderSize: size("1000000"),
			askPricePolicy: flat("95"),
		}
		const usdcCngn: TradingPair = {
			token0: "USDC",
			token1: "CNGN",
			maxOrderSize: size("100000"),
			askPricePolicy: flat("1400"),
		}
		const filler = gateFiller([reference, zarpCngn], zarpCngnRegistry(), {
			fillGas: parseUnits("1", 6),
			relayer: parseUnits("1", 6),
		})
		// The reference feed is ZARP's only path to a USD stable — removing it
		// would orphan the ZARP/CNGN market.
		expect(() => filler.removePair(reference)).toThrow(/no USD anchor/)

		filler.addPair(usdcCngn)
		const o = order(
			"removed-market",
			{ token: bytes20ToBytes32(USDC), amount: parseUnits("1", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1396", 18) },
			parseUnits("10", 6),
		)
		expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
		filler.removePair(usdcCngn)
		expect(await filler.canFill(o)).toBe(false)

		filler.removePair(zarpCngn)
		expect(() => filler.removePair(reference)).toThrow(/cannot be removed/)
	})
})

describe("pickAnchorStable", () => {
	it("prefers USDC when no stable has a market against the symbol", () => {
		expect(pickAnchorStable([{ token0: "CNGN", token1: "CNGN" }], "CNGN")).toBe("USDC")
	})

	it("skips a stable with an existing orientation, in either direction", () => {
		expect(pickAnchorStable([{ token0: "USDC", token1: "CNGN" }], "CNGN")).toBe("USDT")
		expect(pickAnchorStable([{ token0: "CNGN", token1: "USDC" }], "CNGN")).toBe("USDT")
	})

	it("returns null when every stable is taken", () => {
		const pairs = [
			{ token0: "USDC", token1: "CNGN" },
			{ token0: "USDT", token1: "CNGN" },
			{ token0: "CNGN", token1: "DAI" },
		]
		expect(pickAnchorStable(pairs, "CNGN")).toBeNull()
	})
})

describe("assertPairSymbolsResolve", () => {
	const resolver = (addresses: Record<string, Record<string, string>>): PairAddressResolver => ({
		getAddress: (symbol, chain) => addresses[symbol.trim().toUpperCase()]?.[chain] ?? null,
	})

	it("accepts symbols that resolve somewhere and to distinct contracts", () => {
		const registry = resolver({
			USDC: { "EVM-8453": "0x1111111111111111111111111111111111111111" },
			CNGN: { "EVM-8453": "0x2222222222222222222222222222222222222222" },
		})
		expect(() =>
			assertPairSymbolsResolve(
				[{ token0: "USDC", token1: "CNGN", maxOrderSize: "1000" }],
				registry,
				["EVM-8453", "EVM-1"],
			),
		).not.toThrow()
	})

	it("rejects a symbol deployed on none of the configured chains", () => {
		const registry = resolver({
			USDC: { "EVM-8453": "0x1111111111111111111111111111111111111111" },
			CNGN: { "EVM-56": "0x2222222222222222222222222222222222222222" },
		})
		expect(() =>
			assertPairSymbolsResolve([{ token0: "USDC", token1: "CNGN", maxOrderSize: "1000" }], registry, ["EVM-8453"]),
		).toThrow(/'CNGN' does not resolve/)
	})

	it("rejects two symbols aliasing the same contract on a chain", () => {
		const shared = "0x1111111111111111111111111111111111111111"
		const registry = resolver({
			USDC: { "EVM-8453": shared },
			FAKE: { "EVM-8453": shared },
		})
		expect(() =>
			assertPairSymbolsResolve([{ token0: "USDC", token1: "FAKE", maxOrderSize: "1000" }], registry, ["EVM-8453"]),
		).toThrow(/both resolve to/)
	})
})
