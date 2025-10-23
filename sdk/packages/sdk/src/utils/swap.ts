import {
	PublicClient,
	maxUint256,
	encodeAbiParameters,
	parseAbiParameters,
	parseAbiItem,
	encodePacked,
	encodeFunctionData,
	erc20Abi,
} from "viem"
import { ADDRESS_ZERO, ChainConfigService, HexString, Transaction } from ".."
import UniswapRouterV2 from "@/abis/uniswapRouterV2"
import UniswapV3Quoter from "@/abis/uniswapV3Quoter"
import { UNISWAP_V4_QUOTER_ABI } from "@/abis/uniswapV4Quoter"
import universalRouter from "@/abis/universalRouter"
import { UniversalRouterCommands } from "@/utils"
import { PERMIT2_ABI } from "@/abis/permit2"
import { popularTokens } from "@/configs/chain"

const COMMON_FEE_TIERS = [100, 500, 2500, 3000, 10000]

export class Swap {
	private chainConfigService: ChainConfigService
	constructor() {
		this.chainConfigService = new ChainConfigService()
	}
	/**
	 * Gets V2 quote for exact output swap.
	 */
	async getV2QuoteWithAmountOut(
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
	): Promise<bigint> {
		const v2Router = this.chainConfigService.getUniswapRouterV2Address(evmChainID)

		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
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
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
	): Promise<bigint> {
		const v2Router = this.chainConfigService.getUniswapRouterV2Address(evmChainID)

		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
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
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
	): Promise<{ amountIn: bigint; fee: number }> {
		let bestAmountIn = maxUint256
		let bestFee = 0

		const v3Quoter = this.chainConfigService.getUniswapV3QuoterAddress(evmChainID)

		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		for (const fee of COMMON_FEE_TIERS) {
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
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
	): Promise<{ amountOut: bigint; fee: number }> {
		let bestAmountOut = BigInt(0)
		let bestFee = 0

		const v3Quoter = this.chainConfigService.getUniswapV3QuoterAddress(evmChainID)

		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		for (const fee of COMMON_FEE_TIERS) {
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
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		evmChainID: string,
	): Promise<{ amountIn: bigint; fee: number }> {
		let bestAmountIn = maxUint256
		let bestFee = 0

		const v4Quoter = this.chainConfigService.getUniswapV4QuoterAddress(evmChainID)

		for (const fee of COMMON_FEE_TIERS) {
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
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
	): Promise<{ amountOut: bigint; fee: number }> {
		let bestAmountOut = BigInt(0)
		let bestFee = 0

		const v4Quoter = this.chainConfigService.getUniswapV4QuoterAddress(evmChainID)

		for (const fee of COMMON_FEE_TIERS) {
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
	 * Supports both single-hop and multi-hop swaps.
	 */
	createV2SwapCalldataExactIn(
		path: HexString[],
		amountIn: bigint,
		amountOutMinimum: bigint,
		recipient: HexString,
		evmChainID: string,
	): Transaction[] {
		if (path.length < 2) {
			throw new Error("Path must contain at least 2 tokens")
		}

		if (path[0].toLowerCase() === path[path.length - 1].toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}

		const isPermit2 = false
		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const universalRouterAddress = this.chainConfigService.getUniversalRouterAddress(evmChainID)

		const swapPath = path.map((token) => (token === ADDRESS_ZERO ? wethAsset : token))

		const sourceTokenAddress = path[0]
		const targetTokenAddress = path[path.length - 1]

		const commands: number[] = []
		const inputs: HexString[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					universalRouterAddress,
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
					targetTokenAddress === ADDRESS_ZERO ? universalRouterAddress : recipient,
					amountIn,
					amountOutMinimum,
					swapPath,
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
				args: [universalRouterAddress, amountIn],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: universalRouterAddress,
			value: sourceTokenAddress === ADDRESS_ZERO ? amountIn : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V2 exact output swap, including ERC20 transfer if needed.
	 * Supports both single-hop and multi-hop swaps.
	 */
	createV2SwapCalldataExactOut(
		path: HexString[],
		amountOut: bigint,
		amountInMax: bigint,
		recipient: HexString,
		evmChainID: string,
	): Transaction[] {
		if (path.length < 2) {
			throw new Error("Path must contain at least 2 tokens")
		}

		if (path[0].toLowerCase() === path[path.length - 1].toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}

		const isPermit2 = false
		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const universalRouterAddress = this.chainConfigService.getUniversalRouterAddress(evmChainID)

		// Convert ADDRESS_ZERO to WETH in path
		const swapPath = path.map((token) => (token === ADDRESS_ZERO ? wethAsset : token))

		const sourceTokenAddress = path[0]
		const targetTokenAddress = path[path.length - 1]

		const commands: number[] = []
		const inputs: HexString[] = []
		const transactions: Transaction[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					universalRouterAddress,
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
					targetTokenAddress === ADDRESS_ZERO ? universalRouterAddress : recipient,
					amountOut,
					amountInMax,
					swapPath,
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
				args: [universalRouterAddress, amountInMax],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: universalRouterAddress,
			value: sourceTokenAddress === ADDRESS_ZERO ? amountInMax : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V3 exact input swap, including ERC20 transfer if needed.
	 * Supports both single-hop and multi-hop swaps.
	 */
	createV3SwapCalldataExactIn(
		path: HexString[],
		amountIn: bigint,
		amountOutMinimum: bigint,
		fees: number[],
		recipient: HexString,
		evmChainID: string,
	): Transaction[] {
		if (path.length < 2) {
			throw new Error("Path must contain at least 2 tokens")
		}

		if (path[0].toLowerCase() === path[path.length - 1].toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}

		if (fees.length !== path.length - 1) {
			throw new Error("Fees array length must be one less than path length")
		}

		const isPermit2 = false
		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const universalRouterAddress = this.chainConfigService.getUniversalRouterAddress(evmChainID)

		// Build path elements with alternating tokens and fees
		const pathElements: Array<string | number> = []
		for (let i = 0; i < path.length; i++) {
			const token = path[i] === ADDRESS_ZERO ? wethAsset : path[i]
			pathElements.push(token)

			if (i < path.length - 1) {
				pathElements.push(fees[i])
			}
		}

		const types: string[] = []
		for (let i = 0; i < pathElements.length; i++) {
			types.push(i % 2 === 0 ? "address" : "uint24")
		}

		const pathV3 = encodePacked(types, pathElements)

		const sourceTokenAddress = path[0]
		const targetTokenAddress = path[path.length - 1]

		const commands: number[] = []
		const inputs: HexString[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					universalRouterAddress,
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
					targetTokenAddress === ADDRESS_ZERO ? universalRouterAddress : recipient,
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
				args: [universalRouterAddress, amountIn],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: universalRouterAddress,
			value: sourceTokenAddress === ADDRESS_ZERO ? amountIn : 0n,
			data: executeData,
		})

		return transactions
	}

	/**
	 * Creates transaction structure for V3 exact output swap, including ERC20 transfer if needed.
	 * Supports both single-hop and multi-hop swaps.
	 */
	createV3SwapCalldataExactOut(
		path: HexString[],
		amountOut: bigint,
		amountInMax: bigint,
		fees: number[],
		recipient: HexString,
		evmChainID: string,
	): Transaction[] {
		if (path.length < 2) {
			throw new Error("Path must contain at least 2 tokens")
		}

		if (path[0].toLowerCase() === path[path.length - 1].toLowerCase()) {
			throw new Error("Source and target tokens cannot be the same")
		}

		if (fees.length !== path.length - 1) {
			throw new Error("Fees array length must be one less than path length")
		}

		const isPermit2 = false
		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const universalRouterAddress = this.chainConfigService.getUniversalRouterAddress(evmChainID)

		// Build path elements with alternating tokens and fees (reversed for exact output)
		const pathElements: Array<string | number> = []
		for (let i = path.length - 1; i >= 0; i--) {
			const token = path[i] === ADDRESS_ZERO ? wethAsset : path[i]
			pathElements.push(token)

			if (i > 0) {
				pathElements.push(fees[i - 1])
			}
		}

		const types: string[] = []
		for (let i = 0; i < pathElements.length; i++) {
			types.push(i % 2 === 0 ? "address" : "uint24")
		}

		const pathV3 = encodePacked(types, pathElements)

		const sourceTokenAddress = path[0]
		const targetTokenAddress = path[path.length - 1]

		const commands: number[] = []
		const inputs: HexString[] = []

		if (sourceTokenAddress === ADDRESS_ZERO) {
			commands.push(UniversalRouterCommands.WRAP_ETH)
			inputs.push(
				encodeAbiParameters(parseAbiParameters("address recipient, uint256 amountMin"), [
					universalRouterAddress,
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
					targetTokenAddress === ADDRESS_ZERO ? universalRouterAddress : recipient,
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
				args: [universalRouterAddress, amountInMax],
			})

			transactions.push({
				to: sourceTokenAddress,
				value: 0n,
				data: transferData,
			})
		}

		transactions.push({
			to: universalRouterAddress,
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
				args: [this.chainConfigService.getPermit2Address(evmChainID), amountIn],
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
					this.chainConfigService.getUniversalRouterAddress(evmChainID),
					amountIn,
					281474976710655, // Max expiration
				],
			})

			transactions.push({
				to: this.chainConfigService.getPermit2Address(evmChainID),
				value: 0n,
				data: permit2ApprovalData,
			})
		}

		transactions.push({
			to: this.chainConfigService.getUniversalRouterAddress(evmChainID),
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
				args: [this.chainConfigService.getPermit2Address(evmChainID), amountInMax],
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
					this.chainConfigService.getUniversalRouterAddress(evmChainID),
					amountInMax,
					281474976710655, // Max expiration
				],
			})

			transactions.push({
				to: this.chainConfigService.getPermit2Address(evmChainID),
				value: 0n,
				data: permit2ApprovalData,
			})
		}

		transactions.push({
			to: this.chainConfigService.getUniversalRouterAddress(evmChainID),
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
		client: PublicClient,
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
				const amountInV2 = await this.getV2QuoteWithAmountOut(client, tokenIn, tokenOut, amountOut, evmChainID)
				if (amountInV2 === maxUint256) {
					return { protocol: null, amountIn: maxUint256 }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV2SwapCalldataExactOut(
						[tokenIn, tokenOut],
						amountOut,
						amountInV2,
						options.recipient!,
						evmChainID,
					)
				}
				return { protocol: "v2", amountIn: amountInV2, transactions }
			}

			if (options.selectedProtocol === "v3") {
				const { amountIn: amountInV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountOut(
					client,
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
						[tokenIn, tokenOut],
						amountOut,
						amountInV3,
						[bestV3Fee],
						options.recipient!,
						evmChainID,
					)
				}
				return { protocol: "v3", amountIn: amountInV3, fee: bestV3Fee, transactions }
			}

			if (options.selectedProtocol === "v4") {
				const { amountIn: amountInV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountOut(
					client,
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
					)
				}
				return { protocol: "v4", amountIn: amountInV4, fee: bestV4Fee, transactions }
			}
		}

		// If no protocol is selected, query all protocols to find the best one
		const amountInV2 = await this.getV2QuoteWithAmountOut(client, tokenIn, tokenOut, amountOut, evmChainID)

		const { amountIn: amountInV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountOut(
			client,
			tokenIn,
			tokenOut,
			amountOut,
			evmChainID,
		)

		const { amountIn: amountInV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountOut(
			client,
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
					[tokenIn, tokenOut],
					amountOut,
					amountInV2,
					options.recipient!,
					evmChainID,
				)
			} else if (minAmount.protocol === "v3") {
				transactions = this.createV3SwapCalldataExactOut(
					[tokenIn, tokenOut],
					amountOut,
					amountInV3,
					[bestV3Fee],
					options.recipient!,
					evmChainID,
				)
			} else {
				transactions = this.createV4SwapCalldataExactOut(
					tokenIn,
					tokenOut,
					amountOut,
					amountInV4,
					bestV4Fee,
					evmChainID,
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
		client: PublicClient,
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
				const amountOutV2 = await this.getV2QuoteWithAmountIn(client, tokenIn, tokenOut, amountIn, evmChainID)
				if (amountOutV2 === BigInt(0)) {
					return { protocol: null, amountOut: BigInt(0) }
				}
				let transactions: Transaction[] | undefined
				if (options?.generateCalldata) {
					transactions = this.createV2SwapCalldataExactIn(
						[tokenIn, tokenOut],
						amountIn,
						amountOutV2,
						options.recipient!,
						evmChainID,
					)
				}
				return { protocol: "v2", amountOut: amountOutV2, transactions }
			}

			if (options.selectedProtocol === "v3") {
				const { amountOut: amountOutV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountIn(
					client,
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
						[tokenIn, tokenOut],
						amountIn,
						amountOutV3,
						[bestV3Fee],
						options.recipient!,
						evmChainID,
					)
				}
				return { protocol: "v3", amountOut: amountOutV3, fee: bestV3Fee, transactions }
			}

			if (options.selectedProtocol === "v4") {
				const { amountOut: amountOutV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountIn(
					client,
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
					)
				}
				return { protocol: "v4", amountOut: amountOutV4, fee: bestV4Fee, transactions }
			}
		}

		// If no protocol is selected, query all protocols to find the best one
		const amountOutV2 = await this.getV2QuoteWithAmountIn(client, tokenIn, tokenOut, amountIn, evmChainID)

		const { amountOut: amountOutV3, fee: bestV3Fee } = await this.getV3QuoteWithAmountIn(
			client,
			tokenIn,
			tokenOut,
			amountIn,
			evmChainID,
		)

		const { amountOut: amountOutV4, fee: bestV4Fee } = await this.getV4QuoteWithAmountIn(
			client,
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
					[tokenIn, tokenOut],
					amountIn,
					amountOutV2,
					options.recipient!,
					evmChainID,
				)
			} else if (maxAmount.protocol === "v3") {
				transactions = this.createV3SwapCalldataExactIn(
					[tokenIn, tokenOut],
					amountIn,
					amountOutV3,
					[bestV3Fee],
					options.recipient!,
					evmChainID,
				)
			} else {
				transactions = this.createV4SwapCalldataExactIn(
					tokenIn,
					tokenOut,
					amountIn,
					amountOutV4,
					bestV4Fee,
					evmChainID,
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

	/**
	 * Finds the best pair for multi-hop swaps based on popular tokens and liquidity.
	 * Prefers pairs with tokenIn as intermediate token, then returns the pair with highest liquidity.
	 */
	async findPair(
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		evmChainID: string,
		protocol?: "v2" | "v3",
	): Promise<{ pairAddress: HexString; intermediateToken: HexString }> {
		const chainPopularTokens = popularTokens[evmChainID as keyof typeof popularTokens]

		if (chainPopularTokens.length === 0) {
			throw new Error(`No suitable intermediate tokens found for chain ${evmChainID}`)
		}

		const pairCandidates: Array<{
			pairAddress: HexString
			token0: HexString
			token1: HexString
			liquidity: bigint
			hasTokenIn: boolean
			fee?: number
		}> = []

		for (const intermediateToken of chainPopularTokens) {
			try {
				const pair = await this.getPairAddress(
					client,
					intermediateToken as HexString,
					tokenOut,
					evmChainID,
					protocol,
				)

				if (pair.poolAddress && pair.poolAddress !== ADDRESS_ZERO) {
					let liquidity = 0n

					if (protocol === "v2") {
						const reserves = await client.readContract({
							address: pair.poolAddress,
							abi: [parseAbiItem("function getReserves() view returns (uint112, uint112, uint32)")],
							functionName: "getReserves",
						})

						liquidity = BigInt(reserves[0]) + BigInt(reserves[1])
					} else if (protocol === "v3") {
						liquidity = await client.readContract({
							address: pair.poolAddress,
							abi: [parseAbiItem("function liquidity() view returns (uint128)")],
							functionName: "liquidity",
						})
					}

					const hasTokenIn = intermediateToken.toLowerCase() === tokenIn.toLowerCase()

					pairCandidates.push({
						pairAddress: pair.poolAddress,
						token0: intermediateToken as HexString,
						token1: tokenOut,
						liquidity,
						hasTokenIn,
						fee: pair.fee,
					})
				}
			} catch {
				continue
			}
		}

		if (pairCandidates.length === 0) {
			throw new Error(`No valid pairs found for chain ${evmChainID}`)
		}

		pairCandidates.sort((a, b) => {
			if (a.hasTokenIn && !b.hasTokenIn) return -1
			if (!a.hasTokenIn && b.hasTokenIn) return 1
			return b.liquidity > a.liquidity ? 1 : -1
		})

		return {
			pairAddress: pairCandidates[0].pairAddress,
			intermediateToken: pairCandidates[0].token0,
		}
	}

	/**
	 * Gets pair address for V2 or V3 based on protocol parameter
	 * For V3, returns both pool address and fee
	 */
	private async getPairAddress(
		client: PublicClient,
		tokenA: HexString,
		tokenB: HexString,
		evmChainID: string,
		protocolParam?: "v2" | "v3",
	): Promise<{ poolAddress: HexString; fee?: number }> {
		const factoryAddress = this.chainConfigService.getUniswapV2FactoryAddress(evmChainID)
		if (protocolParam === "v2") {
			const poolAddress = await client.readContract({
				address: factoryAddress,
				abi: [parseAbiItem("function getPair(address tokenA, address tokenB) view returns (address pair)")],
				functionName: "getPair",
				args: [tokenA, tokenB],
			})
			return { poolAddress }
		}
		if (protocolParam === "v3") {
			return await this.getBestV3Pool(client, tokenA, tokenB, evmChainID)
		}
		throw new Error(`Invalid protocol parameter: ${protocolParam}`)
	}

	/**
	 * Gets the best V3 pool based on liquidity across different fee tiers
	 */
	private async getBestV3Pool(
		client: PublicClient,
		token0: HexString,
		token1: HexString,
		evmChainID: string,
	): Promise<{ poolAddress: HexString; fee: number }> {
		const factoryAddress = this.chainConfigService.getUniswapV3FactoryAddress(evmChainID)

		let mostLiquidPool = ADDRESS_ZERO
		let bestFee = 0
		let deepestLiquidity = 0n

		for (const fee of COMMON_FEE_TIERS) {
			try {
				const poolAddress = (await client.readContract({
					address: factoryAddress,
					abi: [
						parseAbiItem(
							"function getPool(address tokenA, address tokenB, uint24 fee) view returns (address pool)",
						),
					],
					functionName: "getPool",
					args: [token0, token1, fee],
				})) as HexString

				if (poolAddress !== ADDRESS_ZERO) {
					const liquidity = await client.readContract({
						address: poolAddress,
						abi: [parseAbiItem("function liquidity() view returns (uint128)")],
						functionName: "liquidity",
					})

					if (liquidity > deepestLiquidity) {
						deepestLiquidity = liquidity
						mostLiquidPool = poolAddress
						bestFee = fee
					}
				}
			} catch {
				console.warn(`Failed to get V3 pool for fee ${fee}:`)
				continue
			}
		}

		if (mostLiquidPool === ADDRESS_ZERO) {
			throw new Error(`No V3 pools found for tokens ${token0} and ${token1}`)
		}

		return { poolAddress: mostLiquidPool, fee: bestFee }
	}

	async createMultiHopSwapThroughPair(
		client: PublicClient,
		tokenIn: HexString,
		tokenOut: HexString,
		amountIn: bigint,
		evmChainID: string,
		recipient: HexString,
		protocol: "v2" | "v3" = "v2",
		slippagePercentage: bigint,
		dexPairAddress?: HexString,
	): Promise<{ finalAmountOut: bigint; calldata: Transaction[] }> {
		const wethAsset = this.chainConfigService.getWrappedNativeAssetWithDecimals(evmChainID).asset

		let intermediateToken: HexString

		if (dexPairAddress) {
			const [token0Result, token1Result] = await Promise.all([
				client.readContract({
					address: dexPairAddress,
					abi: [parseAbiItem("function token0() view returns (address)")],
					functionName: "token0",
				}),
				client.readContract({
					address: dexPairAddress,
					abi: [parseAbiItem("function token1() view returns (address)")],
					functionName: "token1",
				}),
			])
			intermediateToken = tokenOut.toLowerCase() === token0Result.toLowerCase() ? token1Result : token0Result
		} else {
			const { intermediateToken: foundIntermediateToken } = await this.findPair(
				client,
				tokenIn,
				tokenOut,
				evmChainID,
				protocol,
			)
			intermediateToken = foundIntermediateToken
		}

		const swapPath = this.buildSwapPath(tokenIn, tokenOut, intermediateToken, wethAsset)

		const { finalAmountOut, fees } = await this.getQuoteForPath(client, swapPath, amountIn, evmChainID, protocol)

		const amountOutMinimum = (finalAmountOut * (10000n - slippagePercentage)) / 10000n

		const calldata =
			protocol === "v2"
				? this.createV2SwapCalldataExactIn(swapPath, amountIn, amountOutMinimum, recipient, evmChainID)
				: this.createV3SwapCalldataExactIn(swapPath, amountIn, amountOutMinimum, fees, recipient, evmChainID)

		return {
			finalAmountOut: amountOutMinimum,
			calldata,
		}
	}

	private buildSwapPath(
		tokenIn: HexString,
		tokenOut: HexString,
		intermediateToken: HexString,
		wethAsset: HexString,
	): HexString[] {
		const normalize = (token: HexString) => token.toLowerCase()

		if (normalize(intermediateToken) === normalize(tokenIn)) {
			return [tokenIn, tokenOut]
		}

		if (normalize(intermediateToken) === normalize(wethAsset)) {
			return [tokenIn, wethAsset, tokenOut]
		}

		return [tokenIn, wethAsset, intermediateToken, tokenOut]
	}

	private async getQuoteForPath(
		client: PublicClient,
		path: HexString[],
		initialAmount: bigint,
		evmChainID: string,
		protocol: "v2" | "v3",
	): Promise<{ finalAmountOut: bigint; fees: number[] }> {
		let currentAmount = initialAmount
		const fees: number[] = []

		for (let i = 0; i < path.length - 1; i++) {
			const tokenIn = path[i]
			const tokenOut = path[i + 1]

			if (protocol === "v2") {
				currentAmount = await this.getV2QuoteWithAmountIn(client, tokenIn, tokenOut, currentAmount, evmChainID)
			} else {
				const quote = await this.getV3QuoteWithAmountIn(client, tokenIn, tokenOut, currentAmount, evmChainID)
				currentAmount = quote.amountOut
				fees.push(quote.fee)
			}
		}

		return { finalAmountOut: currentAmount, fees }
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
}
