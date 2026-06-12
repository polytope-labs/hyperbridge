import "log-timestamp"

import { strict as assert } from "assert"
import type { PublicClient } from "viem"
import { type HexString, Order, TokenInfo } from "@/types"
import { EvmChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { ChainConfigService } from "@/configs/ChainConfigService"
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

// Skipped: IntentGateway contracts not redeployed in the testnet redeployment.
describe.skip("IntentGateway same-chain estimate tests", () => {
	for (const chain of SAME_CHAIN_CASES) {
		it(`Should estimate fee for ${chain} same-chain USDC => EXT`, async () => {
			await runSameChainEstimate(chain)
		}, 1_000_000)
	}
})

describe("Uniswap quote helper", () => {
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

	it("quotes 1 USDC to cNGN on Base through Uniswap V4", async () => {
		const configService = new ChainConfigService()
		console.log("Base USDC -> cNGN intent quote")

		const baseChain = makeEvmChain(CHAINS.base, configService)
		const intentGateway = await IntentGateway.create(baseChain, baseChain)

		const quote = await intentGateway.quoteIntent({
			tokenIn: {
				address: configService.getUsdcAsset(BASE_CHAIN),
				decimals: 6,
				symbol: "USDC",
			},
			tokenOut: {
				address: configService.getCNgnAsset(BASE_CHAIN)!,
				decimals: 6,
				symbol: "cNGN",
			},
			amountIn: 1_000_000n,
		})

		console.log("amountIn:", quote.amountIn.toString())
		console.log("amountOut:", quote.amountOut.toString())
		console.log("protocolFeeBps:", quote.quoteMetadata.protocolFeeBps.toString())
		console.log("poolKey:", quote.quoteMetadata.poolKey)
		console.log("quoterAddress:", quote.quoteMetadata.quoterAddress)

		assert.equal(quote.strategy, "uniswap_v4")
		assert.equal(quote.tradeType, "EXACT_INPUT")
		assert.equal(quote.amountIn, 1_000_000n)
		assert(quote.amountOut > 0n)
		assert.equal(quote.quoteMetadata.poolKey.fee, 1500)
		assert.equal(quote.quoteMetadata.poolKey.tickSpacing, 30)
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
	const apiKey = process.env.BUNDLER_API_KEY
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
	const inputs: TokenInfo[] = [{ token: inputToken, amount }]
	const outputAssets: TokenInfo[] = [{ token: outputToken, amount }]

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

	const estimate = await intentGateway.estimateFillOrder({ order })

	console.log(`${srcKey} => ${destKey}`)
	console.log("Estimated cost (totalGasCostWei):", estimate.totalGasCostWei)
	console.log("Estimated fee (totalGasInFeeToken):", estimate.totalGasInFeeToken)

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
