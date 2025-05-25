import { ChainConfigService, ChainClientManager, ContractInteractionService } from "@/services"
import { ADDRESS_ZERO, bytes32ToBytes20, ExecutionResult, FillOptions, HexString, Order } from "hyperbridge-sdk"
import { FillerStrategy } from "./base"
import { privateKeyToAddress } from "viem/accounts"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { encodeAbiParameters, encodeFunctionData, encodePacked, maxUint256 } from "viem"
import { BATCH_EXECUTOR_ABI } from "@/config/abis/BatchExecutor"
import { UNISWAP_ROUTER_V2_ABI } from "@/config/abis/UniswapRouterV2"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { UNIVERSAL_ROUTER_ABI } from "@/config/abis/UniversalRouter"
import { UNISWAP_V2_FACTORY_ABI } from "@/config/abis/UniswapV2Factory"
import { UNISWAP_V3_FACTORY_ABI } from "@/config/abis/UniswapV3Factory"
import { UNISWAP_V3_POOL_ABI } from "@/config/abis/UniswapV3Pool"
import { UNISWAP_V3_QUOTER_V2_ABI } from "@/config/abis/UniswapV3QuoterV2"

export class StableSwapFiller implements FillerStrategy {
	name = "StableSwapFiller"
	private privateKey: HexString
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: ChainConfigService

	constructor(privateKey: HexString) {
		this.privateKey = privateKey
		this.configService = new ChainConfigService()
		this.clientManager = new ChainClientManager(privateKey)
		this.contractService = new ContractInteractionService(this.clientManager, privateKey)
	}

	/**
	 * Checks the USD value of the filler's balance against the order's USD value
	 * @param order The order to check if it can be filled
	 * @returns True if the filler has enough balance, false otherwise
	 */
	async canFill(order: Order): Promise<boolean> {
		try {
			const destClient = this.clientManager.getPublicClient(order.destChain)
			const currentBlock = await destClient.getBlockNumber()
			const deadline = BigInt(order.deadline)

			if (deadline < currentBlock) {
				console.debug(`Order expired at block ${deadline}, current block ${currentBlock}`)
				return false
			}

			const isAlreadyFilled = await this.contractService.checkIfOrderFilled(order)
			if (isAlreadyFilled) {
				console.debug(`Order is already filled`)
				return false
			}

			const fillerBalanceUsd = await this.contractService.getFillerBalanceUSD(order, order.destChain)

			// Check if the filler has enough USD value to fill the order
			const { outputUsdValue } = await this.contractService.getTokenUsdValue(order)

			if (fillerBalanceUsd.totalBalanceUsd < outputUsdValue) {
				console.debug(`Insufficient USD value for order`)
				return false
			}

			return true
		} catch (error) {
			console.error(`Error in canFill:`, error)
			return false
		}
	}

	/**
	 * Calculates the USD value of the order's inputs, outputs, fees and compares
	 * what will the filler receive and what will the filler pay
	 * @param order The order to calculate the USD value for
	 * @returns The profit in USD (BigInt)
	 */
	async calculateProfitability(order: Order): Promise<bigint> {
		try {
			const { fillGas, postGas } = await this.contractService.estimateGasFillPost(order)
			const { totalGasEstimate } = await this.calculateSwapOperations(order, order.destChain)
			const nativeTokenPriceUsd = await this.contractService.getNativeTokenPriceUsd(order)

			const relayerFeeEth = postGas + (postGas * BigInt(200)) / BigInt(10000)

			const protocolFeeUSD = await this.contractService.getProtocolFeeUSD(order, relayerFeeEth)

			const totalGasWei = fillGas + relayerFeeEth + totalGasEstimate

			const gasCostUsd = (totalGasWei * nativeTokenPriceUsd) / BigInt(10 ** 18)

			const totalGasCostUsd = gasCostUsd + protocolFeeUSD

			const { outputUsdValue, inputUsdValue } = await this.contractService.getTokenUsdValue(order)

			const toReceive = outputUsdValue + order.fees
			const toPay = inputUsdValue + totalGasCostUsd

			const profit = toReceive - toPay

			return profit
		} catch (error) {
			console.error(`Error calculating profitability:`, error)
			return BigInt(0)
		}
	}

	async executeOrder(order: Order): Promise<ExecutionResult> {
		try {
			const { destClient, walletClient } = this.clientManager.getClientsForOrder(order)
			const startTime = Date.now()
			const fillerWalletAddress = privateKeyToAddress(this.privateKey)

			const { calls } = await this.calculateSwapOperations(order, order.destChain)

			const { postGas: postGasEstimate } = await this.contractService.estimateGasFillPost(order)
			const fillOptions: FillOptions = {
				relayerFee: postGasEstimate + (postGasEstimate * BigInt(200)) / BigInt(10000),
			}

			await this.contractService.approveTokensIfNeeded(order)

			const fillOrderData = encodeFunctionData({
				abi: INTENT_GATEWAY_ABI,
				functionName: "fillOrder",
				args: [this.contractService.transformOrderForContract(order), fillOptions as any],
			})

			calls.push({
				to: this.configService.getIntentGatewayAddress(order.destChain),
				data: fillOrderData,
				value: this.contractService.calculateRequiredEthValue(order.outputs),
			})

			const authorization = await walletClient.signAuthorization({
				contractAddress: this.configService.getBatchExecutorAddress(order.destChain),
				account: walletClient.account!,
			})

			const tx = await walletClient.sendTransaction({
				account: walletClient.account!,
				chain: destClient.chain,
				data: encodeFunctionData({
					abi: BATCH_EXECUTOR_ABI,
					functionName: "execute",
					args: [calls],
				}),
				to: fillerWalletAddress,
				authorizationList: [authorization],
			})

			const endTime = Date.now()
			const processingTimeMs = endTime - startTime

			const receipt = await destClient.waitForTransactionReceipt({ hash: tx })

			return {
				success: true,
				txHash: receipt.transactionHash,
				gasUsed: receipt.gasUsed.toString(),
				gasPrice: receipt.effectiveGasPrice.toString(),
				confirmedAtBlock: Number(receipt.blockNumber),
				confirmedAt: new Date(endTime),
				strategyUsed: this.name,
				processingTimeMs,
			}
		} catch (error) {
			console.error(`Error executing order:`, error)
			return {
				success: false,
			}
		}
	}

	async calculateSwapOperations(
		order: Order,
		destChain: string,
	): Promise<{ calls: { to: HexString; data: HexString; value: bigint }[]; totalGasEstimate: bigint }> {
		const contractService = this.contractService
		const cacheService = contractService.cacheService

		// Check cache first
		const cachedOperations = cacheService.getSwapOperations(order.id!)
		if (cachedOperations) {
			console.log(`Using cached swap operations for order ${order.id}`)
			return {
				calls: cachedOperations.calls.map((call) => ({
					to: call.to as HexString,
					data: call.data as HexString,
					value: BigInt(call.value),
				})),
				totalGasEstimate: cachedOperations.totalGasEstimate,
			}
		}

		const calls: { to: HexString; data: HexString; value: bigint }[] = []
		let totalGasEstimate = BigInt(0)
		const fillerWalletAddress = privateKeyToAddress(this.privateKey)
		const destClient = this.clientManager.getPublicClient(destChain)

		const daiAsset = this.configService.getDaiAsset(destChain)
		const usdtAsset = this.configService.getUsdtAsset(destChain)
		const usdcAsset = this.configService.getUsdcAsset(destChain)
		const daiDecimals = await contractService.getTokenDecimals(daiAsset, destChain)
		const usdtDecimals = await contractService.getTokenDecimals(usdtAsset, destChain)
		const usdcDecimals = await contractService.getTokenDecimals(usdcAsset, destChain)

		// Universal Router constants
		const V2_SWAP_EXACT_OUT = 0x09
		const V3_SWAP_EXACT_OUT = 0x01

		for (const token of order.outputs) {
			const tokenAddress = bytes32ToBytes20(token.token)
			const { nativeTokenBalance, daiBalance, usdcBalance, usdtBalance } =
				await contractService.getFillerBalanceUSD(order, destChain)

			const currentBalance =
				tokenAddress == daiAsset
					? daiBalance
					: tokenAddress == usdtAsset
						? usdtBalance
						: tokenAddress == usdcAsset
							? usdcBalance
							: nativeTokenBalance

			const balanceNeeded = token.amount > currentBalance ? token.amount - currentBalance : BigInt(0)

			if (balanceNeeded > BigInt(0)) {
				const normalizedBalances = {
					dai: daiBalance / BigInt(10 ** daiDecimals),
					usdt: usdtBalance / BigInt(10 ** usdtDecimals),
					usdc: usdcBalance / BigInt(10 ** usdcDecimals),
					native: nativeTokenBalance / BigInt(10 ** 18),
				}

				const sortedBalances = Object.entries(normalizedBalances).sort(([, a], [, b]) => Number(b - a))

				// Try to fulfill the requirement using the highest balance first
				let remainingNeeded = balanceNeeded
				for (const [tokenType, normalizedBalance] of sortedBalances) {
					if (remainingNeeded <= BigInt(0)) break

					// Skip if this is the same token we're trying to get
					if (
						(tokenType === "dai" && tokenAddress === daiAsset) ||
						(tokenType === "usdt" && tokenAddress === usdtAsset) ||
						(tokenType === "usdc" && tokenAddress === usdcAsset) ||
						(tokenType === "native" && tokenAddress === ADDRESS_ZERO)
					) {
						continue
					}

					// Get the actual balance with decimals
					const actualBalance =
						tokenType === "dai"
							? daiBalance
							: tokenType === "usdt"
								? usdtBalance
								: tokenType === "usdc"
									? usdcBalance
									: nativeTokenBalance

					// Calculate how much we can swap from this token (in actual uint256 with decimals)
					const swapAmount = actualBalance > remainingNeeded ? remainingNeeded : actualBalance

					if (swapAmount > BigInt(0)) {
						const tokenToSwap =
							tokenType === "dai"
								? daiAsset
								: tokenType === "usdt"
									? usdtAsset
									: tokenType === "usdc"
										? usdcAsset
										: ADDRESS_ZERO

						const bestProtocol = await this.findBestProtocol(
							tokenToSwap,
							tokenAddress,
							swapAmount,
							destChain,
						)

						if (bestProtocol.protocol === null) {
							console.warn(`No liquidity available for swap ${tokenToSwap} -> ${tokenAddress}`)
							continue
						}

						const amountIn = bestProtocol.amountIn

						// Transfer tokens directly to Universal Router (no approval needed)
						const transferData = encodeFunctionData({
							abi: ERC20_ABI,
							functionName: "transfer",
							args: [this.configService.getUniversalRouterAddress(destChain), amountIn],
						})

						const transferCall = {
							to: tokenToSwap,
							data: transferData,
							value: BigInt(0),
						}

						// Universal Router swap call based on protocol
						let commands: HexString
						let inputs: HexString[]
						const isPermit2 = false

						if (bestProtocol.protocol === "v2") {
							// V2 swap
							const path = [tokenToSwap, tokenAddress]
							commands = encodePacked(["uint8"], [V2_SWAP_EXACT_OUT])

							// Inputs for V2_SWAP_EXACT_OUT: (recipient, amountOut, amountInMax, path, isPermit2)
							inputs = [
								encodeAbiParameters(
									[
										{ type: "address", name: "recipient" },
										{ type: "uint256", name: "amountOut" },
										{ type: "uint256", name: "amountInMax" },
										{ type: "address[]", name: "path" },
										{ type: "bool", name: "isPermit2" },
									],
									[fillerWalletAddress, swapAmount, amountIn, path, isPermit2],
								),
							]
						} else {
							// V3 swap
							// Encode path with fee: tokenIn + fee + tokenOut
							const pathV3 = encodePacked(
								["address", "uint24", "address"],
								[tokenToSwap, bestProtocol.fee!, tokenAddress],
							)

							commands = encodePacked(["uint8"], [V3_SWAP_EXACT_OUT])

							// Inputs for V3_SWAP_EXACT_OUT: (recipient, amountOut, amountInMax, path, isPermit2)
							inputs = [
								encodeAbiParameters(
									[
										{ type: "address", name: "recipient" },
										{ type: "uint256", name: "amountOut" },
										{ type: "uint256", name: "amountInMax" },
										{ type: "bytes", name: "path" },
										{ type: "bool", name: "isPermit2" },
									],
									[fillerWalletAddress, swapAmount, amountIn, pathV3, isPermit2],
								),
							]
						}

						const swapData = encodeFunctionData({
							abi: UNIVERSAL_ROUTER_ABI,
							functionName: "execute",
							args: [commands, inputs, order.deadline],
						})

						const call = {
							to: this.configService.getUniversalRouterAddress(destChain),
							data: swapData,
							value: BigInt(0),
						}

						try {
							const { results } = await destClient.simulateCalls({
								account: fillerWalletAddress,
								calls: [transferCall, call],
							})

							const operationGasEstimate = results.reduce(
								(acc, result) => acc + result.gasUsed,
								BigInt(0),
							)

							calls.push(transferCall, call)
							totalGasEstimate += operationGasEstimate
							remainingNeeded -= swapAmount

							console.log(
								`Using ${bestProtocol.protocol.toUpperCase()} for swap ${tokenToSwap} -> ${tokenAddress}, amountIn: ${amountIn}${bestProtocol.fee ? `, fee: ${bestProtocol.fee}` : ""}`,
							)
						} catch (simulationError) {
							console.error(
								`Swap simulation failed for ${tokenType} using ${bestProtocol.protocol}:`,
								simulationError,
							)
							continue
						}
					}
				}

				// If we still need more tokens after trying all balances
				if (remainingNeeded > BigInt(0)) {
					throw new Error(`Insufficient balance to fulfill token requirement for ${tokenAddress}`)
				}
			}
		}

		// Cache the results
		cacheService.setSwapOperations(
			order.id!,
			calls.map((call) => ({
				to: call.to,
				data: call.data,
				value: call.value.toString(),
			})),
			totalGasEstimate,
		)

		return { calls, totalGasEstimate }
	}

	// Find whether uniswap v2 or v3 is the best protocol to use based on the amountIn the filler has to pay
	async findBestProtocol(
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		destChain: string,
	): Promise<{
		protocol: "v2" | "v3" | null
		amountIn: bigint
		fee?: number // For V3
		gasEstimate?: bigint // For V3
	}> {
		const destClient = this.clientManager.getPublicClient(destChain)
		let amountInV2 = maxUint256
		let amountInV3 = maxUint256
		let bestV3Fee = 0
		let v3GasEstimate = BigInt(0)

		const v2Router = this.configService.getUniswapRouterV2Address(destChain)
		const v2Factory = this.configService.getUniswapV2FactoryAddress(destChain)
		const v3Factory = this.configService.getUniswapV3FactoryAddress(destChain)
		const v3Quoter = this.configService.getUniswapV3QuoterAddress(destChain)

		try {
			const v2PairExists = (await destClient.readContract({
				address: v2Factory,
				abi: UNISWAP_V2_FACTORY_ABI,
				functionName: "getPair",
				args: [tokenIn, tokenOut],
			})) as HexString

			if (v2PairExists !== ADDRESS_ZERO) {
				const v2AmountIn = (await destClient.readContract({
					address: v2Router,
					abi: UNISWAP_ROUTER_V2_ABI,
					functionName: "getAmountsIn",
					args: [amountOut, [tokenIn, tokenOut]],
				})) as bigint[]

				amountInV2 = v2AmountIn[0]
			}
		} catch (error) {
			console.warn("V2 quote failed:", error)
		}

		// Find the best pool in v3 with best quote
		let bestV3AmountIn = maxUint256
		const fees = [500, 3000, 10000] // 0.05%, 0.3%, 1%

		for (const fee of fees) {
			try {
				const pool = await destClient.readContract({
					address: v3Factory,
					abi: UNISWAP_V3_FACTORY_ABI,
					functionName: "getPool",
					args: [tokenIn, tokenOut, fee],
				})

				if (pool !== ADDRESS_ZERO) {
					const liquidity = await destClient.readContract({
						address: pool,
						abi: UNISWAP_V3_POOL_ABI,
						functionName: "liquidity",
					})

					if (liquidity > BigInt(0)) {
						// Get quote from quoter
						const quoteResult = (await destClient.readContract({
							address: v3Quoter,
							abi: UNISWAP_V3_QUOTER_V2_ABI,
							functionName: "quoteExactOutputSingle",
							args: [
								{
									tokenIn: tokenIn,
									tokenOut: tokenOut,
									fee: fee,
									amount: amountOut,
									sqrtPriceLimitX96: BigInt(0),
								},
							],
						})) as [bigint, bigint, number, bigint] // [amountIn, sqrtPriceX96After, initializedTicksCrossed, gasEstimate]

						const [amountIn, , , gasEstimate] = quoteResult

						if (amountIn < bestV3AmountIn) {
							bestV3AmountIn = amountIn
							bestV3Fee = fee
							v3GasEstimate = gasEstimate
						}
					}
				}
			} catch (error) {
				console.warn(`V3 quote failed for fee ${fee}:`, error)
				// Continue to next fee tier
			}
		}

		amountInV3 = bestV3AmountIn

		if (amountInV2 === maxUint256 && amountInV3 === maxUint256) {
			// No liquidity in either protocol
			return {
				protocol: null,
				amountIn: maxUint256,
			}
		}

		if (amountInV2 <= amountInV3) {
			return {
				protocol: "v2",
				amountIn: amountInV2,
			}
		} else {
			return {
				protocol: "v3",
				amountIn: amountInV3,
				fee: bestV3Fee,
				gasEstimate: v3GasEstimate,
			}
		}
	}
}
