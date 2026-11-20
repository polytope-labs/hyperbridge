import { describe, it, expect } from "vitest"
import { Decimal } from "decimal.js"
import { parseUnits } from "viem"
import {
	bytes20ToBytes32,
	ChainConfigService,
	SUPPORTED_INTENTS_VERSION,
	type HexString,
	type Order,
	type TokenInfo,
} from "@hyperbridge/sdk"
import { AssetRegistry, validateAssetDefinitions, type BuiltinAssetResolver } from "@/config/asset-registry"
import { FXFiller } from "@/strategies/fx"
import type { LimitOrderStore } from "@/data/types"
import { limitOrderStore } from "./helpers/limit-orders"

// Pure unit tests for the asset registry and the FX engine's order gating
// (mocked services).

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
		// BRIDGE is the same CREATE2 address on both of its chains, with OpenZeppelin's ERC20 layout.
		for (const chain of ["EVM-56", "EVM-137"]) {
			expect(registry.getAddress("BRIDGE", chain)).toBe("0x5b0c50fDd52ECC0d4c682c441eaBaD41FfDEABBB")
			expect(sdk.getAssetMetadataBySymbol(chain, "bridge")?.decimals).toBe(18)
			expect(sdk.getTokenStorageSlots(chain, "0x5b0c50fDd52ECC0d4c682c441eaBaD41FfDEABBB")).toEqual({
				balanceSlot: 0,
				allowanceSlot: 1,
			})
		}
		// Not deployed there → absent, not an error.
		expect(registry.getAddress("EURC", "EVM-56")).toBeNull()
		expect(registry.getAddress("USDR", "EVM-42161")).toBeNull()
		expect(registry.getAddress("BRIDGE", "EVM-1")).toBeNull()
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
			clearFeeCheckWaived: (id: string) => cache.delete(`fw:${id}`),
			setFeeCheckWaived: (id: string, waived: boolean) => cache.set(`fw:${id}`, waived),
			isFeeCheckWaived: (id: string) => cache.get(`fw:${id}`) === true,
		},
		getTokenDecimals: async (token: string) => decimals[token.toLowerCase()] ?? 18,
	}
}

function makeFiller(limitOrders?: LimitOrderStore) {
	const configService = {
		...resolver,
	} as any
	const registry = new AssetRegistry(configService, {
		CNGN: { [CHAIN]: CNGN },
		ZARP: { [CHAIN]: ZARP },
	})
	const signer = { address: SOLVER } as any
	return new FXFiller(signer, configService, {} as any, makeContractService(), registry, { limitOrders })
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

describe("FXFiller order gating", () => {
	it("rejects orders no limit order serves", async () => {
		const filler = makeFiller()
		const order = makeOrder(
			"no-pair",
			{ token: bytes20ToBytes32(USDT), amount: parseUnits("100", 6) },
			{ token: bytes20ToBytes32(CNGN), amount: 0n },
		)
		expect(await filler.canFill(order)).toBe(false)
	})

	it("accepts same-chain cross-asset orders — only same-token orders are chain-restricted", async () => {
		// makeOrder is source == destination: an on-chain USDC→CNGN swap. The
		// same-chain rejection applies to same-token self-swaps, never to FX.
		const filler = makeFiller(
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
	} as any
	const signer = { address: SOLVER } as any

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
		const filler = new FXFiller(signer, cfg, {} as any, makeContractService(), new AssetRegistry(cfg), {
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
		const filler = new FXFiller(signer, cfg, {} as any, makeContractService(), new AssetRegistry(cfg), {
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
	const GATEWAY = "0x6666666666666666666666666666666666666666" as HexString
	const cfg = {
		getUsdcAsset: () => USDC,
		getUsdtAsset: () => USDT,
		getDaiAsset: () => {
			throw new Error("nc")
		},
		getCNgnAsset: () => undefined,
		// No paymaster on any chain: the leg loop's paymasterReserveForToken
		// consults these and must reserve nothing here — this suite asserts
		// exact spread/fee arithmetic with no gas headroom held back.
		getSimplexPaymasterAddress: () => undefined,
		// A cross-chain partial confirms the source gateway's release before it is bid on.
		getIntentGatewayAddress: () => GATEWAY,
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
		registry: AssetRegistry,
		estimate: { fillGas: bigint; relayer: bigint },
		options?: {
			fundingVenues?: unknown[]
			balancesByToken?: Record<string, bigint>
			/** What the operator is offering. Without it the filler prices nothing. */
			limitOrders?: LimitOrderStore
			/** `simplex.minOrderSizeUsd`. The default when omitted. */
			minOrderSizeUsd?: number
			/** `simplex.minPartialFillUsd`. The default when omitted. */
			minPartialFillUsd?: number
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
				clearFeeCheckWaived: (id: string) => cache.delete(`fw:${id}`),
				setFeeCheckWaived: (id: string, waived: boolean) => cache.set(`fw:${id}`, waived),
				isFeeCheckWaived: (id: string) => cache.get(`fw:${id}`) === true,
			},
			getFeeTokenWithDecimals: async () => ({ decimals: 6, address: USDC }),
			getTokenDecimals: async (token: string) => decimalsByAddr[token.toLowerCase()] ?? 18,
			partialFillsFor: async (o: Order) => o.output.assets.map(() => 0n),
			limitOrderSpent: async () => 0n,
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
			readContract: async (params: { address: string; functionName?: string }) =>
				params.address === GATEWAY && params.functionName === "version"
					? SUPPORTED_INTENTS_VERSION
					: (options?.balancesByToken?.[params.address.toLowerCase()] ?? 10n ** 30n), // balanceOf
		}
		const clientManager = { getPublicClient: () => destClient } as any
		const config = {
			...cfg,
			getMinOrderSizeUsd: () => options?.minOrderSizeUsd ?? 20,
			getMinPartialFillUsd: () => options?.minPartialFillUsd ?? 20,
		}
		return new FXFiller(signer, config, clientManager, contractService, registry, {
			fundingVenues: (options?.fundingVenues ?? []) as any,
			limitOrders: options?.limitOrders,
		})
	}

	const feeCheckWaived = (filler: FXFiller, id: string): boolean =>
		(filler as any).contractService.cacheService.isFeeCheckWaived(id)
	const partialFill = (filler: FXFiller, id: string): boolean =>
		(filler as any).contractService.cacheService.isPartialFill(id)
	const bidPlans = (filler: FXFiller, id: string): unknown[] =>
		(filler as any).contractService.cacheService.getBidPlans(id)

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

	/**
	 * The same registry as `usdcOnBoth`, but cNGN is listed under `ZCNGN` first so
	 * `symbolFor` names the address `ZCNGN` while the limit orders price it as
	 * `CNGN`. The payout therefore has no USD price, and only the input-value
	 * fallback can value the bid.
	 */
	const aliasedRegistry = () =>
		new AssetRegistry(cfg, {
			ZCNGN: { [DST]: CNGN },
			CNGN: { [SRC]: CNGN, [DST]: CNGN },
		})

	it("gate 1: rejects when fees do not cover fill gas + relayer fee", async () => {
		const filler = gateFiller(
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

	describe("gate 1 applies below `minOrderSizeUsd` only", () => {
		// Execution costs $5 and every order here carries $4 of fees.
		const estimate = { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }
		const book = () =>
			limitOrderStore([
				{
					base: "USDC",
					quote: "CNGN",
					side: "BID",
					fillChain: DST,
					price: "1450",
					size: "10000000",
					acceptedSources: [SRC],
				},
			])
		/** `usd` of USDC in, asking 1400 CNGN per dollar, with fees a dollar short of the cost. */
		const underpaying = (id: string, usd: string) =>
			order(
				id,
				{ token: bytes20ToBytes32(USDC), amount: parseUnits(usd, 6) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits(usd, 18) * 1400n },
				parseUnits("4", 6),
			)
		const fx = async (minOrderSizeUsd?: number) =>
			gateFiller(usdcOnBoth(), estimate, {
				limitOrders: await book(),
				minOrderSizeUsd,
			})

		it("rejects an order below it whose fees do not cover execution", async () => {
			const filler = await fx()
			const o = underpaying("small", "19.99")

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(feeCheckWaived(filler, o.id!)).toBe(false)
		})

		it("bids on an order at it whatever fees it carries, and scores the shortfall", async () => {
			const filler = await fx()
			const o = underpaying("at-min", "20")

			// $4 of fees against $5 of cost.
			expect(await filler.calculateProfitability(o)).toBe(-1)
			expect(feeCheckWaived(filler, o.id!)).toBe(true)
		})

		it("does not mark an order whose fees cover execution", async () => {
			const filler = await fx()
			const o = order(
				"covered",
				{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1400000", 18) },
				parseUnits("10", 6),
			)

			expect(await filler.calculateProfitability(o)).toBeGreaterThan(0)
			expect(feeCheckWaived(filler, o.id!)).toBe(false)
		})

		it("follows the configured size", async () => {
			const o = underpaying("configured", "1000")

			expect(await (await fx(5000)).calculateProfitability(o)).toBe(0)
			expect(await (await fx(1000)).calculateProfitability(o)).toBe(-1)
		})

		it("checks no order's fees at zero", async () => {
			const filler = await fx(0)
			const o = underpaying("no-check", "1")

			expect(await filler.calculateProfitability(o)).toBe(-1)
			expect(feeCheckWaived(filler, o.id!)).toBe(true)
		})

		it("clears the mark when a later evaluation refuses the order", async () => {
			const filler = await fx()
			const o = underpaying("re-evaluated", "20")
			await filler.calculateProfitability(o)
			expect(feeCheckWaived(filler, o.id!)).toBe(true)

			// The same order re-priced against a book with nothing left to match it.
			;(filler as any).limitOrders = await limitOrderStore([])
			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(feeCheckWaived(filler, o.id!)).toBe(false)
		})
	})

	describe("a partial fill is held to `minPartialFillUsd`", () => {
		// The book pays 1450 CNGN per USDC; `size` is what it has left to pay, so a
		// small one leaves the leg short of its 1.4M CNGN ask: a partial.
		const book = (size: string) =>
			limitOrderStore([
				{ base: "USDC", quote: "CNGN", side: "BID", fillChain: DST, price: "1450", size, acceptedSources: [SRC] },
			])
		/** 1000 USDC in, asking 1.4M CNGN — a full offer is 1.45M. */
		const partialOrder = (id: string, fees = parseUnits("100", 6)) =>
			order(
				id,
				{ token: bytes20ToBytes32(USDC), amount: parseUnits("1000", 6) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1400000", 18) },
				fees,
			)

		it("skips a partial worth less than the floor, planning nothing", async () => {
			// 0.01 CNGN left on the order: about $0.0000069.
			const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("0.01"),
			})
			const o = partialOrder("dust")

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(bidPlans(filler, o.id!)).toHaveLength(0)
			expect(partialFill(filler, o.id!)).toBe(false)
		})

		it("bids a partial at or above the floor", async () => {
			// 100,000 CNGN left: about $69.
			const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("100000"),
			})
			const o = partialOrder("worthwhile")

			await filler.calculateProfitability(o)
			expect(bidPlans(filler, o.id!)).toHaveLength(1)
			expect(partialFill(filler, o.id!)).toBe(true)
		})

		it("bids the dust at zero — the old behaviour, by the operator's choice", async () => {
			const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("0.01"),
				minPartialFillUsd: 0,
			})
			const o = partialOrder("no-floor")

			await filler.calculateProfitability(o)
			expect(bidPlans(filler, o.id!)).toHaveLength(1)
			expect(partialFill(filler, o.id!)).toBe(true)
		})

		it("skips a partial a wallet just above its reserve can only dust", async () => {
			// The wallet holds 1 CNGN and reserves 0.999 of it: 0.001 CNGN spendable,
			// well under the floor, even though the limit order offers plenty.
			const venue = {
				planWithdrawalForToken: async () => ({ calls: [], credited: 0n }),
				walletReserveForToken: () => parseUnits("0.999", 18),
			}
			const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("100000"),
				fundingVenues: [venue],
				balancesByToken: { [CNGN.toLowerCase()]: parseUnits("1", 18) },
			})
			const o = partialOrder("dust-wallet", parseUnits("4", 6))

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(bidPlans(filler, o.id!)).toHaveLength(0)
		})

		it("skips a partial nothing can price", async () => {
			// ZARP/CNGN reaches no dollar stable, so neither the payout nor the input
			// the bid releases has a price; an unpriceable bid is skipped.
			const filler = gateFiller(zarpCngnRegistry(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await limitOrderStore([
					{ base: "ZARP", quote: "CNGN", side: "BID", fillChain: DST, price: "95", size: "10", acceptedSources: [SRC] },
				]),
			})
			const o = order(
				"unpriceable",
				{ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits("95000", 18) },
				parseUnits("4", 6),
			)

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(bidPlans(filler, o.id!)).toHaveLength(0)
		})

		it("refuses a known partial nothing can price before any venue planning", async () => {
			// Same ZARP/CNGN book, with a wallet that holds nothing: absent the cheap
			// pass the shortfall would be planned from the venue, and reserve its
			// liquidity, for a bid the final pass was always going to refuse.
			const planWithdrawalForToken = vi.fn(async () => ({ calls: [], credited: 0n }))
			const venue = { planWithdrawalForToken, walletReserveForToken: () => 0n }
			const filler = gateFiller(zarpCngnRegistry(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await limitOrderStore([
					{ base: "ZARP", quote: "CNGN", side: "BID", fillChain: DST, price: "95", size: "10", acceptedSources: [SRC] },
				]),
				fundingVenues: [venue],
				balancesByToken: { [CNGN.toLowerCase()]: 0n },
			})
			const o = order(
				"unpriceable-no-venue",
				{ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits("95000", 18) },
				parseUnits("4", 6),
			)

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(planWithdrawalForToken).not.toHaveBeenCalled()
			expect(bidPlans(filler, o.id!)).toHaveLength(0)
		})

		it("bids the unpriceable partial at a floor of zero", async () => {
			const filler = gateFiller(zarpCngnRegistry(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await limitOrderStore([
					{ base: "ZARP", quote: "CNGN", side: "BID", fillChain: DST, price: "95", size: "10", acceptedSources: [SRC] },
				]),
				minPartialFillUsd: 0,
			})
			const o = order(
				"unpriceable-no-floor",
				{ token: bytes20ToBytes32(ZARP), amount: parseUnits("1000", 18) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits("95000", 18) },
				parseUnits("4", 6),
			)

			await filler.calculateProfitability(o)
			expect(bidPlans(filler, o.id!)).toHaveLength(1)
		})

		it("leaves a full fill below the floor alone", async () => {
			// 1 USDC in, 1450 CNGN out (about $1) covered whole by the limit order:
			// the floor is a partial's rule and does not touch a full fill.
			const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("100000"),
			})
			const o = order(
				"small-full",
				{ token: bytes20ToBytes32(USDC), amount: parseUnits("1", 6) },
				{ token: bytes20ToBytes32(CNGN), amount: parseUnits("1440", 18) },
				parseUnits("100", 6),
			)

			await filler.calculateProfitability(o)
			expect(bidPlans(filler, o.id!)).toHaveLength(1)
			expect(partialFill(filler, o.id!)).toBe(false)
		})

		it("values a partial by the input it releases when the payout has no price", async () => {
			// The payout token is named `ZCNGN`, which the limit-order rates never
			// mention: 0.01 cNGN released is about $0.0000069 either way, below the floor.
			const filler = gateFiller(aliasedRegistry(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("0.01"),
			})
			const o = partialOrder("fallback-dust")

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(bidPlans(filler, o.id!)).toHaveLength(0)
		})

		it("bids a partial the input fallback puts above the floor", async () => {
			// 100,000 cNGN released is about $69 of input at the order's own rate.
			const filler = gateFiller(aliasedRegistry(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("100000"),
			})
			const o = partialOrder("fallback-worthwhile")

			await filler.calculateProfitability(o)
			expect(bidPlans(filler, o.id!)).toHaveLength(1)
			expect(partialFill(filler, o.id!)).toBe(true)
		})

		it("does not plan a venue withdrawal for a dust partial", async () => {
			// The wallet holds nothing, so absent the cheap pass the shortfall would be
			// planned from the venue even though the bid is dust. It must not be touched.
			const planWithdrawalForToken = vi.fn(async () => ({ calls: [], credited: 0n }))
			const venue = { planWithdrawalForToken, walletReserveForToken: () => 0n }
			const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
				limitOrders: await book("0.01"),
				fundingVenues: [venue],
				balancesByToken: { [CNGN.toLowerCase()]: 0n },
			})
			const o = partialOrder("dust-no-venue")

			expect(await filler.calculateProfitability(o)).toBe(0)
			expect(planWithdrawalForToken).not.toHaveBeenCalled()
		})

		describe("a multi-leg order's leg, fully covered but partial by the leg count", () => {
			// One leg that matches and is covered whole, so `partialFill` is set only by
			// the multi-leg rule; a second leg nothing serves, present to make it multi-leg.
			const multiLegOrder = (id: string, usd: string, requested: string): Order => {
				const o = order(
					id,
					{ token: bytes20ToBytes32(USDC), amount: parseUnits(usd, 6) },
					{ token: bytes20ToBytes32(CNGN), amount: parseUnits(requested, 18) },
					parseUnits("100", 6),
				)
				o.inputs.push({ token: bytes20ToBytes32(USDT), amount: parseUnits("1", 6) })
				o.output.assets.push({ token: bytes20ToBytes32(CNGN), amount: parseUnits("1", 18) })
				return o
			}

			it("skips the leg when it is worth less than the floor", async () => {
				// 0.01 USDC in, 14 cNGN asked, 14.5 covered whole: about $0.01.
				const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
					limitOrders: await book("2000000"),
				})
				const o = multiLegOrder("multi-dust", "0.01", "14")

				expect(await filler.calculateProfitability(o)).toBe(0)
				expect(bidPlans(filler, o.id!)).toHaveLength(0)
			})

			it("bids the leg when it is worth more than the floor", async () => {
				// 1000 USDC in, 1.4M cNGN asked, 1.45M covered whole: about $1000.
				const filler = gateFiller(usdcOnBoth(), { fillGas: parseUnits("2", 6), relayer: parseUnits("3", 6) }, {
					limitOrders: await book("2000000"),
				})
				const o = multiLegOrder("multi-worth", "1000", "1400000")

				await filler.calculateProfitability(o)
				expect(bidPlans(filler, o.id!)).toHaveLength(1)
				expect(partialFill(filler, o.id!)).toBe(true)
			})
		})
	})

	it("gate 2: rejects a same-token order whose realized spread is zero", async () => {
		// The book is healthy (ask 0.999) but the ORDER demands a 1:1 payout —
		// the realized spread on this fill is zero, so the per-leg gate refuses.
		const filler = gateFiller(new AssetRegistry(cfg), { fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) })
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
		const filler = gateFiller(registry, { fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) })
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

	// Two arbitrary non-USD tokens, ZARP and CNGN, both resolved through [assets].
	const zarpCngnRegistry = () =>
		new AssetRegistry(cfg, { ZARP: { [SRC]: ZARP, [DST]: ZARP }, CNGN: { [SRC]: CNGN, [DST]: CNGN } })

	it("rejects an order when a leg demands more than its own ask curve yields", async () => {
		// The USDC/CNGN leg demands 1.6M CNGN but the 1450 ask pays at most
		// 1.45M for 1000 USDC. Each leg is checked against its own curve only —
		// the comfortably-within-curve ZARP/CNGN leg cannot rescue the order.
		const filler = gateFiller(zarpCngnRegistry(), { fillGas: parseUnits("1", 6), relayer: parseUnits("1", 6) })
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
})
