import "log-timestamp"

import { strict as assert } from "node:assert"
import { vi } from "vitest"
import { decodeFunctionData, parseUnits, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { FillOrderEstimate, HexString, Order, TokenInfo } from "@/types"
import { EvmChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { DEFAULT_GRAFFITI } from "@/protocols/intents/types"
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

// Reads the testnet orderbook the gateway picks for BSC Chapel and Polygon Amoy; HYPERFX_ORDERBOOK_URL
// points it elsewhere. The book is live and solvers come and go, so each test checks whichever state it
// finds: prices and amounts while orders serve the route, a consistent "nothing fillable" while none do. BSC_CHAPEL and POLYGON_AMOY override the chains' public RPCs, which rate-limit.
describe("IntentGateway orderbook reads", () => {
	it("queries the best bid and ask using only symbols and chain IDs", async () => {
		const configService = new ChainConfigService()
		const intentGateway = await createLiveIntentGateway(CHAINS.amoy, CHAINS.chapel, configService)
		const rates = await intentGateway.queryBuyAndSellRates({
			tokenInSymbol: "USDC",
			tokenOutSymbol: "cngn",
			sourceChainId: CHAINS.amoy.numericId,
			destinationChainId: CHAINS.chapel.numericId,
		})

		console.log("[queryBuyAndSellRates] Amoy USDC → Chapel cNGN", rates)
		assert.equal(rates.baseTokenSymbol, "USDC")
		assert.equal(rates.quoteTokenSymbol, "cNGN")
		if (rates.bid === null || rates.ask === null) {
			assert.equal(rates.mid, null)
			assert.equal(rates.spreadBps, null)
		}
		if (rates.bid && rates.ask && rates.spread) {
			assert.equal(parseUnits(rates.ask, 18) - parseUnits(rates.bid, 18), parseUnits(rates.spread, 18))
		}
	}, 120_000)

	it("queries route liquidity in both directions", async () => {
		const configService = new ChainConfigService()
		const chapelCngn = configService.getCNgnAsset(CHAINS.chapel.id)
		assert(chapelCngn, "Expected cNGN to be configured on BSC Chapel")

		const crossChain = await createLiveIntentGateway(CHAINS.amoy, CHAINS.chapel, configService)
		const sell = await crossChain.queryAvailableLiquidity({
			tokenIn: configService.getUsdcAsset(CHAINS.amoy.id),
			tokenOut: chapelCngn,
		})
		console.log("[queryAvailableLiquidity] Amoy USDC → Chapel cNGN", sell)
		assert.equal(sell.route, "CROSS_CHAIN")
		assert.equal(sell.side, "BID")
		assert.equal(sell.tokenAddress, chapelCngn)
		assert(parseUnits(sell.availableLiquidity, 18) <= parseUnits(sell.depthOut, 18))

		const sameChain = await createLiveIntentGateway(CHAINS.chapel, CHAINS.chapel, configService)
		const buy = await sameChain.queryAvailableLiquidity({
			tokenIn: chapelCngn,
			tokenOut: configService.getUsdcAsset(CHAINS.chapel.id),
		})
		console.log("[queryAvailableLiquidity] Chapel cNGN → Chapel USDC", buy)
		assert.equal(buy.route, "SAME_CHAIN")
		assert.equal(buy.side, "ASK")
	}, 120_000)

	it("quotes exact-input Amoy USDC to Chapel cNGN as one leg per order", async () => {
		const configService = new ChainConfigService()
		const chapelCngn = configService.getCNgnAsset(CHAINS.chapel.id)
		const cNgnDecimals = configService.getCNgnDecimals(CHAINS.chapel.id)
		assert(chapelCngn && cNgnDecimals !== undefined, "Expected cNGN to be configured on BSC Chapel")
		const usdcDecimals = configService.getUsdcDecimals(CHAINS.amoy.id)
		const intentGateway = await createLiveIntentGateway(CHAINS.amoy, CHAINS.chapel, configService)
		const amountIn = parseUnits("10", usdcDecimals)

		const route = { tokenIn: configService.getUsdcAsset(CHAINS.amoy.id), tokenOut: chapelCngn }
		const liquidity = await intentGateway.queryAvailableLiquidity(route)
		const fillableIn = parseUnits(liquidity.maxFillableIn, usdcDecimals)
		const quote = await intentGateway.quoteIntent({ ...route, amountIn, optimistic: true })

		logIntentQuote("Amoy USDC → Chapel cNGN exact input", quote)
		assert.equal(quote.amountIn, amountIn)
		if (fillableIn < amountIn) {
			// Too little depth: the orderbook's unfillable quote, not an error.
			assert.equal(quote.fillable, false)
			assert.equal(quote.maxFillableIn, fillableIn)
			assert.deepEqual(quote.legs, [])
			return
		}
		assert(quote.fillable)
		assert.equal(quote.side, "BID")
		assert.equal(quote.route, "CROSS_CHAIN")
		assert(quote.legs.length > 0)
		assert.equal(
			quote.legs.reduce((total, leg) => total + leg.amountIn, 0n),
			amountIn,
		)
		// Each leg fills at its own price, less the protocol fee, floored to a raw cNGN unit.
		for (const leg of quote.legs) {
			const grossOut =
				(leg.amountIn * leg.orderRate * 10n ** BigInt(cNgnDecimals)) /
				(10n ** BigInt(usdcDecimals) * 10n ** 18n)
			assert(leg.amountOut <= grossOut)
		}
	}, 120_000)

	it("quotes exact-input Amoy USDC to Chapel cNGN pessimistically at one price", async () => {
		const configService = new ChainConfigService()
		const chapelCngn = configService.getCNgnAsset(CHAINS.chapel.id)
		assert(chapelCngn, "Expected cNGN to be configured on BSC Chapel")
		const usdcDecimals = configService.getUsdcDecimals(CHAINS.amoy.id)
		const intentGateway = await createLiveIntentGateway(CHAINS.amoy, CHAINS.chapel, configService)
		const amountIn = parseUnits("10", usdcDecimals)

		const route = { tokenIn: configService.getUsdcAsset(CHAINS.amoy.id), tokenOut: chapelCngn }
		const quote = await intentGateway.quoteIntent({ ...route, amountIn })

		logIntentQuote("Amoy USDC → Chapel cNGN pessimistic exact input", quote)
		assert.equal(quote.amountIn, amountIn)
		assert.equal(quote.side, "BID")
		if (!quote.fillable) {
			// Too little depth: the orderbook's unfillable quote, not an error.
			assert(quote.rate === null && quote.amountOut === 0n && quote.maxFillableIn < amountIn)
			return
		}
		assert(quote.rate !== null && quote.amountOut > 0n)
		// One price for the whole trade, no better than the optimistic quote's worst leg.
		const optimistic = await intentGateway.quoteIntent({ ...route, amountIn, optimistic: true })
		assert(quote.rate <= (optimistic.legs.at(-1)?.orderRate ?? 0n))
	}, 120_000)

	it("quotes exact-output Amoy cNGN to Chapel USDC", async () => {
		const configService = new ChainConfigService()
		const amoyCngn = configService.getCNgnAsset(CHAINS.amoy.id)
		assert(amoyCngn, "Expected cNGN to be configured on Polygon Amoy")
		const usdcDecimals = configService.getUsdcDecimals(CHAINS.chapel.id)
		const intentGateway = await createLiveIntentGateway(CHAINS.amoy, CHAINS.chapel, configService)
		const amountOut = parseUnits("5", usdcDecimals)

		const route = { tokenIn: amoyCngn, tokenOut: configService.getUsdcAsset(CHAINS.chapel.id) }
		const liquidity = await intentGateway.queryAvailableLiquidity(route)
		const quote = await intentGateway.quoteIntent({ ...route, amountOut, optimistic: true })

		logIntentQuote("Amoy cNGN → Chapel USDC exact output", quote)
		if (!quote.fillable) {
			// Too little depth for the output, or none: the unfillable quote agrees with the route's liquidity.
			const cNgnDecimals = configService.getCNgnDecimals(CHAINS.amoy.id)!
			assert.equal(quote.maxFillableIn, parseUnits(liquidity.maxFillableIn, cNgnDecimals))
			assert.deepEqual(quote.legs, [])
			return
		}
		assert(quote.legs.reduce((total, leg) => total + leg.amountOut, 0n) >= amountOut)
		assert.equal(quote.side, "ASK")
		assert(quote.amountIn > 0n && quote.amountIn <= quote.maxFillableIn)
	}, 120_000)
})

// Skipped: these run against the live Base IntentGateway (0xAe041F7B…), which still reports
// release 2. Re-enable once the mainnet gateways are upgraded to release 3.
describe.skip("IntentGateway placement fee metadata", () => {
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
		// What is under test is how `execute` turns a fill estimate into the placement's
		// fee and native value, not the estimate itself. The live Base gateway still
		// reports release 2, which the fill estimator refuses, so the estimate is fixed
		// here and everything after it (fee token, placeOrder encoding) stays live.
		const estimate: FillOrderEstimate = {
			fillOptions: { relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 0n, outputs: [], inputs: [] },
			inputs: [],
			callGasLimit: 500_000n,
			verificationGasLimit: 100_000n,
			preVerificationGas: 100_000n,
			paymasterVerificationGasLimit: 0n,
			paymasterPostOpGasLimit: 0n,
			maxFeePerGas: 1_000_000_000n,
			maxPriorityFeePerGas: 1_000_000n,
			totalGasCostWei: 700_000_000_000_000n,
			totalGasInFeeToken: 2_500n,
			relayerFeeInSourceFeeToken: 0n,
		}
		// biome-ignore lint/suspicious/noExplicitAny: the estimator is private; this pins its answer
		vi.spyOn((intentGateway as any).gasEstimator, "estimateFillOrder").mockResolvedValue(estimate)
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
		// A same-chain order is quoted at twice the fill gas, and the native value carries a 2% buffer.
		assert.equal(result.value.feeTokenAmount, estimate.totalGasInFeeToken * 2n)
		assert.equal(result.value.nativeFee, estimate.totalGasCostWei + (estimate.totalGasCostWei * 2n) / 100n)

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
	chapel: { id: "EVM-97", numericId: 97, rpcEnvVar: "BSC_CHAPEL" },
	amoy: { id: "EVM-80002", numericId: 80002, rpcEnvVar: "POLYGON_AMOY" },
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

async function createLiveIntentGateway(
	source: ChainDef,
	destination: ChainDef,
	configService: ChainConfigService,
): Promise<IntentGateway> {
	const gateway = await IntentGateway.create(
		makeEvmChain(source, configService),
		makeEvmChain(destination, configService),
	)
	return process.env.HYPERFX_ORDERBOOK_URL ? gateway.withOrderbook(process.env.HYPERFX_ORDERBOOK_URL) : gateway
}

function logIntentQuote(label: string, quote: object): void {
	console.log(`[quoteIntent] ${label}`)
	console.log(JSON.stringify(quote, (_key, value) => (typeof value === "bigint" ? value.toString() : value), 2))
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
