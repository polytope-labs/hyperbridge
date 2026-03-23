import "log-timestamp"

import { toHex } from "viem"
import { strict as assert } from "assert"
import { type HexString, Order, TokenInfo } from "@/types"
import { EvmChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { bytes20ToBytes32 } from "@/utils"

// ---------------------------------------------------------------------------
// Test Cases
// ---------------------------------------------------------------------------

describe("IntentGateway cross-chain estimate tests", () => {
	for (const [src, dest] of CROSS_CHAIN_CASES) {
		it(`Should estimate fee for ${src} => ${dest}`, async () => {
			await runCrossChainEstimate(src, dest)
		}, 1_000_000)
	}
})

describe.sequential("IntentGateway same-chain estimate tests", () => {
	for (const chain of SAME_CHAIN_CASES) {
		it(`Should estimate fee for ${chain} same-chain USDC => EXT`, async () => {
			await runSameChainEstimate(chain)
		}, 1_000_000)
	}
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

const BENEFICIARY = "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString

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
		rpcUrl: process.env[chain.rpcEnvVar]!,
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
		source: toHex(sourceChainId),
		destination: toHex(destChainId),
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
		bytes20ToBytes32(configService.getUsdcAsset(src.id)),
		bytes20ToBytes32(configService.getUsdcAsset(dest.id)),
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
		bytes20ToBytes32(configService.getUsdcAsset(chain.id)),
		bytes20ToBytes32(configService.getExtAsset(chain.id)!),
		100n,
	)

	const estimate = await intentGateway.estimateFillOrder({ order })

	console.log(`${chainKey} same-chain estimated cost (totalGasCostWei):`, estimate.totalGasCostWei)
	console.log(`${chainKey} same-chain USDC => EXT, estimated fee:`, estimate.totalGasInFeeToken)
	assert(estimate.totalGasCostWei > 0n)
	assert(estimate.totalGasInFeeToken > 0n)
}
