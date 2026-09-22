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
import type { LimitOrderStore } from "@/data/types"
import { limitOrderStore } from "./helpers/limit-orders"

// Pure unit tests for the asset registry, [[pairs]] validation, and the
// pairs-driven FX engine's order gating (mocked services).

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

	it("names the symbol an address is known by, and nothing for one it does not hold", () => {
		const registry = new AssetRegistry(resolver as any, { CNGN: { [CHAIN]: CNGN } })
		expect(registry.symbolFor(USDC, CHAIN)).toBe("USDC")
		expect(registry.symbolFor(CNGN, CHAIN)).toBe("CNGN")
		// Case of the caller's address must not matter.
		expect(registry.symbolFor(USDC.toUpperCase(), CHAIN)).toBe("USDC")
		expect(registry.symbolFor("0x9999999999999999999999999999999999999999", CHAIN)).toBeNull()
		// Known symbol, but not deployed on that chain.
		expect(registry.symbolFor(CNGN, OTHER_CHAIN)).toBeNull()
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
					{ token0: "USDC", token1: "CNGN" },
					{ token0: "USDT", token1: "CNGN" },
					{ token0: "ZARP", token1: "CNGN" },
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
					},
				],
				assets,
			),
		).not.toThrow()
		expect(() =>
			validatePairConfigs(
				[{ token0: "USDC", token1: "CNGN" }],
				assets,
			),
		).not.toThrow()
	})

	it("accepts registry-shipped symbols with no [assets] config at all", () => {
		// ZARP anchors through CNGN (via USDC/CNGN), EURC directly via USDC/EURC.
		expect(() =>
			validatePairConfigs([
				{ token0: "USDC", token1: "CNGN" },
				{ token0: "ZARP", token1: "CNGN" },
				{ token0: "USDC", token1: "EURC" },
				{ token0: "EURC", token1: "XSGD" },
			]),
		).not.toThrow()
	})

	it("rejects unknown symbols and duplicates", () => {
		expect(() =>
			validatePairConfigs([{ token0: "USDC", token1: "WAT" }], assets),
		).toThrow(/unknown symbol/)
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN" },
					{ token0: "usdc", token1: "cngn" },
				],
				assets,
			),
		).toThrow(/declared twice/)
		// The reverse orientation is the same market — accepting both would make
		// leg matching declaration-order dependent (and price legs in the wrong unit).
		expect(() =>
			validatePairConfigs(
				[
					{ token0: "USDC", token1: "CNGN" },
					{ token0: "CNGN", token1: "USDC" },
				],
				assets,
			),
		).toThrow(/already declared/)
	})

})

// ---------------------------------------------------------------------------
// FX engine pair matching and construction. Payout sizing at a pair's own rate
// is covered by the exposure-cap and profit-gates suites.
// ---------------------------------------------------------------------------

function makeContractService(): any {
	const cache = new Map<string, unknown>()
	const decimals: Record<string, number> = {
		[USDC.toLowerCase()]: 6,
		[USDT.toLowerCase()]: 6,
		[CNGN.toLowerCase()]: 18,
		[ZARP.toLowerCase()]: 18,
	}
	const bidPlans = new Map<string, unknown>()
	return {
		cacheService: {
			getPairClassifications: (id: string) => cache.get(`pc:${id}`),
			setPairClassifications: (id: string, pairs: unknown) => cache.set(`pc:${id}`, pairs),
			setFillerOutputs: (id: string, outputs: unknown) => cache.set(`fo:${id}`, outputs),
			setMatchedLimitOrder: () => {},
			setBidPlans: (id: string, plans: unknown) => bidPlans.set(id, plans),
			getBidPlans: (id: string) => bidPlans.get(id) ?? [],
			clearBidPlans: (id: string) => bidPlans.delete(id),
			clearPartialFill: (id: string) => cache.delete(`pf:${id}`),
			setPartialFill: (id: string, partial: boolean) => cache.set(`pf:${id}`, partial),
			isPartialFill: (id: string) => cache.get(`pf:${id}`) === true,
		},
		getTokenDecimals: async (token: string) => decimals[token.toLowerCase()] ?? 18,
	}
}

function makeFiller(pairs: TradingPair[], limitOrders?: LimitOrderStore) {
	const configService = {
		...resolver,
		getMaxOverfillBps: () => 500n,
		getMaxConsecutiveClamps: () => 3,
	} as any
	const registry = new AssetRegistry(configService, {
		CNGN: { [CHAIN]: CNGN },
		ZARP: { [CHAIN]: ZARP },
	})
	const signer = { address: SOLVER } as any
	return new FXFiller(signer, configService, {} as any, makeContractService(), pairs, registry, { limitOrders })
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
	it("rejects duplicate and reverse-duplicate engine pairs", () => {
		const registry = new AssetRegistry(resolver as any, { CNGN: { [CHAIN]: CNGN } })
		const signer = { address: SOLVER } as any
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
				{ token0: "USDC", token1: "CNGN" },
				{ token0: "CNGN", token1: "USDC" },
			]),
		).toThrow(/duplicate market/)
	})

	it("rejects orders whose legs match no configured pair", async () => {
		const filler = makeFiller([
			{ token0: "USDC", token1: "CNGN" },
		])
		const order = makeOrder(
			"no-pair",
			{ token: bytes20ToBytes32(USDT), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		expect(await filler.canFill(order)).toBe(false)
	})

	it("accepts same-chain cross-asset orders — only same-token pairs are chain-restricted", async () => {
		// makeOrder is source == destination: an on-chain USDC→CNGN swap. The
		// same-chain rejection applies to same-token self-swaps, never to FX.
		const filler = makeFiller(
			[{ token0: "USDC", token1: "CNGN" }],
			await limitOrderStore([
				{ base: "USDC", quote: "CNGN", side: "BID", fillChain: CHAIN, price: "1500", size: "1500000" },
			]),
		)
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
	const signer = { address: SOLVER } as any
	const usdcUsdc = (): TradingPair[] => [
		{ token0: "USDC", token1: "USDC" },
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

	it("accepts a cross-chain same-token order (the same-asset transfer market)", async () => {
		const filler = new FXFiller(signer, cfg, {} as any, makeContractService(), usdcUsdc(), new AssetRegistry(cfg), {
			limitOrders: await limitOrderStore([
				{
					base: "USDC",
					quote: "USDC",
					side: "BID",
					fillChain: CHAIN_B,
					price: "0.999",
					size: "100000",
					acceptedSources: [CHAIN_A],
				},
			]),
		})
		expect(await filler.canFill(sameTokenOrder(CHAIN_A, CHAIN_B))).toBe(true)
	})

	it("does not serve a chain the order never declared", async () => {
		const filler = new FXFiller(signer, cfg, {} as any, makeContractService(), usdcUsdc(), new AssetRegistry(cfg), {
			limitOrders: await limitOrderStore([
				{
					base: "USDC",
					quote: "USDC",
					side: "BID",
					fillChain: CHAIN_B,
					price: "0.999",
					size: "100000",
					acceptedSources: ["EVM-42161"],
				},
			]),
		})
		expect(await filler.canFill(sameTokenOrder(CHAIN_A, CHAIN_B))).toBe(false)
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
		// No paymaster on any chain: the leg loop's paymasterReserveForToken
		// consults these and must reserve nothing here — this suite asserts
		// exact spread/fee arithmetic with no gas headroom held back.
		getSimplexPaymasterAddress: () => undefined,
	} as any
	const signer = { address: SOLVER } as any

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
		options?: {
			fundingVenues?: unknown[]
			balancesByToken?: Record<string, bigint>
			/** What the operator is offering. Without it the filler prices nothing. */
			limitOrders?: LimitOrderStore
		},
	) {
		const cache = new Map<string, unknown>()
		const bidPlans = new Map<string, unknown>()
		const contractService = {
			cacheService: {
				getPairClassifications: (id: string) => cache.get(`pc:${id}`),
				setPairClassifications: (id: string, v: unknown) => cache.set(`pc:${id}`, v),
				setFillerOutputs: () => {},
				setMatchedLimitOrder: () => {},
				setBidPlans: (id: string, plans: unknown) => bidPlans.set(id, plans),
				getBidPlans: (id: string) => bidPlans.get(id) ?? [],
				clearBidPlans: (id: string) => bidPlans.delete(id),
				setFundingPrepends: () => {},
				clearFundingPrepends: () => {},
				clearPartialFill: (id: string) => cache.delete(`pf:${id}`),
				setPartialFill: (id: string, partial: boolean) => cache.set(`pf:${id}`, partial),
				isPartialFill: (id: string) => cache.get(`pf:${id}`) === true,
			},
			getFeeTokenWithDecimals: async () => ({ decimals: 6, address: USDC }),
			getTokenDecimals: async (token: string) => decimalsByAddr[token.toLowerCase()] ?? 18,
			partialFillsFor: async (o: Order) => o.output.assets.map(() => 0n),
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
			readContract: async (params: { address: string }) =>
				options?.balancesByToken?.[params.address.toLowerCase()] ?? 10n ** 30n, // balanceOf
		}
		const clientManager = { getPublicClient: () => destClient } as any
		return new FXFiller(signer, cfg, clientManager, contractService, pairs, registry, {
			fundingVenues: (options?.fundingVenues ?? []) as any,
			limitOrders: options?.limitOrders,
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
			[{ token0: "USDC", token1: "CNGN" }],
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
			[{ token0: "USDC", token1: "CNGN" }],
			usdcOnBoth(),
			{ fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) },
			{
				limitOrders: await limitOrderStore([
					{ base: "USDC", quote: "CNGN", side: "BID", fillChain: DST, price: "1450", size: "10000000", acceptedSources: [SRC] },
				]),
			},
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
			[{ token0: "USDC", token1: "USDC" }],
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
				{ token0: "USDC", token1: "CNGN" },
				{ token0: "CNGN", token1: "CNGN" },
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
			[{ token0: "USDC", token1: "USDC" }],
			new AssetRegistry(cfg),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
			{
				limitOrders: await limitOrderStore([
					{ base: "USDC", quote: "USDC", side: "BID", fillChain: DST, price: "0.999", size: "100000", acceptedSources: [SRC] },
				]),
			},
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
		token1: "ZARP", // mid 18 ZARP per USDC → ZARP ≈ $1/18
	})

	it("rejects an order when a leg demands more than its own ask curve yields", async () => {
		// The USDC/CNGN leg demands 1.6M CNGN but the 1450 ask pays at most
		// 1.45M for 1000 USDC. Each leg is checked against its own curve only —
		// the comfortably-within-curve ZARP/CNGN leg cannot rescue the order.
		const filler = gateFiller(
			[
				{ token0: "USDC", token1: "CNGN" },
				{ token0: "ZARP", token1: "CNGN" },
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

	it("addPair rejects a duplicate and its reverse", async () => {
		const filler = gateFiller(
			[{ token0: "USDC", token1: "CNGN" }],
			usdcOnBoth(),
			{ fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) },
		)
		filler.addPair({
			token0: "USDC",
			token1: "USDT",
		})

		const usdtPair = () => ({ token0: "USDT", token1: "USDC" })
		expect(() => filler.addPair({ ...usdtPair(), token0: "USDC", token1: "USDT" })).toThrow(/duplicate market/)
		expect(() => filler.addPair(usdtPair())).toThrow(/duplicate market/)
	})

	it("removePair closes a market and refuses to remove the last one", async () => {
		const reference: TradingPair = {
			token0: "USDC",
			token1: "ZARP",
		}
		const zarpCngn: TradingPair = {
			token0: "ZARP",
			token1: "CNGN",
		}
		const usdcCngn: TradingPair = {
			token0: "USDC",
			token1: "CNGN",
		}
		const filler = gateFiller([reference, zarpCngn], zarpCngnRegistry(), {
			fillGas: parseUnits("1", 6),
			relayer: parseUnits("1", 6),
		})
		filler.addPair(usdcCngn)
		filler.removePair(usdcCngn)

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
				[{ token0: "USDC", token1: "CNGN" }],
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
			assertPairSymbolsResolve([{ token0: "USDC", token1: "CNGN" }], registry, ["EVM-8453"]),
		).toThrow(/'CNGN' does not resolve/)
	})

	it("rejects two symbols aliasing the same contract on a chain", () => {
		const shared = "0x1111111111111111111111111111111111111111"
		const registry = resolver({
			USDC: { "EVM-8453": shared },
			FAKE: { "EVM-8453": shared },
		})
		expect(() =>
			assertPairSymbolsResolve([{ token0: "USDC", token1: "FAKE" }], registry, ["EVM-8453"]),
		).toThrow(/both resolve to/)
	})
})
