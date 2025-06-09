import { OrderPlaced } from "@/configs/src/types/models/OrderPlaced"
import { OrderStatus, OrderStatusMetadata, ProtocolParticipant, RewardPointsActivityType } from "@/configs/src/types"
import PriceHelper from "@/utils/price.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"
import { hexToBytes, bytesToHex, keccak256, encodeAbiParameters, toHex, hexToString } from "viem"
import type { Hex } from "viem"
import Decimal from "decimal.js"
import { PointsService } from "./points.service"
import { UNISWAP_ADDRESSES } from "@/addresses/uniswap.addresses"
import { ethers } from "ethers"
import uniswapV2Abi from "@/configs/abis/UniswapV2.abi.json"
import uniswapV3FactoryAbi from "@/configs/abis/UniswapV3Factory.abi.json"
import uniswapV3PoolAbi from "@/configs/abis/UniswapV3Pool.abi.json"
import uniswapV3QuoterV2Abi from "@/configs/abis/UniswapV3QuoterV2.abi.json"
import uniswapV4QuoterAbi from "@/configs/abis/UniswapV4Quoter.abi.json"

export interface TokenInfo {
	token: Hex
	amount: bigint
}

export interface PaymentInfo extends TokenInfo {
	beneficiary: Hex
}

export interface Order {
	id?: string
	user: Hex
	sourceChain: string
	destChain: string
	deadline: bigint
	nonce: bigint
	fees: bigint
	outputs: PaymentInfo[]
	inputs: TokenInfo[]
	callData: Hex
}

export class IntentGatewayService {
	static async getOrCreateOrder(
		order: Order,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
	): Promise<OrderPlaced> {
		const { transactionHash, blockNumber, timestamp } = logsData

		let orderPlaced = await OrderPlaced.get(order.id!)

		const { inputUSD, inputValuesUSD } = await this.getOrderValue(order)

		if (!orderPlaced) {
			orderPlaced = await OrderPlaced.create({
				id: order.id!,
				user: order.user,
				sourceChain: order.sourceChain,
				destChain: order.destChain,
				commitment: order.id!,
				deadline: order.deadline,
				nonce: order.nonce,
				fees: order.fees,
				inputTokens: order.inputs.map((input) => input.token),
				inputAmounts: order.inputs.map((input) => input.amount),
				inputValuesUSD: inputValuesUSD,
				inputUSD: inputUSD,
				outputTokens: order.outputs.map((output) => output.token),
				outputAmounts: order.outputs.map((output) => output.amount),
				outputBeneficiaries: order.outputs.map((output) => output.beneficiary),
				calldata: order.callData,
				status: OrderStatus.PLACED,
				createdAt: timestampToDate(timestamp),
				blockNumber: BigInt(blockNumber),
				blockTimestamp: timestamp,
				transactionHash,
			})
			await orderPlaced.save()

			// Award points for order placement - using USD value directly
			const orderValue = new Decimal(inputUSD)
			const pointsToAward = orderValue.floor().toNumber()

			await PointsService.awardPoints(
				this.bytes32ToBytes20(order.user),
				ethers.utils.toUtf8String(order.sourceChain),
				BigInt(pointsToAward),
				ProtocolParticipant.USER,
				RewardPointsActivityType.ORDER_PLACED_POINTS,
				transactionHash,
				`Points awarded for placing order ${order.id} with value ${inputUSD} USD`,
				timestamp,
			)
		}

		return orderPlaced
	}

	static async getByCommitment(commitment: string): Promise<OrderPlaced | null> {
		const orderPlaced = await OrderPlaced.get(commitment)

		if (!orderPlaced) return null

		return orderPlaced
	}

	private static async getOrderValue(order: Order): Promise<{ inputUSD: string; inputValuesUSD: string[] }> {
		const inputValuesUSD = await this.getInputValuesUSD(order)

		return {
			inputUSD: inputValuesUSD.total,
			inputValuesUSD: inputValuesUSD.values,
		}
	}

	private static async getInputValuesUSD(order: Order): Promise<{ total: string; values: string[] }> {
		return this.getTokenValuesUSD(order.inputs)
	}

	private static async getTokenValuesUSD(
		tokens: { token: string; amount: bigint }[],
	): Promise<{ total: string; values: string[] }> {
		const valuesUSD = await Promise.all(
			tokens.map(async (token) => {
				const tokenAddress = this.bytes32ToBytes20(token.token)
				let decimals = 18

				if (tokenAddress != "0x0000000000000000000000000000000000000000") {
					const tokenContract = ERC6160Ext20Abi__factory.connect(tokenAddress, api)
					decimals = await tokenContract.decimals()
				}

				return this.getTokenPriceInUSD(tokenAddress, token.amount, decimals)
			}),
		)

		const total = valuesUSD.reduce((acc, curr) => {
			return acc.plus(new Decimal(curr.amountValueInUSD))
		}, new Decimal(0))

		return {
			total: total.toFixed(18),
			values: valuesUSD.map((value) => value.amountValueInUSD),
		}
	}

	private static async getTokenPriceInUSD(
		tokenAddress: string,
		amount: bigint,
		decimals: number,
	): Promise<{
		priceInUSD: string
		amountValueInUSD: string
	}> {
		try {
			const isNativeToken = tokenAddress.endsWith("0000000000000000000000000000000000000000")

			let priceInUSD: string
			if (isNativeToken) {
				const nativePrice = await PriceHelper.getNativeCurrencyPrice(chainId)
				priceInUSD = new Decimal(nativePrice.toString()).dividedBy(new Decimal(10).pow(18)).toFixed(18)
			} else {
				const uniswapPrice = await this.getUniswapPrice(tokenAddress, decimals)
				priceInUSD = uniswapPrice.toFixed(18)
			}

			const amountValueInUSD = new Decimal(amount.toString())
				.dividedBy(new Decimal(10).pow(decimals))
				.times(new Decimal(priceInUSD))
				.toFixed(18)

			return {
				priceInUSD,
				amountValueInUSD,
			}
		} catch (error) {
			logger.error(`Error getting token price for ${tokenAddress}: ${error}`)
			return {
				priceInUSD: "0",
				amountValueInUSD: "0",
			}
		}
	}

	private static async getUniswapPrice(tokenAddress: string, decimals: number): Promise<Decimal> {
		try {
			const addresses = UNISWAP_ADDRESSES[`EVM-${chainId}`]
			const { V2_FACTORY, WETH, USDC, USDT, DAI, V4_QUOTER, V3_FACTORY, V3_QUOTER } = addresses

			const amountIn = BigInt(10 ** decimals)

			const quoteTokens = [USDC, USDT, DAI, WETH]

			// Try V2 first
			for (const quoteToken of quoteTokens) {
				const pairAddress = await this.getUniswapV2PairAddress(V2_FACTORY, tokenAddress, quoteToken)
				if (pairAddress) {
					try {
						const price = await this.getPriceFromPair(pairAddress, quoteToken)
						// If we got a price from WETH pair, convert to USD
						if (quoteToken === WETH) {
							const wethUsdcPair = await this.getUniswapV2PairAddress(V2_FACTORY, WETH, USDC)
							if (wethUsdcPair) {
								const wethUsdPrice = await this.getPriceFromPair(wethUsdcPair, USDC)
								return price.times(wethUsdPrice)
							}
						}
						return price
					} catch (error) {
						logger.error(`Error getting price from V2 pair ${pairAddress}: ${error}`)
						continue
					}
				}
			}

			// If V2 fails, try V3

			if (V3_FACTORY && V3_QUOTER) {
				// Out of three tiers, we return the first one that works
				const fees = [500, 3000, 10000] // 0.05%, 0.3%, 1%

				for (const quoteToken of quoteTokens) {
					for (const fee of fees) {
						try {
							const factory = new ethers.Contract(V3_FACTORY, uniswapV3FactoryAbi, api)
							const pool = await factory.getPool(tokenAddress, quoteToken, fee)

							if (pool && pool !== "0x0000000000000000000000000000000000000000") {
								const poolContract = new ethers.Contract(pool, uniswapV3PoolAbi, api)
								const liquidity = await poolContract.liquidity()

								if (liquidity > BigInt(0)) {
									const quoter = new ethers.Contract(V3_QUOTER, uniswapV3QuoterV2Abi, api)
									const quoteResult = await quoter.quoteExactInputSingle({
										tokenIn: tokenAddress,
										tokenOut: quoteToken,
										fee: fee,
										amount: amountIn,
										sqrtPriceLimitX96: BigInt(0),
									})

									const [amountOut] = quoteResult
									let price = new Decimal(amountOut.toString()).dividedBy(new Decimal(10 ** decimals))

									// If we got a price from WETH pair, convert to USD
									if (quoteToken === WETH) {
										const wethUsdcPool = await factory.getPool(WETH, USDC, 500) // Use 0.05% fee tier for stable pairs

										if (
											wethUsdcPool &&
											wethUsdcPool !== "0x0000000000000000000000000000000000000000"
										) {
											const wethQuoteResult = await quoter.quoteExactInputSingle({
												tokenIn: WETH,
												tokenOut: USDC,
												fee: 500,
												amount: BigInt(10 ** decimals),
												sqrtPriceLimitX96: BigInt(0),
											})

											const [wethAmountOut] = wethQuoteResult
											const wethUsdPrice = new Decimal(wethAmountOut.toString()).dividedBy(
												new Decimal(10 ** decimals),
											)
											price = price.times(wethUsdPrice)
										}
									}

									// Return the first valid price we find
									return price
								}
							}
						} catch (error) {
							logger.error(`Error getting price from V3 pool for fee ${fee}: ${error}`)
							continue
						}
					}
				}
			}

			// If V3 fails, try V4
			if (V4_QUOTER) {
				for (const quoteToken of quoteTokens) {
					for (const fee of [500, 3000, 10000]) {
						// Same fee tiers as V3
						try {
							const quoter = new ethers.Contract(V4_QUOTER, uniswapV4QuoterAbi, api)

							const [token0, token1] =
								tokenAddress.toLowerCase() < quoteToken.toLowerCase()
									? [tokenAddress, quoteToken]
									: [quoteToken, tokenAddress]

							const poolKey = {
								currency0: token0,
								currency1: token1,
								fee: fee,
								tickSpacing: fee === 500 ? 10 : fee === 3000 ? 60 : 200, // Tick spacing changes based on fee
								hooks: "0x0000000000000000000000000000000000000000" as Hex,
							}

							// Determine if token0 is the input token
							const zeroForOne = tokenAddress.toLowerCase() === token0.toLowerCase()

							const quoteParams = {
								poolKey,
								zeroForOne,
								exactAmount: amountIn,
								hookData: "0x" as Hex,
							}

							const quoteResult = await quoter.callStatic.quoteExactInputSingle(quoteParams)
							const amountOut = quoteResult.amountOut

							let price = new Decimal(amountOut.toString()).dividedBy(new Decimal(10 ** decimals))

							if (quoteToken === WETH) {
								const wethUsdcPoolKey = {
									currency0: WETH,
									currency1: USDC,
									fee: 500,
									tickSpacing: 10,
									hooks: "0x0000000000000000000000000000000000000000" as Hex,
								}

								const wethQuoteParams = {
									poolKey: wethUsdcPoolKey,
									zeroForOne: true,
									exactAmount: BigInt(10 ** decimals),
									hookData: "0x" as Hex,
								}

								const wethQuoteResult = await quoter.callStatic.quoteExactInputSingle(wethQuoteParams)
								const wethAmountOut = wethQuoteResult.amountOut
								const wethUsdPrice = new Decimal(wethAmountOut.toString()).dividedBy(
									new Decimal(10 ** decimals),
								)
								price = price.times(wethUsdPrice)
							}

							return price
						} catch (error) {
							logger.error(`Error getting price from V4 pool for fee ${fee}: ${error}`)
							continue
						}
					}
				}
			}

			throw new Error(`No Uniswap V2, V3, or V4 pair found for token ${tokenAddress}`)
		} catch (error) {
			logger.error(`Error getting Uniswap price for ${tokenAddress}: ${error}`)
			return new Decimal(1)
		}
	}

	private static async getUniswapV2PairAddress(
		factoryAddress: string,
		token0: string,
		token1: string,
	): Promise<string | null> {
		try {
			const factory = new ethers.Contract(factoryAddress, uniswapV2Abi.factory, api)

			const pairAddress = await factory.callStatic.getPair(token0, token1)
			logger.info(`Pair address for ${token0}/${token1}: ${pairAddress}`)
			return pairAddress === "0x0000000000000000000000000000000000000000" ? null : pairAddress
		} catch (error) {
			logger.error(`Error getting pair address for ${token0}/${token1}: ${error}`)
			return null
		}
	}

	private static async getPriceFromPair(pairAddress: string, quoteToken: string): Promise<Decimal> {
		try {
			const pair = new ethers.Contract(pairAddress, uniswapV2Abi.pair, api)

			const [token0, token1] = await Promise.all([pair.token0(), pair.token1()])

			const [reserve0, reserve1] = await pair.getReserves()

			// Calculate price based on reserves
			const reserveIn = token0.toLowerCase() === quoteToken.toLowerCase() ? reserve0 : reserve1
			const reserveOut = token0.toLowerCase() === quoteToken.toLowerCase() ? reserve1 : reserve0

			if (reserveIn.eq(0) || reserveOut.eq(0)) {
				throw new Error("Zero reserves")
			}

			return new Decimal(reserveIn.toString()).dividedBy(new Decimal(reserveOut.toString()))
		} catch (error) {
			logger.error(`Error getting price from pair ${pairAddress}: ${error}`)
			throw error
		}
	}

	static async updateOrderStatus(
		commitment: string,
		status: OrderStatus,
		logsData: {
			transactionHash: string
			blockNumber: number
			timestamp: bigint
		},
		filler?: string,
	): Promise<void> {
		const { transactionHash, blockNumber, timestamp } = logsData

		const orderPlaced = await OrderPlaced.get(commitment)

		if (orderPlaced) {
			orderPlaced.status = status
			await orderPlaced.save()

			// Award points for order filling - using USD value directly
			if (status === OrderStatus.FILLED && filler) {
				const orderValue = new Decimal(orderPlaced.inputUSD)
				const pointsToAward = orderValue.floor().toNumber()

				await PointsService.awardPoints(
					filler,
					ethers.utils.toUtf8String(orderPlaced.destChain),
					BigInt(pointsToAward),
					ProtocolParticipant.FILLER,
					RewardPointsActivityType.ORDER_FILLED_POINTS,
					transactionHash,
					`Points awarded for filling order ${commitment} with value ${orderPlaced.inputUSD} USD`,
					timestamp,
				)
			}

			// Deduct points when order is cancelled
			if (status === OrderStatus.REFUNDED) {
				const orderValue = new Decimal(orderPlaced.inputUSD)
				const pointsToDeduct = orderValue.floor().toNumber()

				await PointsService.deductPoints(
					orderPlaced.user,
					orderPlaced.sourceChain,
					BigInt(pointsToDeduct),
					ProtocolParticipant.USER,
					RewardPointsActivityType.ORDER_PLACED_POINTS,
					transactionHash,
					`Points deducted for refunded order ${commitment} with value ${orderPlaced.inputUSD} USD`,
					timestamp,
				)
			}
		}

		const orderStatusMetadata = await OrderStatusMetadata.create({
			id: `${commitment}.${status}`,
			orderId: commitment,
			status,
			chain: chainId,
			timestamp,
			blockNumber: blockNumber.toString(),
			filler,
			transactionHash,
			createdAt: timestampToDate(timestamp),
		})

		await orderStatusMetadata.save()
	}

	static bytes32ToBytes20(bytes32: string): string {
		if (bytes32 === "0x0000000000000000000000000000000000000000000000000000000000000000") {
			return "0x0000000000000000000000000000000000000000"
		}

		const bytes = hexToBytes(bytes32 as Hex)
		const addressBytes = bytes.slice(12)
		return bytesToHex(addressBytes) as Hex
	}

	static computeOrderCommitment(order: Order): string {
		const encodedOrder = encodeAbiParameters(
			[
				{
					name: "order",
					type: "tuple",
					components: [
						{ name: "user", type: "bytes32" },
						{ name: "sourceChain", type: "bytes" },
						{ name: "destChain", type: "bytes" },
						{ name: "deadline", type: "uint256" },
						{ name: "nonce", type: "uint256" },
						{ name: "fees", type: "uint256" },
						{
							name: "outputs",
							type: "tuple[]",
							components: [
								{ name: "token", type: "bytes32" },
								{ name: "amount", type: "uint256" },
								{ name: "beneficiary", type: "bytes32" },
							],
						},
						{
							name: "inputs",
							type: "tuple[]",
							components: [
								{ name: "token", type: "bytes32" },
								{ name: "amount", type: "uint256" },
							],
						},
						{ name: "callData", type: "bytes" },
					],
				},
			],
			[
				{
					user: order.user as Hex,
					sourceChain: order.sourceChain as Hex,
					destChain: order.destChain as Hex,
					deadline: order.deadline,
					nonce: order.nonce,
					fees: order.fees,
					outputs: order.outputs.map((output) => ({
						token: output.token as Hex,
						amount: output.amount,
						beneficiary: output.beneficiary as Hex,
					})),
					inputs: order.inputs.map((input) => ({
						token: input.token as Hex,
						amount: input.amount,
					})),
					callData: order.callData as Hex,
				},
			],
		)

		return keccak256(encodedOrder)
	}
}
