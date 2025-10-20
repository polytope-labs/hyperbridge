import { describe, it, expect, beforeAll } from "vitest"
import { strict as assert } from "assert"
import {
	ContractInteractionService,
	ChainClientManager,
	FillerConfigService,
	UserProvidedChainConfig,
} from "@/services"
import {
	Order,
	HexString,
	fetchPrice,
	ADDRESS_ZERO,
	getStorageSlot,
	ERC20Method,
	bytes20ToBytes32,
} from "@hyperbridge/sdk"

import "./setup"
import { decodeFunctionResult, encodeFunctionData, maxUint256, parseUnits, toHex } from "viem"
import { privateKeyToAddress } from "viem/accounts"
import { UNIVERSAL_ROUTER_ABI } from "@/config/abis/UniversalRouter"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { fileURLToPath } from "url"
import { dirname, resolve } from "path"
import { readFileSync } from "fs"
import { parse } from "toml"
import { PERMIT2_ABI } from "@/config/abis/Permit2"

function loadTestConfig() {
	const __filename = fileURLToPath(import.meta.url)
	const __dirname = dirname(__filename)
	const configPath = resolve(__dirname, "test-config.toml")
	let tomlContent = readFileSync(configPath, "utf-8")

	// Replace environment variable placeholders with actual values
	tomlContent = tomlContent.replace(
		/\$\{BSC_CHAPEL\}/g,
		process.env.BSC_CHAPEL || "https://bnb-testnet.api.onfinality.io/public",
	)
	tomlContent = tomlContent.replace(
		/\$\{GNOSIS_CHIADO\}/g,
		process.env.GNOSIS_CHIADO || "https://gnosis-chiado-rpc.publicnode.com",
	)
	tomlContent = tomlContent.replace(
		/\$\{ETH_MAINNET\}/g,
		process.env.ETH_MAINNET || "https://eth-mainnet.g.alchemy.com/v2/demo",
	)
	tomlContent = tomlContent.replace(/\$\{BSC_MAINNET\}/g, process.env.BSC_MAINNET || "https://binance.llamarpc.com")
	tomlContent = tomlContent.replace(
		/\$\{HYPERBRIDGE_GARGANTUA\}/g,
		process.env.HYPERBRIDGE_GARGANTUA || "wss://gargantua.hyperbridge.xyz",
	)

	return parse(tomlContent)
}

describe.skip("ContractInteractionService", () => {
	let contractInteractionService: ContractInteractionService
	let chainClientManager: ChainClientManager
	let chainConfigService: FillerConfigService
	const bscChapelId = "EVM-97"
	const gnosisChiadoId = "EVM-10200"
	const mainnetId = "EVM-1"

	beforeAll(async () => {
		// Load test configuration from TOML file
		const config = loadTestConfig()

		// Convert TOML chain configs to UserProvidedChainConfig format
		const testChainConfigs: UserProvidedChainConfig[] = config.chains.map((chain: any) => ({
			chainId: chain.chainId,
			rpcUrl: chain.rpcUrl,
		}))

		// Convert TOML filler config including CoinGecko
		const fillerConfigForService = config.filler.coingecko
			? {
					privateKey: config.filler.privateKey,
					maxConcurrentOrders: config.filler.maxConcurrentOrders,
					coingecko: config.filler.coingecko,
				}
			: undefined

		chainConfigService = new FillerConfigService(testChainConfigs, fillerConfigForService)
		chainClientManager = new ChainClientManager(chainConfigService, process.env.PRIVATE_KEY as HexString)
		contractInteractionService = new ContractInteractionService(
			chainClientManager,
			process.env.PRIVATE_KEY as HexString,
			chainConfigService,
		)
	})

	describe("Misc test for services", () => {
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

			const inputUSDValue = await fetchPrice("WBNB", 97)
			const outputUSDValue = await fetchPrice("xDAI", 10200)

			expect(result.inputUsdValue).toBe(BigInt(Math.floor(inputUSDValue * Math.pow(10, 18))))
			expect(result.outputUsdValue).toBe(BigInt(Math.floor(outputUSDValue * Math.pow(10, 18))))
		})

		it.skip("should get native token price and handle testnet mapping", async () => {
			// Get native token price usd
			const inputUSDValue = await fetchPrice("WBNB", 97)
			let price = await contractInteractionService.getNativeTokenPrice(bscChapelId)

			expect(price).toBe(BigInt(Math.floor(inputUSDValue * Math.pow(10, 18))))

			// Test testnet map
			let testnetDaiAddr = await chainConfigService.getDaiAsset(bscChapelId)
			let testnetDaiPrice = await fetchPrice(testnetDaiAddr)

			console.log("Testnet DAI price", testnetDaiPrice)
		})

		it("should get V2 quote and swap using the quote", async () => {
			const fillerWalletAddress = privateKeyToAddress(process.env.PRIVATE_KEY as HexString)
			const tokenIn = chainConfigService.getDaiAsset(mainnetId)
			const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
			const tokenInDecimals = await contractInteractionService.getTokenDecimals(tokenIn, mainnetId)
			const tokenOutDecimals = await contractInteractionService.getTokenDecimals(tokenOut, mainnetId)
			const amoutOutBigInt = parseUnits("1000", tokenOutDecimals)
			let quote = await contractInteractionService.getV2QuoteWithAmountOut(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				mainnetId,
			)
			quote = quote + (quote * 200n) / 10000n
			assert(quote != maxUint256)
			assert(quote > parseUnits("1000", tokenInDecimals))
			const calldata = await contractInteractionService.createV2SwapCalldata(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				quote,
				fillerWalletAddress,
				{
					daiAsset: tokenIn,
					usdtAsset: tokenOut,
					usdcAsset: tokenOut,
					wethAsset: tokenOut,
				},
			)

			let calls = []
			calls.push({
				to: tokenIn,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "transfer",
					args: [chainConfigService.getUniversalRouterAddress(mainnetId), quote],
				}),
				value: 0n,
			})
			calls.push({
				to: chainConfigService.getUniversalRouterAddress(mainnetId),
				data: encodeFunctionData({
					abi: UNIVERSAL_ROUTER_ABI,
					functionName: "execute",
					args: [calldata.commands, calldata.inputs],
				}),
				value: 0n,
			})
			calls.push({
				to: tokenOut,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "balanceOf",
					args: [fillerWalletAddress],
				}),
				value: 0n,
			})

			// Balance override
			const client = await chainClientManager.getPublicClient(mainnetId)
			const slot = await getStorageSlot(
				client as any,
				tokenIn,
				(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
			)

			// Simulate with balance ovverides

			const result = await client.simulateCalls({
				account: fillerWalletAddress,
				calls,
				stateOverrides: [
					{
						address: tokenIn,
						stateDiff: [
							{
								slot: slot as `0x${string}`,
								value: toHex(maxUint256),
							},
						],
					},
				],
			})

			assert(result.results[1].status === "success")

			const balanceResult = result.results[2]
			assert(balanceResult.status === "success")

			// Decode the balanceOf return data
			const balance = decodeFunctionResult({
				abi: ERC20_ABI,
				functionName: "balanceOf",
				data: balanceResult.data,
			})

			assert(balance === amoutOutBigInt)
		})

		it("should get v3 quote and swap using the quote", async () => {
			const fillerWalletAddress = privateKeyToAddress(process.env.PRIVATE_KEY as HexString)
			const tokenIn = chainConfigService.getDaiAsset(mainnetId)
			const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
			const tokenInDecimals = await contractInteractionService.getTokenDecimals(tokenIn, mainnetId)
			const tokenOutDecimals = await contractInteractionService.getTokenDecimals(tokenOut, mainnetId)
			const amoutOutBigInt = parseUnits("1000", tokenOutDecimals)
			const { amountIn, fee } = await contractInteractionService.getV3QuoteWithAmountOut(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				mainnetId,
			)
			assert(amountIn != maxUint256)
			assert(amountIn > parseUnits("1000", tokenInDecimals))

			const calldata = await contractInteractionService.createV3SwapCalldata(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				amountIn,
				fee,
				fillerWalletAddress,
				{
					daiAsset: tokenIn,
					usdtAsset: tokenOut,
					usdcAsset: tokenOut,
					wethAsset: tokenOut,
				},
			)

			let calls = []
			calls.push({
				to: tokenIn,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "transfer",
					args: [chainConfigService.getUniversalRouterAddress(mainnetId), amountIn],
				}),
				value: 0n,
			})
			calls.push({
				to: chainConfigService.getUniversalRouterAddress(mainnetId),
				data: encodeFunctionData({
					abi: UNIVERSAL_ROUTER_ABI,
					functionName: "execute",
					args: [calldata.commands, calldata.inputs, 100000000000000n],
				}),
				value: 0n,
			})
			calls.push({
				to: tokenOut,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "balanceOf",
					args: [fillerWalletAddress],
				}),
				value: 0n,
			})

			// Balance override
			const client = await chainClientManager.getPublicClient(mainnetId)
			const slot = await getStorageSlot(
				client as any,
				tokenIn,
				(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
			)

			// Simulate with balance overrides
			const result = await client.simulateCalls({
				account: fillerWalletAddress,
				calls,
				stateOverrides: [
					{
						address: tokenIn,
						stateDiff: [
							{
								slot: slot as `0x${string}`,
								value: toHex(maxUint256),
							},
						],
					},
				],
			})

			assert(result.results[1].status === "success")

			const balanceResult = result.results[2]
			assert(balanceResult.status === "success")

			const balance = decodeFunctionResult({
				abi: ERC20_ABI,
				functionName: "balanceOf",
				data: balanceResult.data,
			})

			assert(balance === amoutOutBigInt)
		})

		it("should get v4 quote and swap using the quote", async () => {
			const fillerWalletAddress = privateKeyToAddress(process.env.PRIVATE_KEY as HexString)
			// ETH / USDC
			let tokenIn = ADDRESS_ZERO
			let tokenOut = chainConfigService.getUsdcAsset(mainnetId)

			let tokenOutDecimals = await contractInteractionService.getTokenDecimals(tokenOut, mainnetId)

			let amoutOutBigInt = parseUnits("1000", tokenOutDecimals)
			let { amountIn, fee } = await contractInteractionService.getV4QuoteWithAmountOut(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				mainnetId,
			)
			assert(amountIn != maxUint256)

			// Now test the swap

			let calldata = await contractInteractionService.createV4SwapCalldata(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				amountIn,
				fee,
			)

			let calls = []
			calls.push({
				to: chainConfigService.getUniversalRouterAddress(mainnetId),
				data: encodeFunctionData({
					abi: UNIVERSAL_ROUTER_ABI,
					functionName: "execute",
					args: [calldata.commands, calldata.inputs, 100000000000000n],
				}),
				value: 0n,
			})
			calls.push({
				to: tokenOut,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "balanceOf",
					args: [fillerWalletAddress],
				}),
				value: 0n,
			})

			const client = await chainClientManager.getPublicClient(mainnetId)

			// Simulate with balance overrides
			let result = await client.simulateCalls({
				account: fillerWalletAddress,
				calls,
				stateOverrides: [
					{
						address: chainConfigService.getUniversalRouterAddress(mainnetId),
						balance: amountIn,
					},
				],
			})

			assert(result.results[0].status === "success")

			let balanceResult = result.results[1]
			assert(balanceResult.status === "success")

			let balance = decodeFunctionResult({
				abi: ERC20_ABI,
				functionName: "balanceOf",
				data: balanceResult.data,
			})

			assert(balance === amoutOutBigInt)

			// // USDC/USDT
			tokenIn = chainConfigService.getUsdtAsset(mainnetId)
			tokenOut = chainConfigService.getUsdcAsset(mainnetId)
			tokenOutDecimals = await contractInteractionService.getTokenDecimals(tokenOut, mainnetId)
			amoutOutBigInt = parseUnits("1000", tokenOutDecimals)
			let { amountIn: amountIn2, fee: fee2 } = await contractInteractionService.getV4QuoteWithAmountOut(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				mainnetId,
			)
			assert(amountIn2 != maxUint256)

			// Now test the swap with adding a buffer to amountIn2
			amountIn2 = amountIn2 + (amountIn2 * 200n) / 10000n

			calldata = await contractInteractionService.createV4SwapCalldata(
				tokenIn,
				tokenOut,
				amoutOutBigInt,
				amountIn2,
				fee2,
			)

			calls = []
			calls.push({
				to: tokenIn,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "approve",
					args: [chainConfigService.getPermit2Address(mainnetId), amountIn2],
				}),
				value: 0n,
			})
			calls.push({
				to: chainConfigService.getPermit2Address(mainnetId),
				data: encodeFunctionData({
					abi: PERMIT2_ABI,
					functionName: "approve",
					args: [
						tokenIn,
						chainConfigService.getUniversalRouterAddress(mainnetId),
						amountIn2,
						281474976710655,
					],
				}),
				value: 0n,
			})

			calls.push({
				to: chainConfigService.getUniversalRouterAddress(mainnetId),
				data: encodeFunctionData({
					abi: UNIVERSAL_ROUTER_ABI,
					functionName: "execute",
					args: [calldata.commands, calldata.inputs],
				}),
				value: 0n,
			})
			calls.push({
				to: tokenOut,
				data: encodeFunctionData({
					abi: ERC20_ABI,
					functionName: "balanceOf",
					args: [fillerWalletAddress],
				}),
				value: 0n,
			})

			let slot = await getStorageSlot(
				client as any,
				tokenIn,
				(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
			)

			// Simulate with balance overrides
			result = await client.simulateCalls({
				account: fillerWalletAddress,
				calls,
				stateOverrides: [
					{
						address: tokenIn,
						stateDiff: [
							{
								slot: slot as `0x${string}`,
								value: toHex(maxUint256 / 3n),
							},
						],
					},
				],
			})

			balanceResult = result.results[3]
			assert(balanceResult.status === "success")

			balance = decodeFunctionResult({
				abi: ERC20_ABI,
				functionName: "balanceOf",
				data: balanceResult.data,
			})

			assert(balance === amoutOutBigInt)
		})
	})
})
