import "log-timestamp"

import { strict as assert } from "node:assert"
import { decodeFunctionData, formatUnits, parseUnits, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { AvailableLiquidity, BuyAndSellRates, HexString, Order, TokenInfo } from "@/types"
import { EvmChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { LiquidityEngine } from "@/protocols/intents/LiquidityEngine"
import { DEFAULT_GRAFFITI } from "@/protocols/intents/types"
import { createQueryClient } from "@/queryClient"
import {
	deductProtocolFee,
	grossUpForProtocolFee,
	UNISWAP_INTENT_QUOTE_CHAIN,
} from "@/protocols/intents/quote/uniswapV4"
import { divCeil } from "@/protocols/intents/quote/shared"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { bytes20ToBytes32 } from "@/utils"
import { UniswapQuoteEngine, type UniswapQuoteAdapter, type UniswapQuoteToken } from "@/utils/uniswapQuote"

// ---------------------------------------------------------------------------
// Test Cases
// ---------------------------------------------------------------------------

// Skipped: IntentGateway contracts not redeployed in the testnet redeployment.
describe.skip("IntentGateway cross-chain estimate tests", () => {
	for (const [src, dest] of CROSS_CHAIN_CASES) {
		it(`Should estimate fee for ${src} => ${dest}`, async () => {
			await runCrossChainEstimate(src, dest)
		}, 1_000_000)
	}
})

describe.skip("IntentGateway BSC => Base cross-chain estimate (simplex repro)", () => {
	it("estimates fillOrder without falling back to default gas values", async () => {
		await runCrossChainEstimate("bsc", "base")
	}, 300_000)
})

// Skipped: IntentGateway contracts not redeployed in the testnet redeployment.
describe.skip("IntentGateway same-chain estimate tests", () => {
	for (const chain of SAME_CHAIN_CASES) {
		it(`Should estimate fee for ${chain} same-chain USDC => EXT`, async () => {
			await runSameChainEstimate(chain)
		}, 1_000_000)
	}
})

describe.skip("Uniswap quote helper", () => {
	it("returns the best exact-input quote across selected protocols", async () => {
		const client = { name: "intent-gateway-quote-test-client" } as unknown as PublicClient
		const quoteEngine = new UniswapQuoteEngine(new QuoteTestAdapter(client))

		const result = await quoteEngine.quote(
			{
				chainId: 8453,
				tokenIn: QUOTE_TOKEN_IN,
				tokenOut: QUOTE_TOKEN_OUT,
				amountIn: 100n,
				tradeType: "EXACT_INPUT",
				protocols: ["v2", "v3", "v4"],
			},
			{ client },
		)

		assert.equal(result.quotes.length, 3)
		assert.equal(result.bestQuote?.protocol, "v4")
		assert.equal(result.bestQuote?.amountOut, 103n)
	})
})

describe("Intent quote helper", () => {
	const BASE_CHAIN = "EVM-8453"

	it.skip("applies the gateway protocol fee to quoted amounts", () => {
		assert.equal(UNISWAP_INTENT_QUOTE_CHAIN, BASE_CHAIN)
		// 30 bps fee: exact-input nets less to the swap, exact-output grosses up.
		assert.equal(deductProtocolFee(1_000_000n, 30n), 997_000n)
		assert.equal(grossUpForProtocolFee(997_000n, 30n), 1_000_000n)
		// Gross-up rounds up so the post-fee net never falls short.
		assert.equal(grossUpForProtocolFee(1n, 30n), 2n)
		// Zero fee is a no-op in both directions.
		assert.equal(deductProtocolFee(1_000_000n, 0n), 1_000_000n)
		assert.equal(grossUpForProtocolFee(1_000_000n, 0n), 1_000_000n)
	})

	it("queries live USDC to cNGN liquidity", async () => {
		const configService = new ChainConfigService()
		const cNgnAddress = configService.getCNgnAsset(BASE_CHAIN)
		assert(cNgnAddress, "Expected cNGN to be configured on Base")

		const intentGateway = await createLiveBaseIntentGateway(configService)
		const liquidity = await intentGateway.queryAvailableLiquidity({
			tokenIn: configService.getUsdcAsset(BASE_CHAIN),
			tokenOut: cNgnAddress,
		})

		assertAndLogLiquidity("USDC → cNGN", liquidity, cNgnAddress)
	}, 120_000)

	it("queries live cNGN to USDC liquidity", async () => {
		const configService = new ChainConfigService()
		const cNgnAddress = configService.getCNgnAsset(BASE_CHAIN)
		const usdcAddress = configService.getUsdcAsset(BASE_CHAIN)
		assert(cNgnAddress, "Expected cNGN to be configured on Base")

		const intentGateway = await createLiveBaseIntentGateway(configService)
		const liquidity = await intentGateway.queryAvailableLiquidity({ tokenIn: cNgnAddress, tokenOut: usdcAddress })

		assertAndLogLiquidity("cNGN → USDC", liquidity, usdcAddress)
	}, 120_000)

	it("queries the indexed Base USDT to cNGN pool directly", async () => {
		const configService = new ChainConfigService()
		const cNgnAddress = configService.getCNgnAsset(BASE_CHAIN)
		assert(cNgnAddress, "Expected cNGN to be configured on Base")

		const intentGateway = await createLiveBaseIntentGateway(configService)
		const liquidity = await intentGateway.queryAvailableLiquidity({
			tokenIn: configService.getUsdtAsset(BASE_CHAIN),
			tokenOut: cNgnAddress,
		})

		logLiquidity("USDT → cNGN", liquidity)
		assert(liquidity, "Expected an indexed Base USDT → cNGN pool sample")
		assert.equal(liquidity.tokenAddress, cNgnAddress.toLowerCase())
		assert(!Number.isNaN(liquidity.updatedAt.getTime()))
		assert(parseUnits(liquidity.destination.totalLiquidity, 18) >= 0n)
		assert(liquidity.destination.providerCount >= 0)
	}, 120_000)

	it("reports destination, unrestricted, and explicit cross-chain liquidity separately", async () => {
		const configService = new ChainConfigService()
		const cNgnAddress = configService.getCNgnAsset(BASE_CHAIN)
		assert(cNgnAddress, "Expected cNGN to be configured on Base")
		const sourceUsdc = configService.getUsdcAsset(CHAINS.eth.id)
		const sourceToken = configService.getAssetMetadataByAddress(CHAINS.eth.id, sourceUsdc)
		const destinationToken = configService.getAssetMetadataByAddress(BASE_CHAIN, cNgnAddress)
		assert(sourceToken, "Expected Ethereum USDC metadata")
		assert(destinationToken, "Expected Base cNGN metadata")

		const engine = new LiquidityEngine(
			createQueryClient({ url: "https://nexus.indexer.polytope.technology/" }),
		)
		const liquidity = await engine.getAvailableLiquidity({
			source: { chain: CHAINS.eth.id, ...sourceToken },
			destination: { chain: BASE_CHAIN, ...destinationToken },
		})

		logLiquidity("Ethereum USDC → Base cNGN", liquidity)
		assert(liquidity, "Expected indexed Base destination liquidity")
		assert.equal(liquidity.sourceChain, CHAINS.eth.id)
		assert.equal(liquidity.destinationChain, BASE_CHAIN)
		assert.equal(liquidity.tokenAddress, cNgnAddress.toLowerCase())
		assert(parseUnits(liquidity.destination.totalLiquidity, 18) > 0n)
		assert(
			parseUnits(liquidity.unrestricted.totalLiquidity, 18) <=
				parseUnits(liquidity.destination.totalLiquidity, 18),
		)
		assert(liquidity.unrestricted.providerCount <= liquidity.destination.providerCount)
		if (liquidity.explicitRoute) {
			assert(
				parseUnits(liquidity.explicitRoute.totalLiquidity, 18) <=
					parseUnits(liquidity.destination.totalLiquidity, 18),
			)
			assert(liquidity.explicitRoute.providerCount <= liquidity.destination.providerCount)
		}
	}, 120_000)

	it("queries buy and sell rates using only symbols and chain IDs", async () => {
		const configService = new ChainConfigService()
		const intentGateway = await createLiveBaseIntentGateway(configService)
		const rates = await intentGateway.queryBuyAndSellRates({
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cngn",
			sourceChainId: 1,
			destinationChainId: 8453,
		})

		logRates("Ethereum USDC → Base cNGN", rates)
		assert(rates, "Expected live chain-specific cNGN/USDC rates from Nexus")
		assert.equal(rates.baseTokenSymbol, "USDC")
		assert.equal(rates.quoteTokenSymbol, "cNGN")
		assert(rates.buyRate && parseUnits(rates.buyRate, 18) > 1_000n * 10n ** 18n)
		assert(rates.sellRate && parseUnits(rates.sellRate, 18) > 1_000n * 10n ** 18n)
		assert(rates.buyRateUpdatedAt && !Number.isNaN(rates.buyRateUpdatedAt.getTime()))
		assert(rates.sellRateUpdatedAt && !Number.isNaN(rates.sellRateUpdatedAt.getTime()))
	}, 120_000)

	it("quotes exact-input BSC USDC to Base cNGN from indexed rates", async () => {
		const configService = new ChainConfigService()
		const cNgnAddress = configService.getCNgnAsset(CHAINS.base.id)
		const cNgnDecimals = configService.getCNgnDecimals(CHAINS.base.id)
		assert(cNgnAddress, "Expected cNGN to be configured on Base")
		assert(cNgnDecimals !== undefined, "Expected cNGN decimals to be configured on Base")
		const intentGateway = await createLiveIntentGateway(CHAINS.bsc, CHAINS.base, configService)
		const usdcDecimals = configService.getUsdcDecimals(CHAINS.bsc.id)
		const amountIn = parseUnits("100", usdcDecimals)

		const quote = await intentGateway.quoteIntent({
			tokenIn: configService.getUsdcAsset(CHAINS.bsc.id),
			tokenOut: cNgnAddress,
			amountIn,
		})

		logIntentQuote("BSC USDC → Base cNGN exact input", quote)
		assert.equal(quote.strategy, "indexed_rates")
		if (quote.strategy !== "indexed_rates") throw new Error("Expected indexed rate quote")
		const scaledRate = parseUnits(quote.quoteMetadata.rate, 18)
		const netAmountIn = deductProtocolFee(amountIn, quote.quoteMetadata.protocolFeeBps)
		const expectedAmountOut =
			(netAmountIn * scaledRate * 10n ** BigInt(cNgnDecimals)) /
			(10n ** BigInt(usdcDecimals) * 10n ** 18n)
		assert.equal(quote.tradeType, "EXACT_INPUT")
		assert.equal(quote.amountIn, amountIn)
		assert.equal(quote.amountOut, expectedAmountOut)
		assert(netAmountIn < amountIn, "Expected the on-chain protocol fee to reduce the priced input")
		assert(quote.amountOut > parseUnits("100000", cNgnDecimals))
		assert.equal(quote.quoteMetadata.sourceChain, CHAINS.bsc.id)
		assert.equal(quote.quoteMetadata.destinationChain, CHAINS.base.id)
		assert.equal(quote.quoteMetadata.rateSide, "buy")
		assert(scaledRate > 1_000n * 10n ** 18n)
		assert(!Number.isNaN(quote.quoteMetadata.rateUpdatedAt.getTime()))
		console.log("Fee-adjusted buy quote:", {
			amountIn: `${formatUnits(quote.amountIn, usdcDecimals)} USDC`,
			amountOut: `${formatUnits(quote.amountOut, cNgnDecimals)} cNGN`,
		})
	}, 120_000)

	it("quotes exact-output Base cNGN to BSC USDC from indexed rates", async () => {
		const configService = new ChainConfigService()
		const cNgnAddress = configService.getCNgnAsset(CHAINS.base.id)
		const cNgnDecimals = configService.getCNgnDecimals(CHAINS.base.id)
		assert(cNgnAddress, "Expected cNGN to be configured on Base")
		assert(cNgnDecimals !== undefined, "Expected cNGN decimals to be configured on Base")
		const intentGateway = await createLiveIntentGateway(CHAINS.base, CHAINS.bsc, configService)
		const usdcDecimals = configService.getUsdcDecimals(CHAINS.bsc.id)
		const amountOut = parseUnits("100", usdcDecimals)

		const quote = await intentGateway.quoteIntent({
			tokenIn: cNgnAddress,
			tokenOut: configService.getUsdcAsset(CHAINS.bsc.id),
			amountOut,
		})

		logIntentQuote("Base cNGN → BSC USDC exact output", quote)
		assert.equal(quote.strategy, "indexed_rates")
		if (quote.strategy !== "indexed_rates") throw new Error("Expected indexed rate quote")
		const scaledRate = parseUnits(quote.quoteMetadata.rate, 18)
		const requiredNetAmountIn = divCeil(
			amountOut * 10n ** BigInt(cNgnDecimals) * scaledRate,
			10n ** BigInt(usdcDecimals) * 10n ** 18n,
		)
		const expectedAmountIn = grossUpForProtocolFee(
			requiredNetAmountIn,
			quote.quoteMetadata.protocolFeeBps,
		)
		assert.equal(quote.tradeType, "EXACT_OUTPUT")
		assert.equal(quote.amountOut, amountOut)
		assert.equal(quote.amountIn, expectedAmountIn)
		assert(
			deductProtocolFee(quote.amountIn, quote.quoteMetadata.protocolFeeBps) >= requiredNetAmountIn,
			"Expected the gross input to cover the required net input after the on-chain protocol fee",
		)
		assert(quote.amountIn > parseUnits("100000", cNgnDecimals))
		assert.equal(quote.quoteMetadata.sourceChain, CHAINS.base.id)
		assert.equal(quote.quoteMetadata.destinationChain, CHAINS.bsc.id)
		assert.equal(quote.quoteMetadata.rateSide, "sell")
		assert(scaledRate > 1_000n * 10n ** 18n)
		assert(!Number.isNaN(quote.quoteMetadata.rateUpdatedAt.getTime()))
		console.log("Fee-adjusted sell quote:", {
			amountIn: `${formatUnits(quote.amountIn, cNgnDecimals)} cNGN`,
			amountOut: `${formatUnits(quote.amountOut, usdcDecimals)} USDC`,
		})
	}, 120_000)
})

describe("IntentGateway placement fee metadata", () => {
	it("exposes the source fee token and exact encoded fee from execute and executeBest", async () => {
		const configService = new ChainConfigService()
		const baseChain = makeEvmChain(CHAINS.base, configService)
		const intentGateway = await IntentGateway.create(baseChain, baseChain)
		const feeToken = await baseChain.getFeeTokenWithDecimals()
		const order = buildOrder(
			CHAINS.base.id,
			CHAINS.base.id,
			configService.getUsdcAsset(CHAINS.base.id),
			configService.getExtAsset(CHAINS.base.id)!,
			1_000_000n,
		)
		order.fees = 1n

		console.log("IntentGateway placement fee metadata test", {
			sourceChain: CHAINS.base.id,
			gateway: configService.getIntentGatewayAddress(CHAINS.base.id),
			feeToken: feeToken.address,
			inputToken: configService.getUsdcAsset(CHAINS.base.id),
			outputToken: configService.getExtAsset(CHAINS.base.id),
			orderFee: order.fees.toString(),
		})

		for (const { method, generator } of [
			{
				method: "execute",
				generator: intentGateway.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 1 }),
			},
			{
				method: "executeBest",
				generator: intentGateway.executeBest(order, DEFAULT_GRAFFITI, { auctionTimeMs: 1 }),
			},
		]) {
			const result = await generator.next()
			assert(!result.done, "Expected the first update to prepare placement")
			assert.equal(result.value.status, "AWAITING_PLACE_ORDER")
			if (result.value.status !== "AWAITING_PLACE_ORDER") throw new Error("Expected placement update")

			assert.equal(
				result.value.to.toLowerCase(),
				configService.getIntentGatewayAddress(CHAINS.base.id).toLowerCase(),
			)
			assert.equal(result.value.feeTokenAddress.toLowerCase(), feeToken.address.toLowerCase())
			assert.equal(result.value.feeTokenAmount, order.fees)
			assert("value" in result.value)
			assert.match(result.value.sessionPrivateKey, /^0x[\da-f]{64}$/i)

			const decoded = decodeFunctionData({ abi: IntentGatewayV2ABI, data: result.value.data })
			assert.equal(decoded.functionName, "placeOrder")
			assert.equal((decoded.args?.[0] as { fees: bigint }).fees, result.value.feeTokenAmount)

			console.log(`${method} AWAITING_PLACE_ORDER`, {
				to: result.value.to,
				value: result.value.value.toString(),
				nativeFee: result.value.nativeFee.toString(),
				feeTokenAddress: result.value.feeTokenAddress,
				feeTokenAmount: result.value.feeTokenAmount.toString(),
				encodedFeeTokenAmount: (decoded.args?.[0] as { fees: bigint }).fees.toString(),
			})
		}
	}, 120_000)

	it("includes native value when execute estimates a zero-fee order", async () => {
		const configService = new ChainConfigService()
		const baseChain = makeEvmChain(CHAINS.base, configService)
		const intentGateway = await IntentGateway.create(baseChain, baseChain)
		const order = buildOrder(
			CHAINS.base.id,
			CHAINS.base.id,
			configService.getUsdcAsset(CHAINS.base.id),
			configService.getExtAsset(CHAINS.base.id)!,
			1_000_000n,
		)

		const result = await intentGateway.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 1 }).next()
		assert(!result.done, "Expected the first update to prepare placement")
		assert.equal(result.value.status, "AWAITING_PLACE_ORDER")
		if (result.value.status !== "AWAITING_PLACE_ORDER") throw new Error("Expected placement update")

		// value carries only native-token inputs; this order's input is USDC.
		assert.equal(result.value.value, 0n, "Expected no native input value for an ERC-20 order")
		assert(result.value.nativeFee > 0n, "Expected a positive native fee for a zero-fee order")
		assert(result.value.feeTokenAmount > 0n, "Expected a positive estimated fee-token amount")

		const decoded = decodeFunctionData({ abi: IntentGatewayV2ABI, data: result.value.data })
		assert.equal(decoded.functionName, "placeOrder")
		assert.equal((decoded.args?.[0] as { fees: bigint }).fees, result.value.feeTokenAmount)

		console.log("execute native-fee AWAITING_PLACE_ORDER", {
			to: result.value.to,
			value: result.value.value.toString(),
			nativeFee: result.value.nativeFee.toString(),
			feeTokenAddress: result.value.feeTokenAddress,
			feeTokenAmount: result.value.feeTokenAmount.toString(),
			encodedFeeTokenAmount: (decoded.args?.[0] as { fees: bigint }).fees.toString(),
		})
	}, 120_000)
})

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const CROSS_CHAIN_CASES: [string, string][] = [
	["bsc", "eth"],
	["bsc", "arbitrum"],
	["base", "bsc"],
	["bsc", "polygon"],
]

const SAME_CHAIN_CASES = ["polygon", "bsc", "base", "arbitrum"]

const BENEFICIARY = "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString
const QUOTE_TOKEN_IN: UniswapQuoteToken = {
	address: "0x1111111111111111111111111111111111111111",
	decimals: 6,
	symbol: "USDC",
	chainId: 8453,
}
const QUOTE_TOKEN_OUT: UniswapQuoteToken = {
	address: "0x2222222222222222222222222222222222222222",
	decimals: 6,
	symbol: "cNGN",
	chainId: 8453,
}

class QuoteTestAdapter implements UniswapQuoteAdapter {
	constructor(private readonly expectedClient: PublicClient) {}

	async findBestProtocolWithAmountIn(
		client: PublicClient,
		_tokenIn: HexString,
		_tokenOut: HexString,
		_amountIn: bigint,
		_evmChainID: string,
		options?: { selectedProtocol?: "v2" | "v3" | "v4"; generateCalldata?: boolean; recipient?: HexString },
	) {
		assert.equal(client, this.expectedClient)

		switch (options?.selectedProtocol) {
			case "v2":
				return { protocol: "v2" as const, amountOut: 95n }
			case "v3":
				return { protocol: "v3" as const, amountOut: 101n, fee: 500 }
			case "v4":
				return { protocol: "v4" as const, amountOut: 103n, fee: 1500 }
			default:
				return { protocol: null, amountOut: 0n }
		}
	}

	async findBestProtocolWithAmountOut(): Promise<never> {
		throw new Error("Unused by exact-input quote test")
	}

	createV2SwapCalldataExactIn(): never {
		throw new Error("Unused without recipient")
	}

	createV2SwapCalldataExactOut(): never {
		throw new Error("Unused by exact-input quote test")
	}

	createV3SwapCalldataExactIn(): never {
		throw new Error("Unused without recipient")
	}

	createV3SwapCalldataExactOut(): never {
		throw new Error("Unused by exact-input quote test")
	}

	createV4SwapCalldataExactIn(): never {
		throw new Error("Unused without recipient")
	}

	createV4SwapCalldataExactOut(): never {
		throw new Error("Unused by exact-input quote test")
	}
}

interface ChainDef {
	id: string
	numericId: number
	rpcEnvVar: string
}

const CHAINS: Record<string, ChainDef> = {
	eth: { id: "EVM-1", numericId: 1, rpcEnvVar: "ETH_MAINNET" },
	bsc: { id: "EVM-56", numericId: 56, rpcEnvVar: "BSC_MAINNET" },
	polygon: { id: "EVM-137", numericId: 137, rpcEnvVar: "POLYGON_MAINNET" },
	base: { id: "EVM-8453", numericId: 8453, rpcEnvVar: "BASE_MAINNET" },
	arbitrum: { id: "EVM-42161", numericId: 42161, rpcEnvVar: "ARBITRUM_MAINNET" },
}

function bundlerUrl(chainId: number): string | undefined {
	let apiKey = process.env.BUNDLER_API_KEY
	if (!apiKey && process.env.BUNDLER_URL) {
		try {
			const url = new URL(process.env.BUNDLER_URL)
			apiKey = url.searchParams.get("apikey") ?? url.searchParams.get("apiKey") ?? undefined
		} catch {}
	}
	return apiKey ? `https://api.pimlico.io/v2/${chainId}/rpc?apikey=${apiKey}` : undefined
}

function makeEvmChain(chain: ChainDef, configService: ChainConfigService, bundlerUrl?: string): EvmChain {
	return EvmChain.fromParams({
		chainId: chain.numericId,
		host: configService.getHostAddress(chain.id),
		rpcUrl: process.env[chain.rpcEnvVar] ?? configService.getRpcUrl(chain.id),
		bundlerUrl,
	})
}

async function createLiveBaseIntentGateway(configService: ChainConfigService): Promise<IntentGateway> {
	return createLiveIntentGateway(CHAINS.base, CHAINS.base, configService)
}

async function createLiveIntentGateway(
	source: ChainDef,
	destination: ChainDef,
	configService: ChainConfigService,
): Promise<IntentGateway> {
	const gateway = await IntentGateway.create(
		makeEvmChain(source, configService),
		makeEvmChain(destination, configService),
	)
	return gateway.withQueryClient(createQueryClient({ url: "https://nexus.indexer.polytope.technology/" }))
}

function logIntentQuote(label: string, quote: Awaited<ReturnType<IntentGateway["quoteIntent"]>>): void {
	console.log(`[quoteIntent] ${label}`)
	console.log({
		...quote,
		amountIn: quote.amountIn.toString(),
		amountOut: quote.amountOut.toString(),
		quoteMetadata: {
			...quote.quoteMetadata,
			protocolFeeBps: quote.quoteMetadata.protocolFeeBps.toString(),
			...(quote.strategy === "indexed_rates"
				? { rateUpdatedAt: quote.quoteMetadata.rateUpdatedAt.toISOString() }
				: {}),
		},
	})
}

function logLiquidity(label: string, liquidity: AvailableLiquidity | undefined): void {
	console.log(`[queryAvailableLiquidity] ${label}`)
	console.log(
		liquidity
			? {
					...liquidity,
					updatedAt: liquidity.updatedAt.toISOString(),
					explicitRoute: liquidity.explicitRoute
						? { ...liquidity.explicitRoute, updatedAt: liquidity.explicitRoute.updatedAt.toISOString() }
						: null,
				}
			: undefined,
	)
}

function logRates(label: string, rates: BuyAndSellRates | undefined): void {
	console.log(`[queryBuyAndSellRates] ${label}`)
	console.log(
		rates
			? {
					...rates,
					buyRateUpdatedAt: rates.buyRateUpdatedAt?.toISOString() ?? null,
					sellRateUpdatedAt: rates.sellRateUpdatedAt?.toISOString() ?? null,
				}
			: undefined,
	)
}

function assertAndLogLiquidity(
	pair: string,
	liquidity: AvailableLiquidity | undefined,
	expectedTokenAddress: HexString,
): asserts liquidity is AvailableLiquidity {
	logLiquidity(pair, liquidity)
	assert(liquidity, `Expected live ${pair} indexed pool liquidity from Nexus`)
	assert.notEqual(liquidity.destination.totalLiquidity, "0")
	assert(liquidity.destination.providerCount > 0)
	assert.equal(liquidity.tokenAddress, expectedTokenAddress.toLowerCase())
	assert(!Number.isNaN(liquidity.updatedAt.getTime()))
	assert(
		parseUnits(liquidity.unrestricted.totalLiquidity, 18) <= parseUnits(liquidity.destination.totalLiquidity, 18),
	)
}

function buildOrder(
	sourceChainId: string,
	destChainId: string,
	inputToken: HexString,
	outputToken: HexString,
	amount: bigint,
): Order {
	const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(inputToken), amount }]
	const outputAssets: TokenInfo[] = [{ token: bytes20ToBytes32(outputToken), amount }]

	return {
		user: BENEFICIARY,
		source: sourceChainId,
		destination: destChainId,
		deadline: 65337297000n,
		nonce: 0n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs,
		output: { beneficiary: BENEFICIARY, assets: outputAssets, call: "0x" as HexString },
	}
}

/**
 * Estimates a cross-chain fill and fails if GasEstimator fell back to its
 * default gas values. estimateFillOrder swallows simulation reverts and only
 * emits a console.warn, so the warning is the observable failure signal.
 */
async function runCrossChainEstimate(srcKey: string, destKey: string) {
	const src = CHAINS[srcKey]
	const dest = CHAINS[destKey]
	const configService = new ChainConfigService()

	const srcChain = makeEvmChain(src, configService)
	const destChain = makeEvmChain(dest, configService, bundlerUrl(dest.numericId))

	const intentGateway = await IntentGateway.create(srcChain, destChain)

	const order = buildOrder(
		src.id,
		dest.id,
		configService.getUsdcAsset(src.id),
		configService.getUsdcAsset(dest.id),
		100n,
	)

	const warnings: string[] = []
	const originalWarn = console.warn
	console.warn = (...args: unknown[]) => {
		warnings.push(args.map((a) => (a instanceof Error ? a.message : String(a))).join(" "))
		originalWarn(...args)
	}

	let estimate
	try {
		estimate = await intentGateway.estimateFillOrder({ order })
	} finally {
		console.warn = originalWarn
	}

	console.log(`${srcKey} => ${destKey}`)
	console.log("callGasLimit:", estimate.callGasLimit)
	console.log("relayerFee:", estimate.fillOptions.relayerFee)
	console.log("nativeDispatchFee:", estimate.fillOptions.nativeDispatchFee)
	console.log("Estimated cost (totalGasCostWei):", estimate.totalGasCostWei)
	console.log("Estimated fee (totalGasInFeeToken):", estimate.totalGasInFeeToken)

	const fallbackWarning = warnings.find((w) => w.includes("gas estimation failed"))
	assert.equal(fallbackWarning, undefined, `estimateFillOrder fell back to default gas values: ${fallbackWarning}`)
	assert(estimate.totalGasCostWei > 0n)
	assert(estimate.totalGasInFeeToken > 0n)
}

async function runSameChainEstimate(chainKey: string) {
	const chain = CHAINS[chainKey]
	const configService = new ChainConfigService()
	const evmChain = makeEvmChain(chain, configService, bundlerUrl(chain.numericId))

	const intentGateway = await IntentGateway.create(evmChain, evmChain)

	const order = buildOrder(
		chain.id,
		chain.id,
		configService.getUsdcAsset(chain.id),
		configService.getExtAsset(chain.id)!,
		100n,
	)

	const estimate = await intentGateway.estimateFillOrder({ order })

	console.log(`${chainKey} same-chain estimated cost (totalGasCostWei):`, estimate.totalGasCostWei)
	console.log(`${chainKey} same-chain USDC => EXT, estimated fee:`, estimate.totalGasInFeeToken)
	assert(estimate.totalGasCostWei > 0n)
	assert(estimate.totalGasInFeeToken > 0n)
}
