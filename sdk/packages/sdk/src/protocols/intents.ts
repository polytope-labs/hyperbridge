import {
	bytes32ToBytes20,
	bytes20ToBytes32,
	constructRedeemEscrowRequestBody,
	getStorageSlot,
	ADDRESS_ZERO,
	MOCK_ADDRESS,
	ERC20Method,
	adjustFeeDecimals,
	fetchPrice,
	parseStateMachineId,
	orderCommitment,
	sleep,
	getRequestCommitment,
	waitForChallengePeriod,
	retryPromise,
} from "@/utils"
import { formatUnits, hexToString, maxUint256, pad, parseUnits, toHex } from "viem"
import {
	DispatchPost,
	IGetRequest,
	IHyperbridgeConfig,
	RequestStatus,
	type FillOptions,
	type HexString,
	type IPostRequest,
	type Order,
} from "@/types"
import IntentGatewayABI from "@/abis/IntentGateway"
import UniswapV2Factory from "@/abis/uniswapV2Factory"
import UniswapRouterV2 from "@/abis/uniswapRouterV2"
import UniswapV3Factory from "@/abis/uniswapV3Factory"
import UniswapV3Pool from "@/abis/uniswapV3Pool"
import UniswapV3Quoter from "@/abis/uniswapV3Quoter"
import { UNISWAP_V4_QUOTER_ABI } from "@/abis/uniswapV4Quoter"
import type { EvmChain } from "@/chains/evm"
import { Decimal } from "decimal.js"
import { getChain, IGetRequestMessage, IProof, SubstrateChain } from "@/chain"
import { IndexerClient } from "@/client"

/**
 * IntentGateway handles cross-chain intent operations between EVM chains.
 * It provides functionality for estimating fill orders, finding optimal swap protocols,
 * and checking order statuses across different chains.
 */
export class IntentGateway {
	private destStateproofCache = new Map<HexString, IProof>()

	public getCachedProof(id: HexString): IProof | undefined {
		return this.destStateproofCache.get(id)
	}

	public clearCachedProof(id: HexString) {
		this.destStateproofCache.delete(id)
	}

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

		const { decimals: sourceChainFeeTokenDecimals } = await this.source.getFeeTokenWithDecimals()

		const { address: destChainFeeTokenAddress, decimals: destChainFeeTokenDecimals } =
			await this.dest.getFeeTokenWithDecimals()

		const { gas: postGasEstimate, postRequestCalldata } = await this.source.estimateGas(postRequest)

		const postGasEstimateInSourceFeeToken = await this.convertGasToFeeToken(postGasEstimate, "source")

		const relayerFeeInSourceFeeToken =
			postGasEstimateInSourceFeeToken + 25n * 10n ** BigInt(sourceChainFeeTokenDecimals - 2)

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

		const stateOverrides = [
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
		let filledWithNativeToken = false

		try {
			let protocolFeeInNativeToken = await this.quoteNative(postRequest, relayerFeeInDestFeeToken)

			// Add 0.5% markup
			protocolFeeInNativeToken = protocolFeeInNativeToken + (protocolFeeInNativeToken * 50n) / 10000n

			destChainFillGas = await this.dest.client.estimateContractGas({
				abi: IntentGatewayABI.ABI,
				address: intentGatewayAddress,
				functionName: "fillOrder",
				args: [transformOrderForContract(order), fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue + protocolFeeInNativeToken,
				stateOverride: stateOverrides as any,
			})
			filledWithNativeToken = true
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

		const fillGasInDestFeeToken = await this.convertGasToFeeToken(destChainFillGas, "dest")
		const fillGasInSourceFeeToken = adjustFeeDecimals(
			fillGasInDestFeeToken,
			destChainFeeTokenDecimals,
			sourceChainFeeTokenDecimals,
		)

		const protocolFeeInSourceFeeToken = adjustFeeDecimals(
			await this.dest.quote(postRequest),
			destChainFeeTokenDecimals,
			sourceChainFeeTokenDecimals,
		)

		let totalEstimateInSourceFeeToken =
			fillGasInSourceFeeToken + protocolFeeInSourceFeeToken + relayerFeeInSourceFeeToken

		let totalNativeTokenAmount = await this.convertFeeTokenToNative(totalEstimateInSourceFeeToken, "source")

		if ([order.destChain, order.sourceChain].includes("EVM-1")) {
			totalEstimateInSourceFeeToken =
				totalEstimateInSourceFeeToken + (totalEstimateInSourceFeeToken * 3000n) / 10000n
			totalNativeTokenAmount = totalNativeTokenAmount + (totalNativeTokenAmount * 3200n) / 10000n
		} else {
			totalEstimateInSourceFeeToken =
				totalEstimateInSourceFeeToken + (totalEstimateInSourceFeeToken * 250n) / 10000n
			totalNativeTokenAmount = totalNativeTokenAmount + (totalNativeTokenAmount * 350n) / 10000n
		}
		return {
			feeTokenAmount: totalEstimateInSourceFeeToken,
			nativeTokenAmount: totalNativeTokenAmount,
			postRequestCalldata,
		}
	}

	/**
	 * Converts fee token amounts back to the equivalent amount in native token.
	 * Uses USD pricing to convert between fee token amounts and native token costs.
	 *
	 * @param feeTokenAmount - The amount in fee token (DAI)
	 * @param getQuoteIn - Whether to use "source" or "dest" chain for the conversion
	 * @returns The fee token amount converted to native token amount
	 * @private
	 */
	private async convertFeeTokenToNative(feeTokenAmount: bigint, getQuoteIn: "source" | "dest"): Promise<bigint> {
		const client = this[getQuoteIn].client
		const evmChainID = `EVM-${client.chain?.id}`
		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = await this[getQuoteIn].getFeeTokenWithDecimals()

		try {
			const { amountOut } = await this.findBestProtocolWithAmountIn(
				getQuoteIn,
				feeToken.address,
				wethAsset,
				feeTokenAmount,
				"v2",
			)

			if (amountOut === 0n) {
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = client.chain?.id
			const feeTokenAmountDecimal = new Decimal(formatUnits(feeTokenAmount, feeToken.decimals))
			const nativeTokenPriceUsd = new Decimal(await fetchPrice(nativeCurrency?.symbol!, chainId))
			const totalCostInNativeToken = feeTokenAmountDecimal.dividedBy(nativeTokenPriceUsd)
			return parseUnits(totalCostInNativeToken.toFixed(nativeCurrency?.decimals!), nativeCurrency?.decimals!)
		}
	}

	/**
	 * Converts gas costs to the equivalent amount in the fee token (DAI).
	 * Uses USD pricing to convert between native token gas costs and fee token amounts.
	 *
	 * @param gasEstimate - The estimated gas units
	 * @param gasEstimateIn - Whether to use "source" or "dest" chain for the conversion
	 * @returns The gas cost converted to fee token amount
	 * @private
	 */
	private async convertGasToFeeToken(gasEstimate: bigint, gasEstimateIn: "source" | "dest"): Promise<bigint> {
		const client = this[gasEstimateIn].client
		const gasPrice = await client.getGasPrice()
		const gasCostInWei = gasEstimate * gasPrice
		const evmChainID = `EVM-${client.chain?.id}`
		const wethAddr = this[gasEstimateIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = await this[gasEstimateIn].getFeeTokenWithDecimals()

		try {
			const { amountOut } = await this.findBestProtocolWithAmountIn(
				gasEstimateIn,
				wethAddr,
				feeToken.address,
				gasCostInWei,
				"v2",
			)
			if (amountOut === 0n) {
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = client.chain?.id
			const gasCostInToken = new Decimal(formatUnits(gasCostInWei, nativeCurrency?.decimals!))
			const tokenPriceUsd = await fetchPrice(nativeCurrency?.symbol!, chainId)
			const gasCostUsd = gasCostInToken.times(tokenPriceUsd)
			const feeTokenPriceUsd = new Decimal(1) // stable coin
			const gasCostInFeeToken = gasCostUsd.dividedBy(feeTokenPriceUsd)
			return parseUnits(gasCostInFeeToken.toFixed(feeToken.decimals), feeToken.decimals)
		}
	}

	/**
	 * Gets a quote for the native token cost of dispatching a post request.
	 *
	 * @param postRequest - The post request to quote
	 * @param fee - The fee amount in fee token
	 * @returns The native token amount required
	 */
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
	 * @param getQuoteIn - Whether to use "source" or "dest" chain for the swap
	 * @param tokenIn - The address of the input token
	 * @param tokenOut - The address of the output token
	 * @param amountOut - The desired output amount
	 * @returns Object containing the best protocol, required input amount, and fee tier (for V3/V4)
	 */
	async findBestProtocolWithAmountOut(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
	): Promise<{ protocol: "v2" | "v3" | "v4" | null; amountIn: bigint; fee?: number }> {
		const client = this[getQuoteIn].client
		const evmChainID = `EVM-${client.chain?.id}`
		let amountInV2 = maxUint256
		let amountInV3 = maxUint256
		let amountInV4 = maxUint256
		let bestV3Fee = 0
		let bestV4Fee = 0
		const commonFees = [100, 500, 3000, 10000]

		const v2Router = this.source.config.getUniswapRouterV2Address(evmChainID)
		const v2Factory = this.source.config.getUniswapV2FactoryAddress(evmChainID)
		const v3Factory = this.source.config.getUniswapV3FactoryAddress(evmChainID)
		const v3Quoter = this.source.config.getUniswapV3QuoterAddress(evmChainID)
		const v4Quoter = this.source.config.getUniswapV4QuoterAddress(evmChainID)

		// For V2/V3, convert native addresses to WETH for quotes
		const wethAsset = this.source.config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		// V2 Protocol Check
		try {
			const v2PairExists = (await client.readContract({
				address: v2Factory,
				abi: UniswapV2Factory.ABI,
				functionName: "getPair",
				args: [tokenInForQuote, tokenOutForQuote],
			})) as HexString

			if (v2PairExists !== ADDRESS_ZERO) {
				const v2AmountIn = (await client.readContract({
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
				const pool = await client.readContract({
					address: v3Factory,
					abi: UniswapV3Factory.ABI,
					functionName: "getPool",
					args: [tokenInForQuote, tokenOutForQuote, fee],
				})

				if (pool !== ADDRESS_ZERO) {
					const liquidity = await client.readContract({
						address: pool,
						abi: UniswapV3Pool.ABI,
						functionName: "liquidity",
					})

					if (liquidity > BigInt(0)) {
						// Use simulateContract for V3 quoter (handles revert-based returns)
						const quoteResult = (
							await client.simulateContract({
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
					await client.simulateContract({
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
	 * @param getQuoteIn - Whether to use "source" or "dest" chain for the swap
	 * @param tokenIn - The address of the input token
	 * @param tokenOut - The address of the output token
	 * @param amountIn - The input amount to swap
	 * @param selectedProtocol - Optional specific protocol to use ("v2", "v3", or "v4")
	 * @returns Object containing the best protocol, expected output amount, and fee tier (for V3/V4)
	 */
	async findBestProtocolWithAmountIn(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		selectedProtocol?: "v2" | "v3" | "v4",
	): Promise<{ protocol: "v2" | "v3" | "v4" | null; amountOut: bigint; fee?: number }> {
		const client = this[getQuoteIn].client
		const evmChainID = `EVM-${client.chain?.id}`
		let amountOutV2 = BigInt(0)
		let amountOutV3 = BigInt(0)
		let amountOutV4 = BigInt(0)
		let bestV3Fee = 0
		let bestV4Fee = 0
		const commonFees = [100, 500, 3000, 10000]

		const v2Router = this.source.config.getUniswapRouterV2Address(evmChainID)
		const v2Factory = this.source.config.getUniswapV2FactoryAddress(evmChainID)
		const v3Factory = this.source.config.getUniswapV3FactoryAddress(evmChainID)
		const v3Quoter = this.source.config.getUniswapV3QuoterAddress(evmChainID)
		const v4Quoter = this.source.config.getUniswapV4QuoterAddress(evmChainID)

		// For V2/V3, convert native addresses to WETH for quotes
		const wethAsset = this.source.config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		// V2 Protocol Check
		try {
			const v2PairExists = (await client.readContract({
				address: v2Factory,
				abi: UniswapV2Factory.ABI,
				functionName: "getPair",
				args: [tokenInForQuote, tokenOutForQuote],
			})) as HexString

			if (v2PairExists !== ADDRESS_ZERO) {
				const v2AmountOut = (await client.readContract({
					address: v2Router,
					abi: UniswapRouterV2.ABI,
					functionName: "getAmountsOut",
					args: [amountIn, [tokenInForQuote, tokenOutForQuote]],
				})) as bigint[]

				amountOutV2 = v2AmountOut[1]
				if (selectedProtocol === "v2") {
					return { protocol: "v2", amountOut: amountOutV2 }
				}
			}
		} catch (error) {
			console.warn("V2 quote failed:", error)
		}

		// V3 Protocol Check - Find the best pool with best quote
		let bestV3AmountOut = BigInt(0)

		for (const fee of commonFees) {
			try {
				const pool = await client.readContract({
					address: v3Factory,
					abi: UniswapV3Factory.ABI,
					functionName: "getPool",
					args: [tokenInForQuote, tokenOutForQuote, fee],
				})

				if (pool !== ADDRESS_ZERO) {
					const liquidity = await client.readContract({
						address: pool,
						abi: UniswapV3Pool.ABI,
						functionName: "liquidity",
					})

					if (liquidity > BigInt(0)) {
						// Use simulateContract for V3 quoter (handles revert-based returns)
						const quoteResult = (
							await client.simulateContract({
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

		if (selectedProtocol === "v3") {
			return { protocol: "v3", amountOut: amountOutV3, fee: bestV3Fee }
		}

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
					await client.simulateContract({
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

		if (selectedProtocol === "v4") {
			return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee }
		}

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

		const filledSlot = await this.dest.client.readContract({
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

	async *cancelOrder(order: Order, hyperbridgeConfig: IHyperbridgeConfig, indexerClient: IndexerClient) {
		const hyperbridge = (await getChain({
			...hyperbridgeConfig,
			hasher: "Keccak",
		})) as SubstrateChain

		const sourceStateMachine = hexToString(order.sourceChain as HexString)
		const destStateMachine = hexToString(order.destChain as HexString)
		const sourceConsensusStateId = this.source.config.getConsensusStateId(sourceStateMachine)
		const destConsensusStateId = this.dest.config.getConsensusStateId(destStateMachine)

		let latestHeight = 0n
		while (latestHeight <= order.deadline) {
			const { stateId } = parseStateMachineId(destStateMachine)

			latestHeight = await hyperbridge.latestStateMachineHeight({
				stateId,
				consensusStateId: destConsensusStateId,
			})

			yield {
				status: "AWAITING_DESTINATION_FINALIZED",
				data: { currentHeight: latestHeight, deadline: order.deadline },
			}

			if (latestHeight <= order.deadline) {
				await sleep(10000)
			}
		}

		yield { status: "DESTINATION_FINALIZED", data: { height: latestHeight } }

		const intentGatewayAddress = this.dest.config.getIntentGatewayAddress(destStateMachine)

		const orderId = orderCommitment(order)

		const slotHash = await this.dest.client.readContract({
			abi: IntentGatewayABI.ABI,
			address: intentGatewayAddress,
			functionName: "calculateCommitmentSlotHash",
			args: [orderId],
		})

		const proof = await this.dest.queryStateProof(latestHeight, [slotHash], intentGatewayAddress)

		const destIProof: IProof = {
			consensusStateId: destConsensusStateId,
			height: latestHeight,
			proof,
			stateMachine: destStateMachine,
		}

		this.destStateproofCache.set(order.id as HexString, destIProof)

		yield { status: "STATE_PROOF_RECEIVED", data: { proof, height: latestHeight } }

		const getRequest = (yield { status: "AWAITING_GET_REQUEST" }) as IGetRequest | undefined

		if (!getRequest) {
			throw new Error("[Cancel Order]: Get Request not provided")
		}

		const commitment = getRequestCommitment({
			...getRequest,
			keys: [...getRequest.keys],
		})

		const statusStream = indexerClient.getRequestStatusStream(commitment)

		for await (const statusUpdate of statusStream) {
			if (statusUpdate.status === RequestStatus.SOURCE_FINALIZED) {
				const sourceHeight = statusUpdate.metadata.blockNumber
				const proof = await this.source.queryProof(
					{ Requests: [commitment] },
					hyperbridgeConfig.stateMachineId,
					BigInt(sourceHeight),
				)

				yield { status: "SOURCE_PROOF_RECIEVED", data: proof }

				const { stateId } = parseStateMachineId(sourceStateMachine)
				const sourceIProof: IProof = {
					height: BigInt(sourceHeight),
					stateMachine: sourceStateMachine,
					consensusStateId: sourceConsensusStateId,
					proof,
				}
				const getRequestMessage: IGetRequestMessage = {
					kind: "GetRequest",
					requests: [getRequest],
					source: sourceIProof,
					response: this.getCachedProof(order.id as HexString)!,
					signer: pad("0x"),
				}

				// wait for challenge period for the source chain
				await waitForChallengePeriod(hyperbridge, {
					height: BigInt(sourceHeight),
					id: {
						stateId,
						consensusStateId: sourceConsensusStateId,
					},
				})

				const receiptKey = hyperbridge.requestReceiptKey(commitment)

				const { api } = hyperbridge

				if (!api) {
					throw new Error("Hyperbridge API is not available")
				}

				let storageValue = await api.rpc.childstate.getStorage(":child_storage:default:ISMP", receiptKey)

				if (storageValue.isNone) {
					console.log("No receipt found. Attempting to submit...")
					try {
						await hyperbridge.submitUnsigned(getRequestMessage)
					} catch {
						console.warn("Submission failed. Awaiting network confirmation...")
					}

					console.log("Waiting for network state update...")
					await sleep(30000)

					storageValue = await retryPromise(
						async () => {
							const value = await api.rpc.childstate.getStorage(":child_storage:default:ISMP", receiptKey)

							if (value.isNone) {
								throw new Error("Receipt not found")
							}

							return value
						},
						{
							maxRetries: 10,
							backoffMs: 5000,
							logMessage: "Checking for receipt",
						},
					)
				}
				console.log("Receipt confirmed on Hyperbridge. Proceeding.")
			}

			yield statusUpdate
		}
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
