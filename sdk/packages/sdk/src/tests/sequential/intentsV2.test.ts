import "log-timestamp"

import { toHex } from "viem"
import { strict as assert } from "assert"
import { type HexString, OrderV2, TokenInfoV2 } from "@/types"
import { EvmChain } from "@/chain"
import { IntentsV2 } from "@/protocols/intentsV2/IntentsV2"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { bytes20ToBytes32 } from "@/utils"

// ---------------------------------------------------------------------------
// Test Cases
// ---------------------------------------------------------------------------

describe("IntentsV2 cross-chain estimate tests", () => {
	for (const [src, dest] of CROSS_CHAIN_CASES) {
		it(`Should estimate fee for ${src} => ${dest}`, async () => {
			await runCrossChainEstimate(src, dest)
		}, 1_000_000)
	}
})

describe.sequential("IntentsV2 same-chain estimate tests", () => {
	for (const chain of SAME_CHAIN_CASES) {
		it(`Should estimate fee for ${chain} same-chain USDC => EXT`, async () => {
			await runSameChainEstimate(chain)
		}, 1_000_000)
	}
})

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const CROSS_CHAIN_CASES: [string, string][] = [["bsc", "eth"]]

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

function makeEvmChain(chain: ChainDef, configService: ChainConfigService): EvmChain {
	return new EvmChain({
		chainId: chain.numericId,
		host: configService.getHostAddress(chain.id),
		rpcUrl: process.env[chain.rpcEnvVar]!,
	})
}

function buildOrderV2(
	sourceChainId: string,
	destChainId: string,
	inputToken: HexString,
	outputToken: HexString,
	amount: bigint,
): OrderV2 {
	const inputs: TokenInfoV2[] = [{ token: inputToken, amount }]
	const outputAssets: TokenInfoV2[] = [{ token: outputToken, amount }]

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
	const destChain = makeEvmChain(dest, configService)

	const intentsV2 = await IntentsV2.create(srcChain, destChain, undefined, bundlerUrl(dest.numericId))

	const order = buildOrderV2(
		src.id,
		dest.id,
		bytes20ToBytes32(configService.getUsdcAsset(src.id)),
		bytes20ToBytes32(configService.getUsdcAsset(dest.id)),
		100n,
	)

	const estimate = await intentsV2.estimateFillOrderV2({ order })

	console.log(`${srcKey} => ${destKey}`)
	console.log("Estimated fee (totalGasInFeeToken):", estimate.totalGasInFeeToken)

	assert(estimate.totalGasInFeeToken > 0n)
}

async function runSameChainEstimate(chainKey: string) {
	const chain = CHAINS[chainKey]
	const configService = new ChainConfigService()
	const evmChain = makeEvmChain(chain, configService)

	const intentsV2 = await IntentsV2.create(evmChain, evmChain, undefined, bundlerUrl(chain.numericId))

	const order = buildOrderV2(
		chain.id,
		chain.id,
		bytes20ToBytes32(configService.getUsdcAsset(chain.id)),
		bytes20ToBytes32(configService.getExtAsset(chain.id)!),
		100n,
	)

	const estimate = await intentsV2.estimateFillOrderV2({ order })

	console.log(`${chainKey} same-chain USDC => EXT, estimated fee:`, estimate.totalGasInFeeToken)
	assert(estimate.totalGasInFeeToken > 0n)
}
