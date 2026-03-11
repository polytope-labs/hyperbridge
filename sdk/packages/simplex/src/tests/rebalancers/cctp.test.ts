import { describe, it, expect } from "vitest"
import { ChainClientManager, FillerConfigService, RebalancingService, type UserProvidedChainConfig } from "@/services"
import type { HexString } from "@hyperbridge/sdk"
import "../setup"

describe("RebalancingService - CCTP", () => {
	it("Should send 0.1 USDC from Polygon Amoy to Arbitrum Sepolia", async () => {
		const { rebalancingService } = await setUp()

		const result = await rebalancingService.sendCctp({
			amount: "0.1",
			source: "EVM-80002", // Polygon Amoy
			destination: "EVM-421614", // Arbitrum Sepolia
		})

		console.log("CCTP Transfer Result:", {
			state: result.state,
			amount: result.amount,
			source: result.source.chain.chain,
			destination: result.destination.chain.chain,
			steps: result.steps.map((s) => ({
				name: s.name,
				state: s.state,
				txHash: s.txHash,
			})),
		})

		expect(result.state).toBe("success")
	}, 300_000) // 5 minute timeout for cross-chain transfer

	it("Should estimate CCTP transfer cost", async () => {
		const { rebalancingService } = await setUp()

		const estimate = await rebalancingService.estimateCctp({
			amount: "0.1",
			source: "EVM-80002", // Polygon Amoy
			destination: "EVM-421614", // Arbitrum Sepolia
		})

		console.log("CCTP Estimate:", {
			amount: estimate.amount,
			source: estimate.source.chain,
			destination: estimate.destination.chain,
			fees: estimate.fees,
		})

		expect(estimate.amount).toBe("0.1")
		expect(estimate.fees).toBeDefined()
	}, 60_000)
})

async function setUp() {
	const polygonAmoyId = "EVM-80002"
	const arbitrumSepoliaId = "EVM-421614"

	const testChainConfigs: UserProvidedChainConfig[] = [
		{
			chainId: 80002, // Polygon Amoy
			rpcUrl: process.env.POLYGON_AMOY!,
		},
		{
			chainId: 421614, // Arbitrum Sepolia
			rpcUrl: process.env.ARBITRUM_SEPOLIA!,
		},
	]

	const chainConfigService = new FillerConfigService(testChainConfigs)
	const privateKey = process.env.PRIVATE_KEY as HexString
	const chainClientManager = new ChainClientManager(chainConfigService, privateKey)

	const rebalancingService = new RebalancingService(chainClientManager, chainConfigService, privateKey)

	return {
		rebalancingService,
		chainClientManager,
		chainConfigService,
		polygonAmoyId,
		arbitrumSepoliaId,
	}
}
