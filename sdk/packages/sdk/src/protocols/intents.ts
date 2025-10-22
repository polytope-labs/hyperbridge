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
	UniversalRouterCommands,
	maxBigInt,
	getGasPriceFromEtherscan,
	USE_ETHERSCAN_CHAINS,
} from "@/utils"
import {
	encodeFunctionData,
	formatUnits,
	hexToString,
	maxUint256,
	pad,
	parseUnits,
	toHex,
	encodePacked,
	encodeAbiParameters,
	parseAbiParameters,
	erc20Abi,
} from "viem"
import {
	DispatchPost,
	IGetRequest,
	IHyperbridgeConfig,
	RequestStatus,
	type FillOptions,
	type HexString,
	type IPostRequest,
	type Order,
	type Transaction,
} from "@/types"
import IntentGatewayABI from "@/abis/IntentGateway"
import UniswapRouterV2 from "@/abis/uniswapRouterV2"
import UniswapV3Quoter from "@/abis/uniswapV3Quoter"
import { UNISWAP_V4_QUOTER_ABI } from "@/abis/uniswapV4Quoter"
import UniswapV2Pair from "@/abis/uniswapV2Pair"
import type { EvmChain } from "@/chains/evm"
import { Decimal } from "decimal.js"
import { getChain, IGetRequestMessage, IProof, requestCommitmentKey, SubstrateChain } from "@/chain"
import { IndexerClient } from "@/client"
import { PERMIT2_ABI } from "@/abis/permit2"
import universalRouter from "@/abis/universalRouter"

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

		const { decimals: sourceChainFeeTokenDecimals } = await this.source.getFeeTokenWithDecimals()

		const { address: destChainFeeTokenAddress, decimals: destChainFeeTokenDecimals } =
			await this.dest.getFeeTokenWithDecimals()

		const { gas: postGasEstimate, postRequestCalldata } = await this.source.estimateGas(postRequest)

		const postGasEstimateInSourceFeeToken = await this.convertGasToFeeToken(
			postGasEstimate,
			"source",
			order.sourceChain,
		)

		const minRelayerFee = 5n * 10n ** BigInt(sourceChainFeeTokenDecimals - 2)
		const postGasWithIncentive = postGasEstimateInSourceFeeToken + (postGasEstimateInSourceFeeToken * 1n) / 100n
		const relayerFeeInSourceFeeToken = maxBigInt(postGasWithIncentive, minRelayerFee)

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
		try {
			let protocolFeeInNativeToken = await this.quoteNative(postRequest, relayerFeeInDestFeeToken).catch(() =>
				this.dest.quoteNative(postRequest, relayerFeeInDestFeeToken).catch(() => 0n),
			)
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

		const fillGasInDestFeeToken = await this.convertGasToFeeToken(destChainFillGas, "dest", order.destChain)
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

		let totalNativeTokenAmount = await this.convertFeeTokenToNative(
			totalEstimateInSourceFeeToken,
			"source",
			order.sourceChain,
		)

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
	 * @param evmChainID - The EVM chain ID in format "EVM-{id}"
	 * @returns The fee token amount converted to native token amount
	 * @private
	 */
	private async convertFeeTokenToNative(
		feeTokenAmount: bigint,
		getQuoteIn: "source" | "dest",
		evmChainID: string,
	): Promise<bigint> {
		const client = this[getQuoteIn].client
		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = await this[getQuoteIn].getFeeTokenWithDecimals()

		try {
			const { amountOut } = await this.findBestProtocolWithAmountIn(
				getQuoteIn,
				feeToken.address,
				wethAsset,
				feeTokenAmount,
				evmChainID,
				{ selectedProtocol: "v2" },
			)

			if (amountOut === 0n) {
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = Number.parseInt(evmChainID.split("-")[1])
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
	 * @param evmChainID - The EVM chain ID in format "EVM-{id}"
	 * @returns The gas cost converted to fee token amount
	 * @private
	 */
	private async convertGasToFeeToken(
		gasEstimate: bigint,
		gasEstimateIn: "source" | "dest",
		evmChainID: string,
	): Promise<bigint> {
		const client = this[gasEstimateIn].client
		const useEtherscan = USE_ETHERSCAN_CHAINS.has(evmChainID)
		const etherscanApiKey = useEtherscan ? this[gasEstimateIn].config.getEtherscanApiKey() : undefined
		const gasPrice =
			useEtherscan && etherscanApiKey
				? await retryPromise(() => getGasPriceFromEtherscan(evmChainID, etherscanApiKey), {
						maxRetries: 3,
						backoffMs: 250,
					}).catch(async () => {
						console.warn({ evmChainID }, "Error getting gas price from etherscan, using client's gas price")
						return await client.getGasPrice()
					})
				: await client.getGasPrice()
		const gasCostInWei = gasEstimate * gasPrice
		const wethAddr = this[gasEstimateIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = await this[gasEstimateIn].getFeeTokenWithDecimals()

		try {
			const { amountOut } = await this.findBestProtocolWithAmountIn(
				gasEstimateIn,
				wethAddr,
				feeToken.address,
				gasCostInWei,
				evmChainID,
				{ selectedProtocol: "v2" },
			)
			if (amountOut === 0n) {
				console.log("Amount out not found")
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = Number.parseInt(evmChainID.split("-")[1])
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
	 * Gets V2 quote for exact output swap.
	 */
	async getV2QuoteWithAmountOut(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
	): Promise<bigint> {
		const client = this[getQuoteIn].client
		const v2Router = this[getQuoteIn].config.getUniswapRouterV2Address(evmChainID)

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		try {
			const v2AmountIn = await client.simulateContract({
				address: v2Router,
				abi: UniswapRouterV2.ABI,
				// @ts-ignore
				functionName: "getAmountsIn",
				// @ts-ignore
				args: [amountOut, [tokenInForQuote, tokenOutForQuote]],
			})

			return v2AmountIn.result[0]
		} catch {
			console.warn("V2 quote failed:")
			return maxUint256
		}
	}

	/**
	 * Gets V2 quote for exact input swap.
	 */
	async getV2QuoteWithAmountIn(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
	): Promise<bigint> {
		const client = this[getQuoteIn].client
		const v2Router = this[getQuoteIn].config.getUniswapRouterV2Address(evmChainID)

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		try {
			const v2AmountOut = await client.simulateContract({
				address: v2Router,
				abi: UniswapRouterV2.ABI,
				// @ts-ignore
				functionName: "getAmountsOut",
				// @ts-ignore
				args: [amountIn, [tokenInForQuote, tokenOutForQuote]],
			})

			return v2AmountOut.result[1]
		} catch {
			console.warn("V2 quote failed:")
			return BigInt(0)
		}
	}

	/**
	 * Gets V3 quote for exact output swap.
	 */
	async getV3QuoteWithAmountOut(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
	): Promise<{ amountIn: bigint; fee: number }> {
		const client = this[getQuoteIn].client
		const commonFees = [100, 500, 3000, 10000]
		let bestAmountIn = maxUint256
		let bestFee = 0

		const v3Quoter = this[getQuoteIn].config.getUniswapV3QuoterAddress(evmChainID)

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		for (const fee of commonFees) {
			try {
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

				if (amountIn < bestAmountIn) {
					bestAmountIn = amountIn
					bestFee = fee
				}
			} catch {
				console.warn(`V3 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		return { amountIn: bestAmountIn, fee: bestFee }
	}

	/**
	 * Gets V3 quote for exact input swap.
	 */
	async getV3QuoteWithAmountIn(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
	): Promise<{ amountOut: bigint; fee: number }> {
		const client = this[getQuoteIn].client
		const commonFees = [100, 500, 3000, 10000]
		let bestAmountOut = BigInt(0)
		let bestFee = 0

		const v3Quoter = this[getQuoteIn].config.getUniswapV3QuoterAddress(evmChainID)

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		for (const fee of commonFees) {
			try {
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

				if (amountOut > bestAmountOut) {
					bestAmountOut = amountOut
					bestFee = fee
				}
			} catch {
				console.warn(`V3 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		return { amountOut: bestAmountOut, fee: bestFee }
	}

	/**
	 * Gets V4 quote for exact output swap.
	 */
	async getV4QuoteWithAmountOut(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
	): Promise<{ amountIn: bigint; fee: number }> {
		const client = this[getQuoteIn].client
		const commonFees = [100, 500, 3000, 10000]
		let bestAmountIn = maxUint256
		let bestFee = 0

		const v4Quoter = this[getQuoteIn].config.getUniswapV4QuoterAddress(evmChainID)

		for (const fee of commonFees) {
			try {
				const currency0 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenIn : tokenOut
				const currency1 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenOut : tokenIn

				const zeroForOne = tokenIn.toLowerCase() === currency0.toLowerCase()

				const poolKey = {
					currency0: currency0,
					currency1: currency1,
					fee: fee,
					tickSpacing: this.getTickSpacing(fee),
					hooks: ADDRESS_ZERO,
				}

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
								hookData: "0x",
							},
						],
					})
				).result as [bigint, bigint]

				const amountIn = quoteResult[0]

				if (amountIn < bestAmountIn) {
					bestAmountIn = amountIn
					bestFee = fee
				}
			} catch {
				console.warn(`V4 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		return { amountIn: bestAmountIn, fee: bestFee }
	}

	/**
	 * Gets V4 quote for exact input swap.
	 */
	async getV4QuoteWithAmountIn(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
	): Promise<{ amountOut: bigint; fee: number }> {
		const client = this[getQuoteIn].client
		const commonFees = [100, 500, 3000, 10000]
		let bestAmountOut = BigInt(0)
		let bestFee = 0

		const v4Quoter = this[getQuoteIn].config.getUniswapV4QuoterAddress(evmChainID)

		for (const fee of commonFees) {
			try {
				const currency0 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenIn : tokenOut
				const currency1 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenOut : tokenIn

				const zeroForOne = tokenIn.toLowerCase() === currency0.toLowerCase()

				const poolKey = {
					currency0: currency0,
					currency1: currency1,
					fee: fee,
					tickSpacing: this.getTickSpacing(fee),
					hooks: ADDRESS_ZERO,
				}

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
								hookData: "0x",
							},
						],
					})
				).result as [bigint, bigint]

				const amountOut = quoteResult[0]

				if (amountOut > bestAmountOut) {
					bestAmountOut = amountOut
					bestFee = fee
				}
			} catch {
				console.warn(`V4 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		return { amountOut: bestAmountOut, fee: bestFee }
	}

	/**
	 * Creates transaction structure for V2 exact input swap, including ERC20 transfer if needed.
	 */
	createV2SwapCalldataExactIn(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountIn: bigint,
		amountOutMinimum: bigint,
		recipient: HexString,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
	): Transaction[] {
		if (sourceTokenAddress.toLowerCase() === targetTokenAddress.toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}

		const isPermit2 = false // Router constant for self

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const swapSourceAddress = sourceTokenAddress === ADDRESS_ZERO ? wethAsset : sourceTokenAddress
		const swapTargetAddress = targetTokenAddress === ADDRESS_ZERO ? wethAsset : targetTokenAddress

		const path = [swapSourceAddress, swapTargetAddress]

		const commands: number[] = []
		const inputs: HexString[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
					amountIn,
				]),
			)
		}

		commands.push(UniversalRouterCommands.V2_SWAP_EXACT_IN)
		inputs.push(
			encodeAbiParameters(
				parseAbiParameters(
					"address recipient, uint256 amountIn, uint256 amountOutMinimum, address[] path, bool isPermit2",
				),
				[
					targetTokenAddress === ADDRESS_ZERO
						? this[getQuoteIn].config.getUniversalRouterAddress(evmChainID)
						: recipient,
					amountIn,
					amountOutMinimum,
					path,
					isPermit2,
				],
			),
		)

		if (targetTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.UNWRAP_WETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					recipient,
					amountOutMinimum,
				]),
			)
		}

		const commandsEncoded = this.encodeCommands(commands)
		const executeData = encodeFunctionData({
			abi: universalRouter.ABI,
			functionName: "execute",
			args: [commandsEncoded, inputs],
		})

		const transactions: Transaction[] = []

		if (sourceTokenAddress !== ADDRESS_ZERO) {
			const transferData = encodeFunctionData({
				abi: erc20Abi,
				functionName: "transfer",
				args: [this[getQuoteIn].config.getUniversalRouterAddress(evmChainID), amountIn],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
			value: sourceTokenAddress === ADDRESS_ZERO ? amountIn : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V2 exact output swap, including ERC20 transfer if needed.
	 */
	createV2SwapCalldataExactOut(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		recipient: HexString,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
	): Transaction[] {
		if (sourceTokenAddress.toLowerCase() === targetTokenAddress.toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}
		const isPermit2 = false

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const swapSourceAddress = sourceTokenAddress === ADDRESS_ZERO ? wethAsset : sourceTokenAddress
		const swapTargetAddress = targetTokenAddress === ADDRESS_ZERO ? wethAsset : targetTokenAddress

		const path = [swapSourceAddress, swapTargetAddress]

		const commands: number[] = []
		const inputs: HexString[] = []
		const transactions: Transaction[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
					amountInMax,
				]),
			)
		}

		commands.push(UniversalRouterCommands.V2_SWAP_EXACT_OUT)
		inputs.push(
			encodeAbiParameters(
				parseAbiParameters(
					"address recipient, uint256 amountOut, uint256 amountInMax, address[] path, bool isPermit2",
				),
				[
					targetTokenAddress === ADDRESS_ZERO
						? this[getQuoteIn].config.getUniversalRouterAddress(evmChainID)
						: recipient,
					amountOut,
					amountInMax,
					path,
					isPermit2,
				],
			),
		)

		if (targetTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.UNWRAP_WETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [recipient, amountOut]),
			)
		}

		const commandsEncoded = this.encodeCommands(commands)
		const executeData = encodeFunctionData({
			abi: universalRouter.ABI,
			functionName: "execute",
			args: [commandsEncoded, inputs],
		})

		if (sourceTokenAddress !== ADDRESS_ZERO) {
			const transferData = encodeFunctionData({
				abi: erc20Abi,
				functionName: "transfer",
				args: [this[getQuoteIn].config.getUniversalRouterAddress(evmChainID), amountInMax],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
			value: sourceTokenAddress === ADDRESS_ZERO ? amountInMax : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V3 exact input swap, including ERC20 transfer if needed.
	 */
	createV3SwapCalldataExactIn(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountIn: bigint,
		amountOutMinimum: bigint,
		fee: number,
		recipient: HexString,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
	): Transaction[] {
		if (sourceTokenAddress.toLowerCase() === targetTokenAddress.toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}
		const isPermit2 = false

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const swapSourceAddress = sourceTokenAddress === ADDRESS_ZERO ? wethAsset : sourceTokenAddress
		const swapTargetAddress = targetTokenAddress === ADDRESS_ZERO ? wethAsset : targetTokenAddress

		const pathV3 = encodePacked(["address", "uint24", "address"], [swapSourceAddress, fee, swapTargetAddress])

		const commands: number[] = []
		const inputs: HexString[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
					amountIn,
				]),
			)
		}

		commands.push(UniversalRouterCommands.V3_SWAP_EXACT_IN)
		inputs.push(
			encodeAbiParameters(
				parseAbiParameters(
					"address recipient, uint256 amountIn, uint256 amountOutMinimum, bytes path, bool isPermit2",
				),
				[
					targetTokenAddress === ADDRESS_ZERO
						? this[getQuoteIn].config.getUniversalRouterAddress(evmChainID)
						: recipient,
					amountIn,
					amountOutMinimum,
					pathV3,
					isPermit2,
				],
			),
		)

		if (targetTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.UNWRAP_WETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					recipient,
					amountOutMinimum,
				]),
			)
		}

		const commandsEncoded = this.encodeCommands(commands)
		const executeData = encodeFunctionData({
			abi: universalRouter.ABI,
			functionName: "execute",
			args: [commandsEncoded, inputs],
		})

		const transactions: Transaction[] = []

		if (sourceTokenAddress !== ADDRESS_ZERO) {
			const transferData = encodeFunctionData({
				abi: erc20Abi,
				functionName: "transfer",
				args: [this[getQuoteIn].config.getUniversalRouterAddress(evmChainID), amountIn],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
			value: sourceTokenAddress === ADDRESS_ZERO ? amountIn : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V3 exact output swap, including ERC20 transfer if needed.
	 */
	createV3SwapCalldataExactOut(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		fee: number,
		recipient: HexString,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
	): Transaction[] {
		if (sourceTokenAddress.toLowerCase() === targetTokenAddress.toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}
		const isPermit2 = false

		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const swapSourceAddress = sourceTokenAddress === ADDRESS_ZERO ? wethAsset : sourceTokenAddress
		const swapTargetAddress = targetTokenAddress === ADDRESS_ZERO ? wethAsset : targetTokenAddress

		const pathV3 = encodePacked(["address", "uint24", "address"], [swapTargetAddress, fee, swapSourceAddress])

		const commands: number[] = []
		const inputs: HexString[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
					amountInMax,
				]),
			)
		}

		commands.push(UniversalRouterCommands.V3_SWAP_EXACT_OUT)
		inputs.push(
			encodeAbiParameters(
				parseAbiParameters(
					"address recipient, uint256 amountOut, uint256 amountInMax, bytes path, bool isPermit2",
				),
				[
					targetTokenAddress === ADDRESS_ZERO
						? this[getQuoteIn].config.getUniversalRouterAddress(evmChainID)
						: recipient,
					amountOut,
					amountInMax,
					pathV3,
					isPermit2,
				],
			),
		)

		if (targetTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.UNWRAP_WETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [recipient, amountOut]),
			)
		}

		const commandsEncoded = this.encodeCommands(commands)
		const executeData = encodeFunctionData({
			abi: universalRouter.ABI,
			functionName: "execute",
			args: [commandsEncoded, inputs],
		})

		const transactions: Transaction[] = []

		if (sourceTokenAddress !== ADDRESS_ZERO) {
			const transferData = encodeFunctionData({
				abi: erc20Abi,
				functionName: "transfer",
				args: [this[getQuoteIn].config.getUniversalRouterAddress(evmChainID), amountInMax],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
			value: sourceTokenAddress === ADDRESS_ZERO ? amountInMax : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V4 exact input swap, including Permit2 approvals for ERC20 tokens.
	 */
	createV4SwapCalldataExactIn(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountIn: bigint,
		amountOutMinimum: bigint,
		fee: number,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
	): Transaction[] {
		if (sourceTokenAddress.toLowerCase() === targetTokenAddress.toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}
		const currency0 =
			sourceTokenAddress.toLowerCase() < targetTokenAddress.toLowerCase()
				? sourceTokenAddress
				: targetTokenAddress
		const currency1 =
			sourceTokenAddress.toLowerCase() < targetTokenAddress.toLowerCase()
				? targetTokenAddress
				: sourceTokenAddress

		const zeroForOne = sourceTokenAddress.toLowerCase() === currency0.toLowerCase()

		const poolKey = {
			currency0: currency0,
			currency1: currency1,
			fee: fee,
			tickSpacing: this.getTickSpacing(fee),
			hooks: ADDRESS_ZERO,
		}

		const actions = encodePacked(
			["uint8", "uint8", "uint8"],
			[
				UniversalRouterCommands.V4_SWAP_EXACT_IN_SINGLE,
				UniversalRouterCommands.SETTLE_ALL,
				UniversalRouterCommands.TAKE_ALL,
			],
		)

		const swapParams = encodeAbiParameters(
			parseAbiParameters(
				"((address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks) poolKey, bool zeroForOne, uint128 amountIn, uint128 amountOutMinimum, bytes hookData)",
			),
			[
				{
					poolKey,
					zeroForOne,
					amountIn,
					amountOutMinimum,
					hookData: "0x",
				},
			],
		)

		const settleParams = encodeAbiParameters(parseAbiParameters("address currency, uint128 amount"), [
			sourceTokenAddress,
			amountIn,
		])

		const takeParams = encodeAbiParameters(parseAbiParameters("address currency, uint128 amount"), [
			targetTokenAddress,
			amountOutMinimum,
		])

		const params = [swapParams, settleParams, takeParams]

		const commands = encodePacked(["uint8"], [UniversalRouterCommands.V4_SWAP])
		const inputs = [encodeAbiParameters(parseAbiParameters("bytes actions, bytes[] params"), [actions, params])]

		const executeData = encodeFunctionData({
			abi: universalRouter.ABI,
			functionName: "execute",
			args: [commands, inputs],
		})

		const transactions: Transaction[] = []

		if (sourceTokenAddress !== ADDRESS_ZERO) {
			const approveToPermit2Data = encodeFunctionData({
				abi: erc20Abi,
				functionName: "approve",
				args: [this[getQuoteIn].config.getPermit2Address(evmChainID), amountIn],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: approveToPermit2Data,
			})

			const permit2ApprovalData = encodeFunctionData({
				abi: PERMIT2_ABI,
				functionName: "approve",
				args: [
					sourceTokenAddress,
					this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
					amountIn,
					281474976710655, // Max expiration
				],
			})

			transactions.push({
				to: this[getQuoteIn].config.getPermit2Address(evmChainID),
				value: 0n,
				data: permit2ApprovalData,
			})
		}

		transactions.push({
			to: this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
			value: sourceTokenAddress === ADDRESS_ZERO ? amountIn : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V4 exact output swap, including Permit2 approvals for ERC20 tokens.
	 */
	createV4SwapCalldataExactOut(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		fee: number,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
	): Transaction[] {
		if (sourceTokenAddress.toLowerCase() === targetTokenAddress.toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}
		const currency0 =
			sourceTokenAddress.toLowerCase() < targetTokenAddress.toLowerCase()
				? sourceTokenAddress
				: targetTokenAddress
		const currency1 =
			sourceTokenAddress.toLowerCase() < targetTokenAddress.toLowerCase()
				? targetTokenAddress
				: sourceTokenAddress

		const zeroForOne = sourceTokenAddress.toLowerCase() === currency0.toLowerCase()

		const poolKey = {
			currency0: currency0,
			currency1: currency1,
			fee: fee,
			tickSpacing: this.getTickSpacing(fee),
			hooks: ADDRESS_ZERO,
		}

		const actions = encodePacked(
			["uint8", "uint8", "uint8"],
			[
				UniversalRouterCommands.V4_SWAP_EXACT_OUT_SINGLE,
				UniversalRouterCommands.SETTLE_ALL,
				UniversalRouterCommands.TAKE_ALL,
			],
		)

		const swapParams = encodeAbiParameters(
			parseAbiParameters(
				"((address currency0, address currency1, uint24 fee, int24 tickSpacing, address hooks) poolKey, bool zeroForOne, uint128 amountOut, uint128 amountInMaximum, bytes hookData)",
			),
			[
				{
					poolKey,
					zeroForOne,
					amountOut,
					amountInMaximum: amountInMax,
					hookData: "0x",
				},
			],
		)

		const settleParams = encodeAbiParameters(parseAbiParameters("address currency, uint128 amount"), [
			sourceTokenAddress,
			amountInMax,
		])

		const takeParams = encodeAbiParameters(parseAbiParameters("address currency, uint128 amount"), [
			targetTokenAddress,
			amountOut,
		])

		const params = [swapParams, settleParams, takeParams]

		const commands = encodePacked(["uint8"], [UniversalRouterCommands.V4_SWAP])
		const inputs = [encodeAbiParameters(parseAbiParameters("bytes actions, bytes[] params"), [actions, params])]

		const executeData = encodeFunctionData({
			abi: universalRouter.ABI,
			functionName: "execute",
			args: [commands, inputs],
		})

		const transactions: Transaction[] = []

		if (sourceTokenAddress !== ADDRESS_ZERO) {
			const approveToPermit2Data = encodeFunctionData({
				abi: erc20Abi,
				functionName: "approve",
				args: [this[getQuoteIn].config.getPermit2Address(evmChainID), amountInMax],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: approveToPermit2Data,
			})

			const permit2ApprovalData = encodeFunctionData({
				abi: PERMIT2_ABI,
				functionName: "approve",
				args: [
					sourceTokenAddress,
					this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
					amountInMax,
					281474976710655, // Max expiration
				],
			})

			transactions.push({
				to: this[getQuoteIn].config.getPermit2Address(evmChainID),
				value: 0n,
				data: permit2ApprovalData,
			})
		}

		transactions.push({
			to: this[getQuoteIn].config.getUniversalRouterAddress(evmChainID),
			value: sourceTokenAddress === ADDRESS_ZERO ? amountInMax : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Finds the best Uniswap protocol (V2, V3, or V4) for swapping tokens given a desired output amount.
	 * Compares liquidity and pricing across different protocols and fee tiers.
	 *
	 * @param getQuoteIn - Whether to use "source" or "dest" chain for the swap
	 * @param tokenIn - The address of the input token
	 * @param tokenOut - The address of the output token
	 * @param amountOut - The desired output amount
	 * @returns Object containing the best protocol, required input amount, fee tier (for V3/V4), and transaction structure
	 */
	async findBestProtocolWithAmountOut(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
		options?: {
			selectedProtocol?: "v2" | "v3" | "v4"
			generateCalldata?: boolean
			recipient?: HexString
		},
	): Promise<{
		protocol: "v2" | "v3" | "v4" | null
		amountIn: bigint
		fee?: number
		transactions?: Transaction[]
	}> {
		if (options?.generateCalldata && !options?.recipient) {
			throw new Error("Recipient address is required when generating calldata")
		}

		if (options?.selectedProtocol) {
			if (options.selectedProtocol === "v2") {
				const amountInV2 = await this.getV2QuoteWithAmountOut(
					getQuoteIn,
					tokenIn,
					tokenOut,
					amountOut,
					evmChainID,
				)
				if (amountInV2 === maxUint256) {
					return { protocol: null, amountIn: maxUint256 }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV2SwapCalldataExactOut(
						tokenIn,
						tokenOut,
						amountOut,
						amountInV2,
						options.recipient!,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v2", amountIn: amountInV2, transactions }
			}

			if (options.selectedProtocol === "v3") {
				const { amountIn: amountInV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountOut(
					getQuoteIn,
					tokenIn,
					tokenOut,
					amountOut,
					evmChainID,
				)
				if (amountInV3 === maxUint256) {
					return { protocol: null, amountIn: maxUint256 }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV3SwapCalldataExactOut(
						tokenIn,
						tokenOut,
						amountOut,
						amountInV3,
						bestV3Fee,
						options.recipient!,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v3", amountIn: amountInV3, fee: bestV3Fee, transactions }
			}

			if (options.selectedProtocol === "v4") {
				const { amountIn: amountInV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountOut(
					getQuoteIn,
					tokenIn,
					tokenOut,
					amountOut,
					evmChainID,
				)
				if (amountInV4 === maxUint256) {
					return { protocol: null, amountIn: maxUint256 }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV4SwapCalldataExactOut(
						tokenIn,
						tokenOut,
						amountOut,
						amountInV4,
						bestV4Fee,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v4", amountIn: amountInV4, fee: bestV4Fee, transactions }
			}
		}

		// If no protocol is selected, query all protocols to find the best one
		const amountInV2 = await this.getV2QuoteWithAmountOut(getQuoteIn, tokenIn, tokenOut, amountOut, evmChainID)

		const { amountIn: amountInV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountOut(
			getQuoteIn,
			tokenIn,
			tokenOut,
			amountOut,
			evmChainID,
		)

		const { amountIn: amountInV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountOut(
			getQuoteIn,
			tokenIn,
			tokenOut,
			amountOut,
			evmChainID,
		)

		if (amountInV2 === maxUint256 && amountInV3 === maxUint256 && amountInV4 === maxUint256) {
			return {
				protocol: null,
				amountIn: maxUint256,
			}
		}

		// Prefer V4 when V4 is close to the best of V2/V3 (within thresholdBps)
		if (amountInV4 !== maxUint256) {
			const thresholdBps = 100n // 1%
			if (amountInV3 !== maxUint256 && this.isWithinThreshold(amountInV4, amountInV3, thresholdBps)) {
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV4SwapCalldataExactOut(
						tokenIn,
						tokenOut,
						amountOut,
						amountInV4,
						bestV4Fee,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v4", amountIn: amountInV4, fee: bestV4Fee, transactions }
			}
			if (amountInV2 !== maxUint256 && this.isWithinThreshold(amountInV4, amountInV2, thresholdBps)) {
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV4SwapCalldataExactOut(
						tokenIn,
						tokenOut,
						amountOut,
						amountInV4,
						bestV4Fee,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v4", amountIn: amountInV4, fee: bestV4Fee, transactions }
			}
		}

		const minAmount = [
			{ protocol: "v2" as const, amountIn: amountInV2 },
			{ protocol: "v3" as const, amountIn: amountInV3, fee: bestV3Fee },
			{ protocol: "v4" as const, amountIn: amountInV4, fee: bestV4Fee },
		].reduce((best, current) => (current.amountIn < best.amountIn ? current : best))

		let transactions: Transaction[] | undefined
		if (options?.generateCalldata) {
			if (minAmount.protocol === "v2") {
				transactions = this.createV2SwapCalldataExactOut(
					tokenIn,
					tokenOut,
					amountOut,
					amountInV2,
					options.recipient!,
					evmChainID,
					getQuoteIn,
				)
			} else if (minAmount.protocol === "v3") {
				transactions = this.createV3SwapCalldataExactOut(
					tokenIn,
					tokenOut,
					amountOut,
					amountInV3,
					bestV3Fee,
					options.recipient!,
					evmChainID,
					getQuoteIn,
				)
			} else {
				transactions = this.createV4SwapCalldataExactOut(
					tokenIn,
					tokenOut,
					amountOut,
					amountInV4,
					bestV4Fee,
					evmChainID,
					getQuoteIn,
				)
			}
		}

		if (minAmount.protocol === "v2") {
			return {
				protocol: "v2",
				amountIn: amountInV2,
				transactions,
			}
		} else if (minAmount.protocol === "v3") {
			return {
				protocol: "v3",
				amountIn: amountInV3,
				fee: bestV3Fee,
				transactions,
			}
		} else {
			return {
				protocol: "v4",
				amountIn: amountInV4,
				fee: bestV4Fee,
				transactions,
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
	 * @param evmChainID - The EVM chain ID in format "EVM-{id}"
	 * @param selectedProtocol - Optional specific protocol to use ("v2", "v3", or "v4")
	 * @returns Object containing the best protocol, expected output amount, fee tier (for V3/V4), and transaction structure
	 */
	async findBestProtocolWithAmountIn(
		getQuoteIn: "source" | "dest",
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
		options?: {
			selectedProtocol?: "v2" | "v3" | "v4"
			generateCalldata?: boolean
			recipient?: HexString
		},
	): Promise<{
		protocol: "v2" | "v3" | "v4" | null
		amountOut: bigint
		fee?: number
		transactions?: Transaction[]
	}> {
		if (options?.generateCalldata && !options?.recipient) {
			throw new Error("Recipient address is required when generating calldata")
		}

		if (options?.selectedProtocol) {
			if (options.selectedProtocol === "v2") {
				const amountOutV2 = await this.getV2QuoteWithAmountIn(
					getQuoteIn,
					tokenIn,
					tokenOut,
					amountIn,
					evmChainID,
				)
				if (amountOutV2 === BigInt(0)) {
					return { protocol: null, amountOut: BigInt(0) }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV2SwapCalldataExactIn(
						tokenIn,
						tokenOut,
						amountIn,
						amountOutV2,
						options.recipient!,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v2", amountOut: amountOutV2, transactions }
			}

			if (options.selectedProtocol === "v3") {
				const { amountOut: amountOutV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountIn(
					getQuoteIn,
					tokenIn,
					tokenOut,
					amountIn,
					evmChainID,
				)
				if (amountOutV3 === BigInt(0)) {
					return { protocol: null, amountOut: BigInt(0) }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV3SwapCalldataExactIn(
						tokenIn,
						tokenOut,
						amountIn,
						amountOutV3,
						bestV3Fee,
						options.recipient!,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v3", amountOut: amountOutV3, fee: bestV3Fee, transactions }
			}

			if (options.selectedProtocol === "v4") {
				const { amountOut: amountOutV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountIn(
					getQuoteIn,
					tokenIn,
					tokenOut,
					amountIn,
					evmChainID,
				)
				if (amountOutV4 === BigInt(0)) {
					return { protocol: null, amountOut: BigInt(0) }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV4SwapCalldataExactIn(
						tokenIn,
						tokenOut,
						amountIn,
						amountOutV4,
						bestV4Fee,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee, transactions }
			}
		}

		// If no protocol is selected, query all protocols to find the best one
		const amountOutV2 = await this.getV2QuoteWithAmountIn(getQuoteIn, tokenIn, tokenOut, amountIn, evmChainID)

		const { amountOut: amountOutV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountIn(
			getQuoteIn,
			tokenIn,
			tokenOut,
			amountIn,
			evmChainID,
		)

		const { amountOut: amountOutV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountIn(
			getQuoteIn,
			tokenIn,
			tokenOut,
			amountIn,
			evmChainID,
		)

		// If no liquidity found in any protocol
		if (amountOutV2 === BigInt(0) && amountOutV3 === BigInt(0) && amountOutV4 === BigInt(0)) {
			return {
				protocol: null,
				amountOut: BigInt(0),
			}
		}

		// Prefer V4 when V4 is close to the best of V2/V3 (within thresholdBps)
		if (amountOutV4 !== BigInt(0)) {
			const thresholdBps = 100n // 1%
			if (amountOutV3 !== BigInt(0) && this.isWithinThreshold(amountOutV4, amountOutV3, thresholdBps)) {
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV4SwapCalldataExactIn(
						tokenIn,
						tokenOut,
						amountIn,
						amountOutV4,
						bestV4Fee,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee, transactions }
			}
			if (amountOutV2 !== BigInt(0) && this.isWithinThreshold(amountOutV4, amountOutV2, thresholdBps)) {
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV4SwapCalldataExactIn(
						tokenIn,
						tokenOut,
						amountIn,
						amountOutV4,
						bestV4Fee,
						evmChainID,
						getQuoteIn,
					)
				}
				return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee, transactions }
			}
		}

		const maxAmount = [
			{ protocol: "v2" as const, amountOut: amountOutV2 },
			{ protocol: "v3" as const, amountOut: amountOutV3, fee: bestV3Fee },
			{ protocol: "v4" as const, amountOut: amountOutV4, fee: bestV4Fee },
		].reduce((best, current) => (current.amountOut > best.amountOut ? current : best))

		let transactions: Transaction[] | undefined
		if (options?.generateCalldata) {
			if (maxAmount.protocol === "v2") {
				transactions = this.createV2SwapCalldataExactIn(
					tokenIn,
					tokenOut,
					amountIn,
					amountOutV2,
					options.recipient!,
					evmChainID,
					getQuoteIn,
				)
			} else if (maxAmount.protocol === "v3") {
				transactions = this.createV3SwapCalldataExactIn(
					tokenIn,
					tokenOut,
					amountIn,
					amountOutV3,
					bestV3Fee,
					options.recipient!,
					evmChainID,
					getQuoteIn,
				)
			} else {
				transactions = this.createV4SwapCalldataExactIn(
					tokenIn,
					tokenOut,
					amountIn,
					amountOutV4,
					bestV4Fee,
					evmChainID,
					getQuoteIn,
				)
			}
		}

		if (maxAmount.protocol === "v2") {
			return {
				protocol: "v2",
				amountOut: amountOutV2,
				transactions,
			}
		} else if (maxAmount.protocol === "v3") {
			return {
				protocol: "v3",
				amountOut: amountOutV3,
				fee: bestV3Fee,
				transactions,
			}
		} else {
			return {
				protocol: "v4",
				amountOut: amountOutV4,
				fee: bestV4Fee,
				transactions,
			}
		}
	}

	async createMultiHopSwapThroughPair(
		dexPairAddress: HexString,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
		recipient: HexString,
		protocol: "v2" | "v3" = "v2",
	): Promise<{ finalAmountOut: bigint; calldata: Transaction[] }> {
		const client = this[getQuoteIn].client
		const wethAsset = this[getQuoteIn].config.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const calldispatcher = this[getQuoteIn].config.getCalldispatcherAddress(evmChainID)

		const [token0, token1] = await Promise.all([
			client.readContract({
				address: dexPairAddress,
				abi: UniswapV2Pair.ABI,
				functionName: "token0",
			}),
			client.readContract({
				address: dexPairAddress,
				abi: UniswapV2Pair.ABI,
				functionName: "token1",
			}),
		])

		const intermediateToken = tokenOut.toLowerCase() === token0.toLowerCase() ? token1 : token0

		const swapPath = this.buildSwapPath(tokenIn, tokenOut, intermediateToken, wethAsset, calldispatcher, recipient)

		return this.executeSwapPath(swapPath, amountIn, evmChainID, getQuoteIn, protocol)
	}

	private buildSwapPath(
		tokenIn: HexString,
		tokenOut: HexString,
		intermediateToken: HexString,
		wethAsset: HexString,
		calldispatcher: HexString,
		recipient: HexString,
	): SwapSegment[] {
		const normalize = (token: HexString) => token.toLowerCase()

		// Direct swap: tokenIn -> tokenOut (when intermediateToken === tokenIn)
		if (normalize(intermediateToken) === normalize(tokenIn)) {
			return [{ from: tokenIn, to: tokenOut, recipient }]
		}

		// Two-hop swap: tokenIn -> WETH -> tokenOut
		if (normalize(intermediateToken) === normalize(wethAsset)) {
			return [
				{ from: tokenIn, to: wethAsset, recipient: calldispatcher },
				{ from: wethAsset, to: tokenOut, recipient },
			]
		}

		// Three-hop swap: tokenIn -> WETH -> intermediateToken -> tokenOut
		return [
			{ from: tokenIn, to: wethAsset, recipient: calldispatcher },
			{ from: wethAsset, to: intermediateToken, recipient: calldispatcher },
			{ from: intermediateToken, to: tokenOut, recipient },
		]
	}

	private async executeSwapPath(
		path: SwapSegment[],
		initialAmount: bigint,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
		protocol: "v2" | "v3",
	): Promise<{ finalAmountOut: bigint; calldata: Transaction[] }> {
		let currentAmount = initialAmount
		const calldata: Transaction[] = []

		for (let i = 0; i < path.length; i++) {
			const segment = path[i]
			// Using 0.5% slippage for all swaps except the last one, which gets 1%
			const isLastSwap = i === path.length - 1
			const slippage = isLastSwap && path.length > 1 ? 990n : 995n

			const swapResult = await this.createSwapCalldata(
				protocol,
				segment.from,
				segment.to,
				currentAmount,
				segment.recipient,
				evmChainID,
				getQuoteIn,
				slippage,
			)

			currentAmount = swapResult.amountOut
			calldata.push(...swapResult.calldata)
		}

		return {
			finalAmountOut: currentAmount,
			calldata,
		}
	}

	private async createSwapCalldata(
		protocol: "v2" | "v3",
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		recipient: HexString,
		evmChainID: string,
		getQuoteIn: "source" | "dest",
		slippageFactor: bigint,
	): Promise<{ amountOut: bigint; calldata: Transaction[] }> {
		if (protocol === "v2") {
			let amountOut = await this.getV2QuoteWithAmountIn(getQuoteIn, tokenIn, tokenOut, amountIn, evmChainID)
			amountOut = (amountOut * slippageFactor) / 1000n

			return {
				amountOut,
				calldata: this.createV2SwapCalldataExactIn(
					tokenIn,
					tokenOut,
					amountIn,
					amountOut,
					recipient,
					evmChainID,
					getQuoteIn,
				),
			}
		} else {
			const { amountOut, fee } = await this.getV3QuoteWithAmountIn(
				getQuoteIn,
				tokenIn,
				tokenOut,
				amountIn,
				evmChainID,
			)
			const adjustedAmountOut = (amountOut * slippageFactor) / 1000n

			return {
				amountOut: adjustedAmountOut,
				calldata: this.createV3SwapCalldataExactIn(
					tokenIn,
					tokenOut,
					amountIn,
					adjustedAmountOut,
					fee,
					recipient,
					evmChainID,
					getQuoteIn,
				),
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

	async submitAndConfirmReceipt(hyperbridge: SubstrateChain, commitment: HexString, message: IGetRequestMessage) {
		let storageValue = await hyperbridge.queryRequestReceipt(commitment)

		if (!storageValue) {
			console.log("No receipt found. Attempting to submit...")
			try {
				await hyperbridge.submitUnsigned(message)
			} catch {
				console.warn("Submission failed. Awaiting network confirmation...")
			}

			console.log("Waiting for network state update...")
			await sleep(30000)

			storageValue = await retryPromise(
				async () => {
					const value = await hyperbridge.queryRequestReceipt(commitment)
					if (!value) throw new Error("Receipt not found")
					return value
				},
				{ maxRetries: 10, backoffMs: 5000, logMessage: "Checking for receipt" },
			)
		}

		console.log("Hyperbridge Receipt confirmed.")
	}

	async *cancelOrder(
		order: Order,
		hyperbridgeConfig: IHyperbridgeConfig,
		indexerClient: IndexerClient,
		storedData?: StoredCancellationData,
	) {
		const hyperbridge = (await getChain({ ...hyperbridgeConfig, hasher: "Keccak" })) as SubstrateChain
		const sourceStateMachine = hexToString(order.sourceChain as HexString)
		const destStateMachine = hexToString(order.destChain as HexString)

		const sourceConsensusStateId = this.source.config.getConsensusStateId(sourceStateMachine)
		const destConsensusStateId = this.dest.config.getConsensusStateId(destStateMachine)

		let destIProof: IProof

		if (storedData?.destIProof) {
			destIProof = storedData.destIProof
			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		} else {
			let latestHeight = 0n
			let lastFailedHeight: bigint | null = null
			let proofHex: HexString | null = null

			while (!proofHex) {
				const height = await retryPromise(
					() =>
						indexerClient.queryLatestStateMachineHeight({
							statemachineId: destStateMachine,
							chain: hyperbridgeConfig.stateMachineId,
						}),
					{ maxRetries: 5, backoffMs: 500, logMessage: "Failed to fetch latest state machine height" },
				)

				if (!height) {
					throw new Error("No state machine updates found for destination chain")
				}

				latestHeight = height

				const shouldFetchProof =
					lastFailedHeight === null ? latestHeight > order.deadline : latestHeight > lastFailedHeight

				if (!shouldFetchProof) {
					yield {
						status: "AWAITING_DESTINATION_FINALIZED",
						data: {
							currentHeight: latestHeight,
							deadline: order.deadline,
							...(lastFailedHeight && { lastFailedHeight }),
						},
					}
					await sleep(10000)
					continue
				}

				try {
					const intentGatewayAddress = this.dest.config.getIntentGatewayAddress(destStateMachine)
					const orderId = orderCommitment(order)
					const slotHash = await this.dest.client.readContract({
						abi: IntentGatewayABI.ABI,
						address: intentGatewayAddress,
						functionName: "calculateCommitmentSlotHash",
						args: [orderId],
					})
					proofHex = await this.dest.queryStateProof(latestHeight, [slotHash], intentGatewayAddress)
				} catch (error) {
					lastFailedHeight = latestHeight
					yield {
						status: "PROOF_FETCH_FAILED",
						data: {
							failedHeight: latestHeight,
							error: error instanceof Error ? error.message : String(error),
							deadline: order.deadline,
						},
					}
					await sleep(10000)
				}
			}

			destIProof = {
				consensusStateId: destConsensusStateId,
				height: latestHeight,
				proof: proofHex,
				stateMachine: destStateMachine,
			}

			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		}

		const getRequest = storedData?.getRequest ?? ((yield { status: "AWAITING_GET_REQUEST" }) as IGetRequest)
		if (!getRequest) throw new Error("[Cancel Order]: Get Request not provided")

		const commitment = getRequestCommitment({ ...getRequest, keys: [...getRequest.keys] })

		const sourceStatusStream = indexerClient.getRequestStatusStream(commitment)
		for await (const statusUpdate of sourceStatusStream) {
			yield statusUpdate

			if (statusUpdate.status !== RequestStatus.SOURCE_FINALIZED) {
				continue
			}

			let sourceHeight = BigInt(statusUpdate.metadata.blockNumber)
			let proof: HexString | undefined
			// Check if request was delivered while waiting for proof
			const checkIfAlreadyDelivered = async () => {
				const currentStatus = await indexerClient.queryGetRequestWithStatus(commitment)
				return (
					currentStatus?.statuses.some((status) => status.status === RequestStatus.HYPERBRIDGE_DELIVERED) ??
					false
				)
			}

			const { slot1, slot2 } = requestCommitmentKey(commitment)

			while (true) {
				try {
					proof = await this.source.queryStateProof(sourceHeight, [slot1, slot2])
					break
				} catch {
					const failedHeight = sourceHeight
					while (sourceHeight <= failedHeight) {
						if (await checkIfAlreadyDelivered()) {
							break
						}

						const nextHeight = await retryPromise(
							() =>
								indexerClient.queryLatestStateMachineHeight({
									statemachineId: sourceStateMachine,
									chain: hyperbridgeConfig.stateMachineId,
								}),
							{
								maxRetries: 5,
								backoffMs: 5000,
								logMessage: "Failed to fetch latest state machine height (post-source-proof failure)",
							},
						)

						if (!nextHeight) {
							throw new Error(
								`No state machine updates found for ${sourceStateMachine} on chain ${hyperbridgeConfig.stateMachineId}`,
							)
						}

						if (nextHeight <= failedHeight) {
							await sleep(10000)
							continue
						}

						sourceHeight = nextHeight
					}

					if (await checkIfAlreadyDelivered()) {
						break
					}
				}
			}

			if (proof) {
				const sourceIProof: IProof = {
					height: sourceHeight,
					stateMachine: sourceStateMachine,
					consensusStateId: sourceConsensusStateId,
					proof,
				}

				yield { status: "SOURCE_PROOF_RECEIVED", data: sourceIProof }

				const getRequestMessage: IGetRequestMessage = {
					kind: "GetRequest",
					requests: [getRequest],
					source: sourceIProof,
					response: destIProof,
					signer: pad("0x"),
				}

				await waitForChallengePeriod(hyperbridge, {
					height: sourceHeight,
					id: {
						stateId: parseStateMachineId(sourceStateMachine).stateId,
						consensusStateId: sourceConsensusStateId,
					},
				})

				await this.submitAndConfirmReceipt(hyperbridge, commitment, getRequestMessage)
			}
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

	/**
	 * Encodes multiple command bytes into packed format
	 * @private
	 */
	private encodeCommands(commands: number[]): HexString {
		if (commands.length === 0) {
			throw new Error("Commands array cannot be empty")
		}

		// Build the type array and ensure proper typing
		const types = Array(commands.length).fill("uint8")

		// Use type assertion for viem's strict typing
		return encodePacked(types as any, commands as any)
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

interface StoredCancellationData {
	destIProof?: IProof
	getRequest?: IGetRequest
	sourceIProof?: IProof
}
interface SwapSegment {
	from: HexString
	to: HexString
	recipient: HexString
}
