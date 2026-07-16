import "log-timestamp"

import { strict as assert } from "node:assert"
import type { PublicClient } from "viem"
import type { HexString, Order, TokenInfo } from "@/types"
import { EvmChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { createQueryClient } from "@/queryClient"
import {
	deductProtocolFee,
	grossUpForProtocolFee,
	UNISWAP_INTENT_QUOTE_CHAIN,
} from "@/protocols/intents/quote/uniswapV4"
import { PhantomSnapshotIntentQuoteStrategy } from "@/protocols/intents/quote"
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

	it("quotes USDT through the Base USDC Phantom snapshot", async () => {
		const configService = new ChainConfigService()
		console.log("USDT -> cNGN intent quote through Base USDC snapshot")

		const baseChain = makeEvmChain(CHAINS.base, configService)
		const cNgnAddress = configService.getCNgnAsset(BASE_CHAIN)
		assert(cNgnAddress)
		const queryClient = createQueryClient({ url: "https://nexus.indexer.polytope.technology/" })
		const intentGateway = (await IntentGateway.create(baseChain, baseChain)).withQueryClient(queryClient)

		const quote = await intentGateway.quoteIntent({
			tokenIn: configService.getUsdtAsset(BASE_CHAIN),
			tokenOut: cNgnAddress,
			amountIn: 1_000_000n,
		})

		console.log("amountIn:", quote.amountIn.toString())
		console.log("amountOut:", quote.amountOut.toString())
		console.log("protocolFeeBps:", quote.quoteMetadata.protocolFeeBps.toString())
		assert.equal(quote.strategy, "phantom_snapshot")
		if (quote.strategy !== "phantom_snapshot") throw new Error("Expected Phantom snapshot quote")
		console.log("snapshot commitment:", quote.quoteMetadata.commitment)
		console.log("snapshot block:", quote.quoteMetadata.blockNumber.toString())

		assert.equal(quote.tradeType, "EXACT_INPUT")
		assert.equal(quote.amountIn, 1_000_000n)
		assert(quote.amountOut > 0n)
		assert.equal(quote.quoteMetadata.quoteChain, BASE_CHAIN)
		assert(quote.quoteMetadata.medianPrice > 0n)
		assert(quote.quoteMetadata.bidCount > 0)

		const bscUsdtQuote = await new PhantomSnapshotIntentQuoteStrategy(configService, () => queryClient).quote(
			{
				tokenIn: configService.getUsdtAsset(CHAINS.bsc.id),
				tokenOut: cNgnAddress,
				amountIn: 1_000_000n,
			},
			{
				stateMachineId: CHAINS.bsc.id,
				client: {
					readContract: async () => [0n, 0n, 0n, 0n, 5n],
				} as unknown as PublicClient,
			},
			{ stateMachineId: BASE_CHAIN, client: baseChain.client },
		)
		assert.equal(bscUsdtQuote.strategy, "phantom_snapshot")
		if (bscUsdtQuote.strategy !== "phantom_snapshot") throw new Error("Expected Phantom snapshot quote")
		assert.equal(bscUsdtQuote.quoteMetadata.tokenA, configService.getUsdcAsset(BASE_CHAIN))
		assert.equal(bscUsdtQuote.quoteMetadata.tokenB.toLowerCase(), cNgnAddress.toLowerCase())
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
