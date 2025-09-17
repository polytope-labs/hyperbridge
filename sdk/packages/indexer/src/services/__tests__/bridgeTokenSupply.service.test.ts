import { BridgeTokenSupplyService } from "../bridgeTokenSupply.service"
import { replaceWebsocketWithHttp } from "@/utils/rpc.helpers"

// Mock the logger to avoid console output during tests
;(global as any).logger = {
	debug: jest.fn(),
	info: jest.fn(),
	warn: jest.fn(),
	error: jest.fn(),
}

// Mock chainId global
;(global as any).chainId = "0x1234567890abcdef"

describe("BridgeTokenSupplyService", () => {
	const TEST_RPC_URL = "wss://nexus.ibp.network"
	const HTTP_RPC_URL = replaceWebsocketWithHttp(TEST_RPC_URL)

	describe("getCirculatingSupply", () => {
		it("should successfully calculate circulating supply from live network", async () => {
			// This test connects to the live network, so it might be slow
			// and could fail if the network is unavailable
			const result = await BridgeTokenSupplyService["getCirculatingSupply"](HTTP_RPC_URL)

			// Verify the result is not an error
			expect(result).not.toBeInstanceOf(Error)

			// Verify the result is a valid BigInt
			expect(typeof result).toBe("bigint")
			expect(result).toBeGreaterThanOrEqual(BigInt(0))

			console.log(`Circulating supply: ${result}`)
		}, 30000) // 30 second timeout for network calls

		it("should successfully get total supply from live network", async () => {
			const result = await BridgeTokenSupplyService["getTotalSupply"](HTTP_RPC_URL)

			// Verify the result is not an error
			expect(result).not.toBeInstanceOf(Error)

			// Verify the result is a valid BigInt
			expect(typeof result).toBe("bigint")
			expect(result).toBeGreaterThan(BigInt(0))

			console.log(`Total supply: ${result}`)
		}, 15000) // 15 second timeout

		it("should successfully get inactive issuance from live network", async () => {
			const result = await BridgeTokenSupplyService["getInactiveIssuance"](HTTP_RPC_URL)

			// Verify the result is not an error
			expect(result).not.toBeInstanceOf(Error)

			// Verify the result is a valid BigInt
			expect(typeof result).toBe("bigint")
			expect(result).toBeGreaterThanOrEqual(BigInt(0))

			console.log(`Inactive issuance: ${result}`)
		}, 15000) // 15 second timeout

		it("should successfully get total account locks from live network", async () => {
			const result = await BridgeTokenSupplyService["getTotalAccountLocks"](HTTP_RPC_URL)

			// Verify the result is not an error
			expect(result).not.toBeInstanceOf(Error)

			// Verify the result is a valid BigInt
			expect(typeof result).toBe("bigint")
			expect(result).toBeGreaterThanOrEqual(BigInt(0))

			console.log(`Total account locks: ${result}`)
		}, 45000) // 45 second timeout for potentially many RPC calls

		it("should have circulating supply less than or equal to total supply", async () => {
			// Get both values
			const totalSupply = await BridgeTokenSupplyService["getTotalSupply"](HTTP_RPC_URL)
			const circulatingSupply = await BridgeTokenSupplyService["getCirculatingSupply"](HTTP_RPC_URL)

			// Verify neither is an error
			expect(totalSupply).not.toBeInstanceOf(Error)
			expect(circulatingSupply).not.toBeInstanceOf(Error)

			// Type assertions for proper type checking
			const totalSupplyBigInt = totalSupply as bigint
			const circulatingSupplyBigInt = circulatingSupply as bigint

			// Verify circulating supply is not greater than total supply
			expect(circulatingSupplyBigInt).toBeLessThanOrEqual(totalSupplyBigInt)

			console.log(`Total supply: ${totalSupplyBigInt}`)
			console.log(`Circulating supply: ${circulatingSupplyBigInt}`)
			console.log(`Locked/Inactive: ${totalSupplyBigInt - circulatingSupplyBigInt}`)
		}, 45000) // 45 second timeout
	})
})
