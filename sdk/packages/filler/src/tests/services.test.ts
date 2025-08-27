import { describe, it, expect, beforeAll } from "vitest"
import { ContractInteractionService, ChainClientManager } from "@/services"
import { ChainConfigService, Order, HexString, fetchTokenUsdPrice } from "@hyperbridge/sdk"

import "./setup"

describe.sequential("ContractInteractionService", () => {
	let contractInteractionService: ContractInteractionService
	let chainClientManager: ChainClientManager
	let chainConfigService: ChainConfigService
	let bscChapelId: string
	let gnosisChiadoId: string

	beforeAll(async () => {
		bscChapelId = "EVM-97"
		gnosisChiadoId = "EVM-10200"
		chainConfigService = new ChainConfigService()
		chainClientManager = new ChainClientManager(process.env.PRIVATE_KEY as HexString)
		contractInteractionService = new ContractInteractionService(
			chainClientManager,
			process.env.PRIVATE_KEY as HexString,
		)
	})

	describe("getTokenUsdValue", () => {
		it.skip("should calculate USD values for native token orders", async () => {
			const order: Order = {
				id: "test-order-1",
				user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
				sourceChain: bscChapelId,
				destChain: gnosisChiadoId,
				deadline: 6533729700n,
				nonce: 0n,
				fees: 1000000n,
				inputs: [
					{
						token: "0x0000000000000000000000000000000000000000000000000000000000000000", // Native token
						amount: 1n * 10n ** 18n,
					},
				],
				outputs: [
					{
						token: "0x0000000000000000000000000000000000000000000000000000000000000000", // Native token
						amount: 1n * 10n ** 18n,
						beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
					},
				],
				callData: "0x" as HexString,
			}

			const result = await contractInteractionService.getTokenUsdValue(order)

			const inputUSDValue = await fetchTokenUsdPrice("WBNB")
			const outputUSDValue = await fetchTokenUsdPrice("xDAI")

			expect(result.inputUsdValue).toBe(BigInt(Math.floor(inputUSDValue * Math.pow(10, 18))))
			expect(result.outputUsdValue).toBe(BigInt(Math.floor(outputUSDValue * Math.pow(10, 18))))
		})

		it.skip("should get native token price and handle testnet mapping", async () => {
			// Get native token price usd
			const inputUSDValue = await fetchTokenUsdPrice("WBNB")
			let price = await contractInteractionService.getNativeTokenPrice(bscChapelId)

			expect(price).toBe(BigInt(Math.floor(inputUSDValue * Math.pow(10, 18))))

			// Test testnet map
			let testnetDaiAddr = await chainConfigService.getDaiAsset(bscChapelId)
			let testnetDaiPrice = await fetchTokenUsdPrice(testnetDaiAddr)

			console.log("Testnet DAI price", testnetDaiPrice)
		})
	})
})
