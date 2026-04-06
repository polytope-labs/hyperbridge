import { describe, it, expect } from "vitest"
import { ChainClientManager, FillerConfigService, type ResolvedChainConfig } from "@/services"
import { createSimplexSigner, SignerType } from "@/services/wallet"
import { UniswapV4FundingPlanner } from "@/funding/uniswapV4/UniswapV4FundingPlanner"
import type { UniswapV4OutputFundingConfig } from "@/funding/types"
import type { HexString } from "@hyperbridge/sdk"
import { privateKeyToAccount } from "viem/accounts"
import "../setup"

const BASE_CHAIN = "EVM-8453"

describe.skip("UniswapV4 FundingVenue — cNGN price on Base", () => {
	it("should return a valid cNGN/USD price from the V4 pool", async () => {
		const privateKey = process.env.PRIVATE_KEY as HexString
		const solver = privateKeyToAccount(privateKey).address as HexString

		const chainConfigs: ResolvedChainConfig[] = [{ chainId: 8453, rpcUrl: process.env.BASE_MAINNET! }]

		const configService = new FillerConfigService(chainConfigs)
		const signer = await createSimplexSigner({ type: SignerType.PrivateKey, key: privateKey })
		const clientManager = new ChainClientManager(configService, signer)

		const fundingConfig: UniswapV4OutputFundingConfig = {
			positionsByChain: {
				[BASE_CHAIN]: [{ tokenId: 2087350n }],
			},
		}

		const venue = new UniswapV4FundingPlanner(clientManager, fundingConfig, configService)
		const cNGN = configService.getCNgnAsset(BASE_CHAIN)
		if (!cNGN) {
			throw new Error("cNGN asset not found")
		}

		await venue.initialise(solver)

		const price = await venue.getExoticTokenPrice(BASE_CHAIN, cNGN)
		console.log("cNGN/USD price:", price?.toString())

		expect(price).not.toBeNull()
		expect(price!.isPositive()).toBe(true)
	})
})
