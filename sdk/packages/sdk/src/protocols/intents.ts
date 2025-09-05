import {
	bytes32ToBytes20,
	bytes20ToBytes32,
	constructRedeemEscrowRequestBody,
	fetchTokenUsdPrice,
	getStorageSlot,
	ADDRESS_ZERO,
	MOCK_ADDRESS,
	ERC20Method,
	adjustFeeDecimals,
} from "@/utils"
import { maxUint256, toHex } from "viem"
import { DispatchPost, type FillOptions, type HexString, type IPostRequest, type Order } from "@/types"
import IntentGatewayABI from "@/abis/IntentGateway"
import UniswapV2Factory from "@/abis/uniswapV2Factory"
import UniswapRouterV2 from "@/abis/uniswapRouterV2"
import UniswapV3Factory from "@/abis/uniswapV3Factory"
import UniswapV3Pool from "@/abis/uniswapV3Pool"
import UniswapV3Quoter from "@/abis/uniswapV3Quoter"
import { UNISWAP_V4_QUOTER_ABI } from "@/abis/uniswapV4Quoter"
import { type PublicClient } from "viem"
import { EvmChain } from "@/chains/evm"

/**
 * IntentGateway handles cross-chain intent operations between EVM chains.
 * It provides functionality for estimating fill orders, finding optimal swap protocols,
 * and checking order statuses across different chains.
 */
export class IntentGateway {
	/**
	 * Creates a new IntentGateway instance for cross-chain operations.
	 * @param source - The source EVM chain
	 * @param dest - The destination EVM chain
	 */
	constructor(
		public readonly source: EvmChain,
		public readonly dest: EvmChain,
	) {}

	/**
	 * Estimates the total cost required to fill an order, including gas fees, relayer fees,
	 * protocol fees, and swap operations.
	 *
	 * @param order - The order to estimate fill costs for
	 * @returns An object containing the estimated cost in both fee token and native token, plus the post request calldata
	 */
	async estimateFillOrder(
		order: Order,
	): Promise<{ feeTokenAmount: bigint; nativeTokenAmount: bigint; postRequestCalldata: HexString }> {
		const postRequest: IPostRequest = {
			source: order.destChain,
			dest: order.sourceChain,
			body: constructRedeemEscrowRequestBody(order, MOCK_ADDRESS),
			timeoutTimestamp: 0n,
			nonce: await this.source.getHostNonce(),
			from: this.source.config.getIntentGatewayAddress(order.destChain),
			to: this.source.config.getIntentGatewayAddress(order.sourceChain),
		}

		const { decimals: sourceChainFeeTokenDecimals, address: sourceChainFeeTokenAddress } =
			await this.source.getFeeTokenWithDecimals()
		const { address: destChainFeeTokenAddress, decimals: destChainFeeTokenDecimals } =
			await this.dest.getFeeTokenWithDecimals()

		const { gas: postGasEstimate, postRequestCalldata } = await this.source.estimateGas(postRequest)

		const postGasEstimateInSourceFeeToken = await this.convertGasToFeeToken(
			postGasEstimate,
			this.source.client,
			sourceChainFeeTokenDecimals,
		)

		const RELAYER_FEE_BPS = 200n
		const relayerFeeInSourceFeeToken =
			postGasEstimateInSourceFeeToken + (postGasEstimateInSourceFeeToken * RELAYER_FEE_BPS) / 10000n

		const relayerFeeInDestFeeToken = adjustFeeDecimals(
			relayerFeeInSourceFeeToken,
			sourceChainFeeTokenDecimals,
			destChainFeeTokenDecimals,
		)

		const fillOptions: FillOptions = {
			relayerFee: relayerFeeInDestFeeToken,
		}

		const totalEthValue = order.outputs
			.filter((output) => bytes32ToBytes20(output.token) === ADDRESS_ZERO)
			.reduce((sum, output) => sum + output.amount, 0n)

		const intentGatewayAddress = this.source.config.getIntentGatewayAddress(order.destChain)
		const testValue = toHex(maxUint256 / 2n)

		const orderOverrides = await Promise.all(
			order.outputs.map(async (output) => {
				const tokenAddress = bytes32ToBytes20(output.token)

				if (tokenAddress === ADDRESS_ZERO) {
					return null
				}

				try {
					const stateDiffs = []

					const balanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(MOCK_ADDRESS).slice(2)
					const balanceSlot = await getStorageSlot(this.dest.client, tokenAddress, balanceData as HexString)
					stateDiffs.push({ slot: balanceSlot as HexString, value: testValue })

					try {
						const allowanceData =
							ERC20Method.ALLOWANCE +
							bytes20ToBytes32(MOCK_ADDRESS).slice(2) +
							bytes20ToBytes32(intentGatewayAddress).slice(2)
						const allowanceSlot = await getStorageSlot(
							this.dest.client,
							tokenAddress,
							allowanceData as HexString,
						)
						stateDiffs.push({ slot: allowanceSlot as HexString, value: testValue })
					} catch (e) {
						console.warn(`Could not find allowance slot for token ${tokenAddress}:`, e)
					}

					return { address: tokenAddress, stateDiff: stateDiffs }
				} catch (e) {
					console.warn(`Could not find balance slot for token ${tokenAddress}:`, e)
					return null
				}
			}),
		).then((results) => results.filter(Boolean))

		let stateOverrides = [
			// Mock address with ETH balance so that any chain estimation runs
			// even when the address doesn't hold any native token in that chain
			{
				address: MOCK_ADDRESS,
				balance: maxUint256,
			},
			...orderOverrides.map((override) => ({
				address: override!.address,
				stateDiff: override!.stateDiff,
			})),
		]

		let destChainFillGas = 0n

		try {
			const protocolFeeInNativeToken = await this.quoteNative(postRequest, relayerFeeInDestFeeToken)
			destChainFillGas = await this.dest.client.estimateContractGas({
				abi: IntentGatewayABI.ABI,
				address: intentGatewayAddress,
				functionName: "fillOrder",
				args: [transformOrderForContract(order), fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue + protocolFeeInNativeToken,
				stateOverride: stateOverrides as any,
			})
		} catch {
			console.warn(
				`Could not estimate gas for fill order with native token as fees for chain ${order.destChain}, now trying with fee token as fees`,
			)

			const destFeeTokenBalanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(MOCK_ADDRESS).slice(2)
			const destFeeTokenBalanceSlot = await getStorageSlot(
				this.dest.client,
				destChainFeeTokenAddress,
				destFeeTokenBalanceData as HexString,
			)
			const destFeeTokenAllowanceData =
				ERC20Method.ALLOWANCE +
				bytes20ToBytes32(MOCK_ADDRESS).slice(2) +
				bytes20ToBytes32(intentGatewayAddress).slice(2)
			const destFeeTokenAllowanceSlot = await getStorageSlot(
				this.dest.client,
				destChainFeeTokenAddress,
				destFeeTokenAllowanceData as HexString,
			)
			const feeTokenStateDiffs = [
				{ slot: destFeeTokenBalanceSlot, value: testValue },
				{ slot: destFeeTokenAllowanceSlot, value: testValue },
			]

			stateOverrides.push({
				address: destChainFeeTokenAddress,
				stateDiff: feeTokenStateDiffs as any,
			})

			destChainFillGas = await this.dest.client.estimateContractGas({
				abi: IntentGatewayABI.ABI,
				address: intentGatewayAddress,
				functionName: "fillOrder",
				args: [transformOrderForContract(order), fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue,
				stateOverride: stateOverrides as any,
			})
		}

		const fillGasInSourceFeeToken = await this.convertGasToFeeToken(
			destChainFillGas,
			this.dest.client,
			sourceChainFeeTokenDecimals,
		)

		const protocolFeeInSourceFeeToken = adjustFeeDecimals(
			// Following baseIsmpModule.sol, the protocol fee is added to the relayer fee
			(await this.dest.quote(postRequest)) + relayerFeeInDestFeeToken,
			destChainFeeTokenDecimals,
			sourceChainFeeTokenDecimals,
		)

		const totalEstimate = fillGasInSourceFeeToken + protocolFeeInSourceFeeToken + relayerFeeInSourceFeeToken

		const SWAP_OPERATIONS_BPS = 2500n
		const swapOperationsInFeeToken = (totalEstimate * SWAP_OPERATIONS_BPS) / 10000n
		const totalFeeTokenAmount = totalEstimate + swapOperationsInFeeToken

		const totalNativeTokenAmount = await this.convertFeeTokenToNative(
			totalFeeTokenAmount,
			this.source.client,
			sourceChainFeeTokenDecimals,
		)

		return {
			feeTokenAmount: totalFeeTokenAmount,
			nativeTokenAmount: totalNativeTokenAmount,
			postRequestCalldata,
		}
	}

	/**
	 * Converts fee token amounts back to the equivalent amount in native token.
	 * Uses USD pricing to convert between fee token amounts and native token costs.
	 *
	 * @param feeTokenAmount - The amount in fee token (DAI)
	 * @param publicClient - The client for the chain to get native token info
	 * @param feeTokenDecimals - The decimal places of the fee token
	 * @returns The fee token amount converted to native token amount
	 * @private
	 */
	private async convertFeeTokenToNative(
		feeTokenAmount: bigint,
		publicClient: PublicClient,
		feeTokenDecimals: number,
	): Promise<bigint> {
		const nativeToken = publicClient.chain?.nativeCurrency

		if (!nativeToken?.symbol || !nativeToken?.decimals) {
			throw new Error("Chain native currency information not available")
		}

		const feeTokenAmountNumber = Number(feeTokenAmount) / Math.pow(10, feeTokenDecimals)

		const nativeTokenPriceUsd = await fetchTokenUsdPrice(nativeToken.symbol)

		const totalCostInNativeToken = feeTokenAmountNumber / nativeTokenPriceUsd

		return BigInt(Math.floor(totalCostInNativeToken * Math.pow(10, nativeToken.decimals)))
	}

	/**
	 * Converts gas costs to the equivalent amount in the fee token (DAI).
	 * Uses USD pricing to convert between native token gas costs and fee token amounts.
	 *
	 * @param gasEstimate - The estimated gas units
	 * @param publicClient - The client for the chain to get gas prices
	 * @param targetDecimals - The decimal places of the target fee token
	 * @returns The gas cost converted to fee token amount
	 * @private
	 */
	private async convertGasToFeeToken(
		gasEstimate: bigint,
		publicClient: PublicClient,
		targetDecimals: number,
	): Promise<bigint> {
		const gasPrice = await publicClient.getGasPrice()
		const gasCostInWei = gasEstimate * gasPrice
		const nativeToken = publicClient.chain?.nativeCurrency

		if (!nativeToken?.symbol || !nativeToken?.decimals) {
			throw new Error("Chain native currency information not available")
		}

		const gasCostInToken = Number(gasCostInWei) / Math.pow(10, nativeToken.decimals)
		const tokenPriceUsd = await fetchTokenUsdPrice(nativeToken.symbol)
		const gasCostUsd = gasCostInToken * tokenPriceUsd

		const feeTokenPriceUsd = await fetchTokenUsdPrice("DAI")
		const gasCostInFeeToken = gasCostUsd / feeTokenPriceUsd

		return BigInt(Math.floor(gasCostInFeeToken * Math.pow(10, targetDecimals)))
	}

	async quoteNative(postRequest: IPostRequest, fee: bigint): Promise<bigint> {
		const dispatchPost: DispatchPost = {
			dest: toHex(postRequest.dest),
			to: postRequest.to,
			body: postRequest.body,
			timeout: postRequest.timeoutTimestamp,
			fee: fee,
			payer: postRequest.from,
		}

		const quoteNative = await this.dest.client.readContract({
			address: this.dest.config.getIntentGatewayAddress(postRequest.dest),
			abi: IntentGatewayABI.ABI,
			functionName: "quoteNative",
			args: [dispatchPost] as any,
		})

		return quoteNative
	}

	/**
	 * Finds the best Uniswap protocol (V2, V3, or V4) for swapping tokens given a desired output amount.
	 * Compares liquidity and pricing across different protocols and fee tiers.
	 *
	 * @param chain - The chain identifier where the swap will occur
	 * @param tokenIn - The address of the input token
	 * @param tokenOut - The address of the output token
	 * @param amountOut - The desired output amount
	 * @returns Object containing the best protocol, required input amount, and fee tier (for V3/V4)
	 */
	async findBestProtocolWithAmountOut(
		chain: string,
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
	): Promise<{ protocol: "v2" | "v3" | "v4" | null; amountIn: bigint; fee?: number }> {
		const destClient = this.dest.client
		let amountInV2 = maxUint256
		let amountInV3 = maxUint256
		let amountInV4 = maxUint256
		let bestV3Fee = 0
		let bestV4Fee = 0
		const commonFees = [100, 500, 3000, 10000]

		const v2Router = this.source.config.getUniswapRouterV2Address(chain)
		const v2Factory = this.source.config.getUniswapV2FactoryAddress(chain)
		const v3Factory = this.source.config.getUniswapV3FactoryAddress(chain)
		const v3Quoter = this.source.config.getUniswapV3QuoterAddress(chain)
		const v4Quoter = this.source.config.getUniswapV4QuoterAddress(chain)

		// For V2/V3, convert native addresses to WETH for quotes
		const wethAsset = this.source.config.getWrappedNativeAssetWithDecimals(chain).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		// V2 Protocol Check
		try {
			const v2PairExists = (await destClient.readContract({
				address: v2Factory,
				abi: UniswapV2Factory.ABI,
				functionName: "getPair",
				args: [tokenInForQuote, tokenOutForQuote],
			})) as HexString

			if (v2PairExists !== ADDRESS_ZERO) {
				const v2AmountIn = (await destClient.readContract({
					address: v2Router,
					abi: UniswapRouterV2.ABI,
					functionName: "getAmountsIn",
					args: [amountOut, [tokenInForQuote, tokenOutForQuote]],
				})) as bigint[]

				amountInV2 = v2AmountIn[0]
			}
		} catch (error) {
			console.warn("V2 quote failed:", error)
		}

		// V3 Protocol Check - Find the best pool with best quote
		let bestV3AmountIn = maxUint256

		for (const fee of commonFees) {
			try {
				const pool = await destClient.readContract({
					address: v3Factory,
					abi: UniswapV3Factory.ABI,
					functionName: "getPool",
					args: [tokenInForQuote, tokenOutForQuote, fee],
				})

				if (pool !== ADDRESS_ZERO) {
					const liquidity = await destClient.readContract({
						address: pool,
						abi: UniswapV3Pool.ABI,
						functionName: "liquidity",
					})

					if (liquidity > BigInt(0)) {
						// Use simulateContract for V3 quoter (handles revert-based returns)
						const quoteResult = (
							await destClient.simulateContract({
								address: v3Quoter,
								abi: UniswapV3Quoter.ABI,
								functionName: "quoteExactOutputSingle",
								args: [
									{
										tokenIn: tokenInForQuote,
										tokenOut: tokenOutForQuote,
										fee: fee,
										amount: amountOut,
										sqrtPriceLimitX96: BigInt(0),
									},
								],
							})
						).result as [bigint, bigint, number, bigint]

						const amountIn = quoteResult[0]

						if (amountIn < bestV3AmountIn) {
							bestV3AmountIn = amountIn
							bestV3Fee = fee
						}
					}
				}
			} catch (error) {
				console.warn(`V3 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		amountInV3 = bestV3AmountIn

		// V4 Protocol Check - Find the best pool with best quote (uses native addresses directly)
		let bestV4AmountIn = maxUint256

		for (const fee of commonFees) {
			try {
				// Create pool key for V4 - can use native addresses directly
				const currency0 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenIn : tokenOut
				const currency1 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenOut : tokenIn

				const zeroForOne = tokenIn.toLowerCase() === currency0.toLowerCase()

				const poolKey = {
					currency0: currency0,
					currency1: currency1,
					fee: fee,
					tickSpacing: this.getTickSpacing(fee),
					hooks: ADDRESS_ZERO, // No hooks
				}

				// Get quote from V4 quoter
				const quoteResult = (
					await destClient.simulateContract({
						address: v4Quoter,
						abi: UNISWAP_V4_QUOTER_ABI,
						functionName: "quoteExactOutputSingle",
						args: [
							{
								poolKey: poolKey,
								zeroForOne: zeroForOne,
								exactAmount: amountOut,
								hookData: "0x", // Empty hook data
							},
						],
					})
				).result as [bigint, bigint] // [amountIn, gasEstimate]

				const amountIn = quoteResult[0]

				if (amountIn < bestV4AmountIn) {
					bestV4AmountIn = amountIn
					bestV4Fee = fee
				}
			} catch (error) {
				console.warn(`V4 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		amountInV4 = bestV4AmountIn

		if (amountInV2 === maxUint256 && amountInV3 === maxUint256 && amountInV4 === maxUint256) {
			// No liquidity in any protocol
			return {
				protocol: null,
				amountIn: maxUint256,
			}
		}

		// Prefer V4 when V4 is close to the best of V2/V3 (within thresholdBps)
		if (amountInV4 !== maxUint256) {
			const thresholdBps = 100n // 1%
			if (amountInV3 !== maxUint256 && this.isWithinThreshold(amountInV4, amountInV3, thresholdBps)) {
				return { protocol: "v4", amountIn: amountInV4, fee: bestV4Fee }
			}
			if (amountInV2 !== maxUint256 && this.isWithinThreshold(amountInV4, amountInV2, thresholdBps)) {
				return { protocol: "v4", amountIn: amountInV4, fee: bestV4Fee }
			}
		}

		const minAmount = [
			{ protocol: "v2" as const, amountIn: amountInV2 },
			{ protocol: "v3" as const, amountIn: amountInV3, fee: bestV3Fee },
			{ protocol: "v4" as const, amountIn: amountInV4, fee: bestV4Fee },
		].reduce((best, current) => (current.amountIn < best.amountIn ? current : best))

		if (minAmount.protocol === "v2") {
			return {
				protocol: "v2",
				amountIn: amountInV2,
			}
		} else if (minAmount.protocol === "v3") {
			return {
				protocol: "v3",
				amountIn: amountInV3,
				fee: bestV3Fee,
			}
		} else {
			return {
				protocol: "v4",
				amountIn: amountInV4,
				fee: bestV4Fee,
			}
		}
	}

	/**
	 * Finds the best Uniswap protocol (V2, V3, or V4) for swapping tokens given an input amount.
	 * Compares liquidity and pricing across different protocols and fee tiers.
	 *
	 * @param chain - The chain identifier where the swap will occur
	 * @param tokenIn - The address of the input token
	 * @param tokenOut - The address of the output token
	 * @param amountIn - The input amount to swap
	 * @returns Object containing the best protocol, expected output amount, and fee tier (for V3/V4)
	 */
	async findBestProtocolWithAmountIn(
		chain: string,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
	): Promise<{ protocol: "v2" | "v3" | "v4" | null; amountOut: bigint; fee?: number }> {
		const destClient = this.dest.client
		let amountOutV2 = BigInt(0)
		let amountOutV3 = BigInt(0)
		let amountOutV4 = BigInt(0)
		let bestV3Fee = 0
		let bestV4Fee = 0
		const commonFees = [100, 500, 3000, 10000]

		const v2Router = this.source.config.getUniswapRouterV2Address(chain)
		const v2Factory = this.source.config.getUniswapV2FactoryAddress(chain)
		const v3Factory = this.source.config.getUniswapV3FactoryAddress(chain)
		const v3Quoter = this.source.config.getUniswapV3QuoterAddress(chain)
		const v4Quoter = this.source.config.getUniswapV4QuoterAddress(chain)

		// For V2/V3, convert native addresses to WETH for quotes
		const wethAsset = this.source.config.getWrappedNativeAssetWithDecimals(chain).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		// V2 Protocol Check
		try {
			const v2PairExists = (await destClient.readContract({
				address: v2Factory,
				abi: UniswapV2Factory.ABI,
				functionName: "getPair",
				args: [tokenInForQuote, tokenOutForQuote],
			})) as HexString

			if (v2PairExists !== ADDRESS_ZERO) {
				const v2AmountOut = (await destClient.readContract({
					address: v2Router,
					abi: UniswapRouterV2.ABI,
					functionName: "getAmountsOut",
					args: [amountIn, [tokenInForQuote, tokenOutForQuote]],
				})) as bigint[]

				amountOutV2 = v2AmountOut[1]
			}
		} catch (error) {
			console.warn("V2 quote failed:", error)
		}

		// V3 Protocol Check - Find the best pool with best quote
		let bestV3AmountOut = BigInt(0)

		for (const fee of commonFees) {
			try {
				const pool = await destClient.readContract({
					address: v3Factory,
					abi: UniswapV3Factory.ABI,
					functionName: "getPool",
					args: [tokenInForQuote, tokenOutForQuote, fee],
				})

				if (pool !== ADDRESS_ZERO) {
					const liquidity = await destClient.readContract({
						address: pool,
						abi: UniswapV3Pool.ABI,
						functionName: "liquidity",
					})

					if (liquidity > BigInt(0)) {
						// Use simulateContract for V3 quoter (handles revert-based returns)
						const quoteResult = (
							await destClient.simulateContract({
								address: v3Quoter,
								abi: UniswapV3Quoter.ABI,
								functionName: "quoteExactInputSingle",
								args: [
									{
										tokenIn: tokenInForQuote,
										tokenOut: tokenOutForQuote,
										fee: fee,
										amountIn: amountIn,
										sqrtPriceLimitX96: BigInt(0),
									},
								],
							})
						).result as [bigint, bigint, number, bigint]

						const amountOut = quoteResult[0]

						if (amountOut > bestV3AmountOut) {
							bestV3AmountOut = amountOut
							bestV3Fee = fee
						}
					}
				}
			} catch (error) {
				console.warn(`V3 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		amountOutV3 = bestV3AmountOut

		// V4 Protocol Check - Find the best pool with best quote (uses native addresses directly)
		let bestV4AmountOut = BigInt(0)

		for (const fee of commonFees) {
			try {
				// Create pool key for V4 - can use native addresses directly
				const currency0 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenIn : tokenOut
				const currency1 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenOut : tokenIn

				const zeroForOne = tokenIn.toLowerCase() === currency0.toLowerCase()

				const poolKey = {
					currency0: currency0,
					currency1: currency1,
					fee: fee,
					tickSpacing: this.getTickSpacing(fee),
					hooks: ADDRESS_ZERO, // No hooks
				}

				// Get quote from V4 quoter
				const quoteResult = (
					await destClient.simulateContract({
						address: v4Quoter,
						abi: UNISWAP_V4_QUOTER_ABI,
						functionName: "quoteExactInputSingle",
						args: [
							{
								poolKey: poolKey,
								zeroForOne: zeroForOne,
								exactAmount: amountIn,
								hookData: "0x", // Empty hook data
							},
						],
					})
				).result as [bigint, bigint] // [amountOut, gasEstimate]

				const amountOut = quoteResult[0]

				if (amountOut > bestV4AmountOut) {
					bestV4AmountOut = amountOut
					bestV4Fee = fee
				}
			} catch (error) {
				console.warn(`V4 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		amountOutV4 = bestV4AmountOut

		if (amountOutV2 === BigInt(0) && amountOutV3 === BigInt(0) && amountOutV4 === BigInt(0)) {
			// No liquidity in any protocol
			return {
				protocol: null,
				amountOut: BigInt(0),
			}
		}

		// Prefer V4 when V4 is close to the best of V2/V3 (within thresholdBps)
		if (amountOutV4 !== BigInt(0)) {
			const thresholdBps = 100n // 1%
			if (amountOutV3 !== BigInt(0) && this.isWithinThreshold(amountOutV4, amountOutV3, thresholdBps)) {
				return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee }
			}
			if (amountOutV2 !== BigInt(0) && this.isWithinThreshold(amountOutV4, amountOutV2, thresholdBps)) {
				return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee }
			}
		}

		const maxAmount = [
			{ protocol: "v2" as const, amountOut: amountOutV2 },
			{ protocol: "v3" as const, amountOut: amountOutV3, fee: bestV3Fee },
			{ protocol: "v4" as const, amountOut: amountOutV4, fee: bestV4Fee },
		].reduce((best, current) => (current.amountOut > best.amountOut ? current : best))

		if (maxAmount.protocol === "v2") {
			return {
				protocol: "v2",
				amountOut: amountOutV2,
			}
		} else if (maxAmount.protocol === "v3") {
			return {
				protocol: "v3",
				amountOut: amountOutV3,
				fee: bestV3Fee,
			}
		} else {
			return {
				protocol: "v4",
				amountOut: amountOutV4,
				fee: bestV4Fee,
			}
		}
	}

	/**
	 * Checks if an order has been filled by verifying the commitment status on-chain.
	 * Reads the storage slot corresponding to the order's commitment hash.
	 *
	 * @param order - The order to check
	 * @returns True if the order has been filled, false otherwise
	 */
	async isOrderFilled(order: Order): Promise<boolean> {
		const intentGatewayAddress = this.source.config.getIntentGatewayAddress(order.destChain)

		let filledSlot = await this.dest.client.readContract({
			abi: IntentGatewayABI.ABI,
			address: intentGatewayAddress,
			functionName: "calculateCommitmentSlotHash",
			args: [order.id as HexString],
		})

		const filledStatus = await this.dest.client.getStorageAt({
			address: intentGatewayAddress,
			slot: filledSlot,
		})
		return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
	}

	/**
	 * Returns the tick spacing for a given fee tier in Uniswap V4
	 * @param fee - The fee tier in basis points
	 * @returns The tick spacing value
	 */
	private getTickSpacing(fee: number): number {
		switch (fee) {
			case 100: // 0.01%
				return 1
			case 500: // 0.05%
				return 10
			case 3000: // 0.30%
				return 60
			case 10000: // 1.00%
				return 200
			default:
				return 60 // Default to medium
		}
	}

	/**
	 * Returns true if candidate <= reference * (1 + thresholdBps/10000)
	 * @param candidate - The candidate amount to compare
	 * @param reference - The reference amount
	 * @param thresholdBps - The threshold in basis points
	 * @returns True if candidate is within threshold of reference
	 */
	private isWithinThreshold(candidate: bigint, reference: bigint, thresholdBps: bigint): boolean {
		const basisPoints = 10000n
		return candidate * basisPoints <= reference * (basisPoints + thresholdBps)
	}
}

/**
 * Transforms an Order object into the format expected by the smart contract.
 * Converts chain IDs to hex format and restructures input/output arrays.
 *
 * @param order - The order to transform
 * @returns The order in contract-compatible format
 */
function transformOrderForContract(order: Order) {
	return {
		sourceChain: toHex(order.sourceChain),
		destChain: toHex(order.destChain),
		fees: order.fees,
		callData: order.callData,
		deadline: order.deadline,
		nonce: order.nonce,
		inputs: order.inputs.map((input) => ({
			token: input.token,
			amount: input.amount,
		})),
		outputs: order.outputs.map((output) => ({
			token: output.token,
			amount: output.amount,
			beneficiary: output.beneficiary,
		})),
		user: order.user,
	}
}
