import { describe, it, expect } from "vitest"
import { ChainClientManager, FillerConfigService, RebalancingService, type UserProvidedChainConfig } from "@/services"
import type { HexString } from "@hyperbridge/sdk"
import "../setup"

describe("RebalancingService - USDT0 (LayerZero OFT)", () => {
	it.only("Should send 1 USDT from Arbitrum to Polygon", async () => {
		const { rebalancingService } = await setUp()

		const result = await rebalancingService.sendUsdt0({
			amount: "1",
			source: "EVM-42161", // Arbitrum
			destination: "EVM-137", // Polygon
		})

		console.log("USDT0 Transfer Result:", {
			success: result.success,
			txHash: result.txHash,
			amountSent: result.amountSent.toString(),
			amountReceived: result.amountReceived.toString(),
			nativeFee: result.nativeFee.toString(),
		})

		expect(result.success).toBe(true)
		expect(result.txHash).toBeDefined()
		expect(result.amountSent).toBe(BigInt("1000000")) // 1 USDT (6 decimals)
		expect(result.amountReceived).toBeGreaterThan(0n)
	}, 300_000) // 5 minute timeout for cross-chain transfer

	it("Should estimate USDT0 transfer cost from Arbitrum to Polygon", async () => {
		const { rebalancingService } = await setUp()

		const estimate = await rebalancingService.estimateUsdt0({
			amount: "10",
			source: "EVM-42161", // Arbitrum
			destination: "EVM-137", // Polygon
		})

		console.log("USDT0 Estimate:", {
			amountSent: estimate.amountSent.toString(),
			amountReceived: estimate.amountReceived.toString(),
			nativeFee: estimate.nativeFee.toString(),
			minAmount: estimate.minAmount.toString(),
			maxAmount: estimate.maxAmount.toString(),
		})

		expect(estimate.amountSent).toBe(BigInt("10000000")) // 10 USDT (6 decimals)
		expect(estimate.amountReceived).toBeGreaterThan(0n)
		expect(estimate.nativeFee).toBeGreaterThan(0n)
	}, 60_000)
})

async function setUp() {
	const arbitrumId = "EVM-42161"
	const polygonId = "EVM-137"

	const testChainConfigs: UserProvidedChainConfig[] = [
		{
			chainId: 42161, // Arbitrum
			rpcUrl: process.env.ARBITRUM_MAINNET!,
		},
		{
			chainId: 137, // Polygon
			rpcUrl: process.env.POLYGON_MAINNET!,
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
		arbitrumId,
		polygonId,
	}
}
